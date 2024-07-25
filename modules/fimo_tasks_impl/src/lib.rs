//! Implementation of the fimo_tasks symbols.
//!
//! The main and only task of this crate is the implementation and export of a fimo module which
//! exposes the symbols declared by the [`fimo_tasks`] interface.
//!
//! # Module info
//!
//! - Name: `fimo_tasks_impl`
//! - Description: Threading subsystem of the Fimo Engine
//! - Author: Fimo
//! - License: MIT License and Apache License, Version 2.0
//!
//! ## Parameters:
//!
//! - `default_stack_size: u32` (public, dependency, `default = 512KB`): Default stack size in
//!   bytes.
//!
//! ## Imported symbols:
//!
//! None
//!
//! ## Exposed symbols:
//!
//! - [`fimo_tasks::Context`](fimo_tasks::symbols::fimo_tasks::Context)

#![feature(arbitrary_self_types)]
#![feature(exposed_provenance)]
#![feature(result_flattening)]
#![feature(strict_provenance)]
#![feature(thread_local)]

use crate::{
    module_export::TasksModule,
    worker_group::{WorkerGroupFFI, WorkerGroupImpl},
};
use crossbeam_channel::{Receiver, Sender};
use fimo_std::{
    allocator::FimoAllocator,
    context::{Context as StdContext, ContextView},
    error::Error,
    ffi::FFITransferable,
    module::{Module, PreModule},
    tracing::ThreadAccess,
};
use fimo_tasks::{bindings, WorkerGroupId};
use rustc_hash::FxHashMap;
use std::{
    ffi::CString,
    mem::ManuallyDrop,
    sync::{Arc, RwLock},
    thread::JoinHandle,
};
use worker_group::event_loop::stack_manager::StackDescriptor;

// We are currently building each module in separate dynamic library.
// If we decide to support static linking in the future this should be
// hidden behind a `#[cfg(...)]`.
#[global_allocator]
static GLOBAL: FimoAllocator = FimoAllocator;

mod context;
mod module_export;
mod worker_group;

#[derive(Debug)]
enum RuntimeMessage {
    Exit,
    ShutdownWorkerGroup(Arc<WorkerGroupImpl>),
}

#[derive(Debug)]
struct Runtime {
    shared: Arc<RuntimeShared>,
    inner_thread: Option<JoinHandle<()>>,
}

impl Runtime {
    fn new(module: PreModule<'_, TasksModule<'_>>) -> Result<Self, Error> {
        let _span = fimo_std::span_trace!(
            module.context(),
            "module constructor, module: {}",
            module.module_info()
        );
        fimo_std::emit_debug!(
            module.context(),
            "initializing module, info: {:?}",
            module.module_info()
        );
        fimo_std::emit_trace!(module.context(), "initializing module");
        fimo_std::emit_trace!(
            module.context(),
            "default_stack_size: {}",
            module.parameters().default_stack_size().read(&module)?
        );

        let (sx, inner_thread) = RuntimeInner::start(module);

        Ok(Self {
            shared: RuntimeShared::new(module, sx),
            inner_thread: Some(inner_thread),
        })
    }

    fn shutdown(&mut self, module: PreModule<'_, TasksModule<'_>>) {
        let _span = fimo_std::span_trace!(
            module.context(),
            "module destructor, module: {}",
            module.module_info()
        );
        fimo_std::emit_debug!(module.context(), "destroying module");

        fimo_std::emit_trace!(module.context(), "shutting down runtime");
        self.shared.shutdown();

        fimo_std::emit_trace!(module.context(), "joining inner thread");
        self.inner_thread
            .take()
            .expect("inner thread already joined")
            .join()
            .expect("inner thread panicked");
    }

    fn shared_runtime(&self) -> &Arc<RuntimeShared> {
        &self.shared
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        assert!(
            self.inner_thread.is_none(),
            "runtime thread should be joined"
        );
        assert!(self.shared.is_shutdown(), "runtime was not shut down");
    }
}

#[derive(Debug)]
struct RuntimeShared {
    context: StdContext,
    sx: Sender<RuntimeMessage>,
    worker_group_manager: RwLock<WorkerGroupManager>,
}

impl RuntimeShared {
    fn new(module: PreModule<'_, TasksModule<'_>>, sx: Sender<RuntimeMessage>) -> Arc<Self> {
        Arc::new(Self {
            context: module.context().to_context(),
            sx,
            worker_group_manager: RwLock::new(WorkerGroupManager::new()),
        })
    }

    fn is_shutdown(&self) -> bool {
        let _span = fimo_std::span_trace!(*self.context, "");
        fimo_std::emit_trace!(*self.context, "locking worker group manager");
        let guard = self.worker_group_manager.read().unwrap();
        guard.is_closed()
    }

    fn shutdown(&self) {
        let _span = fimo_std::span_trace!(*self.context, "");

        {
            fimo_std::emit_trace!(*self.context, "cleaning up all worker groups");
            let mut guard = self.worker_group_manager.write().unwrap();
            let groups = guard.close();
            for (_, group) in groups {
                self.send_runtime_message(RuntimeMessage::ShutdownWorkerGroup(group));
            }
        }

        fimo_std::emit_trace!(*self.context, "shutting down inner thread");
        self.send_runtime_message(RuntimeMessage::Exit);
    }

    fn query_worker_groups(&self) -> WorkerGroupQuery {
        let _span = fimo_std::span_trace!(*self.context, "");
        fimo_std::emit_trace!(*self.context, "querying worker groups");
        let guard = self
            .worker_group_manager
            .read()
            .expect("could not lock worker group manager");
        guard.query_workers()
    }

    fn worker_group_by_id(&self, group_id: WorkerGroupId) -> Option<Arc<WorkerGroupImpl>> {
        let _span = fimo_std::span_trace!(*self.context, "group_id: {group_id:?}");
        fimo_std::emit_trace!(*self.context, "searching for group");
        let guard = self
            .worker_group_manager
            .read()
            .expect("could not lock worker group manager");
        guard.find_by_id(group_id)
    }

    fn spawn_worker_group(self: &Arc<Self>) -> Result<Arc<WorkerGroupImpl>, Error> {
        let _span = fimo_std::span_trace!(*self.context, "");
        {
            fimo_std::emit_trace!(*self.context, "requesting a new worker group");
            let mut guard = self.worker_group_manager.write().unwrap();
            guard.spawn_new(
                CString::new("").unwrap(),
                false,
                1,
                512 * 1024,
                vec![],
                self,
            )
        }
    }

    fn shutdown_worker_group(&self, group_id: WorkerGroupId) {
        let _span = fimo_std::span_trace!(*self.context, "group_id: {group_id:?}");
        {
            fimo_std::emit_trace!(*self.context, "removing group from runtime");
            let mut guard = self
                .worker_group_manager
                .write()
                .expect("could not lock worker group manager");
            match guard.remove_by_id(group_id) {
                None => {
                    fimo_std::emit_trace!(*self.context, "group already removed");
                }
                Some(group) => {
                    fimo_std::emit_trace!(*self.context, "sending group to cleanup");
                    self.send_runtime_message(RuntimeMessage::ShutdownWorkerGroup(group));
                }
            }
        }
    }

    fn send_runtime_message(&self, message: RuntimeMessage) {
        fimo_std::emit_debug!(self.context, "sending message: {message:?}");
        self.sx
            .send(message)
            .expect("could not send runtime message");
    }
}

#[derive(Debug)]
struct WorkerGroupManager {
    closed: bool,
    next_id: WorkerGroupId,
    groups: FxHashMap<WorkerGroupId, Arc<WorkerGroupImpl>>,
}

impl WorkerGroupManager {
    fn new() -> Self {
        Self {
            closed: false,
            next_id: WorkerGroupId(0),
            groups: Default::default(),
        }
    }

    fn is_closed(&self) -> bool {
        self.closed
    }

    fn spawn_new(
        &mut self,
        name: CString,
        visible: bool,
        num_workers: usize,
        default_stack_size: usize,
        stacks: Vec<StackDescriptor>,
        runtime: &Arc<RuntimeShared>,
    ) -> Result<Arc<WorkerGroupImpl>, Error> {
        let ctx = *runtime.context;
        let _span = fimo_std::span_trace!(
            ctx,
            "this: {self:?}, name: {name:?}, visible: {visible:?}, num_workers: {num_workers:?}, \
            default_stack_size: {default_stack_size:?}, stacks: {stacks:?}, runtime: {runtime:?}"
        );

        if self.closed {
            fimo_std::emit_error!(ctx, "manager is already closed");
            return Err(Error::EPERM);
        }

        let id = self.next_id;
        if id.0 == usize::MAX {
            fimo_std::emit_error!(ctx, "run out of worker group ids");
            return Err(Error::E2BIG);
        }

        let group = WorkerGroupImpl::new(
            ctx,
            id,
            name,
            visible,
            num_workers,
            default_stack_size,
            stacks,
            runtime.clone(),
        );
        self.groups.insert(id, group.clone());
        Ok(group)
    }

    fn close(&mut self) -> FxHashMap<WorkerGroupId, Arc<WorkerGroupImpl>> {
        assert!(!self.closed, "manager has already been closed");
        self.closed = true;
        std::mem::take(&mut self.groups)
    }

    fn query_workers(&self) -> WorkerGroupQuery {
        let groups = self.groups.values().filter(|g| g.is_visible()).cloned();
        WorkerGroupQuery::new(groups)
    }

    fn find_by_id(&self, id: WorkerGroupId) -> Option<Arc<WorkerGroupImpl>> {
        self.groups.get(&id).cloned()
    }

    fn remove_by_id(&mut self, id: WorkerGroupId) -> Option<Arc<WorkerGroupImpl>> {
        self.groups.remove(&id)
    }
}

#[derive(Debug)]
struct WorkerGroupQuery {
    groups: Box<[bindings::FiTasksWorkerGroupQuery]>,
}

impl WorkerGroupQuery {
    fn new(groups: impl Iterator<Item = Arc<WorkerGroupImpl>>) -> Self {
        #[repr(transparent)]
        struct DropGuard(bindings::FiTasksWorkerGroupQuery);
        impl Drop for DropGuard {
            fn drop(&mut self) {
                // Safety: Assuming that the caller does not provide wrong input, it is the only
                // type that can cast to it.
                let group = unsafe { WorkerGroupFFI::from_ffi(self.0.grp) };
                drop(group);
            }
        }

        // Cast the groups to the ffi representation.
        let mut groups = groups
            .map(|group| {
                DropGuard(bindings::FiTasksWorkerGroupQuery {
                    grp: WorkerGroupFFI(group).into_ffi(),
                    next: std::ptr::null_mut(),
                })
            })
            .collect::<Box<_>>();

        // Set the `next` pointers
        for i in 0..groups.len() {
            let next = if i == groups.len() - 1 {
                std::ptr::null_mut()
            } else {
                // Safety: Is in the same allocation
                unsafe { groups.as_mut_ptr().add(i + 1) }
            };
            groups[i].0.next = next.cast();
        }

        // Now that we can not panic anymore we cast the slice.
        // Safety: `DropGuard` is a transparent wrapper.
        let groups = unsafe {
            let len = groups.len();
            let data = Box::into_raw(groups).cast();
            let raw = std::slice::from_raw_parts_mut(data, len);
            Box::from_raw(raw)
        };

        Self { groups }
    }
}

impl Drop for WorkerGroupQuery {
    fn drop(&mut self) {
        for group in self.groups.iter() {
            // Safety: Assuming that the caller does not provide wrong input, it is the only type
            // that can cast to it.
            let group = unsafe { WorkerGroupFFI::from_ffi(group.grp) };
            drop(group);
        }
    }
}

impl FFITransferable<*mut bindings::FiTasksWorkerGroupQuery> for WorkerGroupQuery {
    fn into_ffi(self) -> *mut bindings::FiTasksWorkerGroupQuery {
        let this = ManuallyDrop::new(self);

        // Safety: We don't drop self.
        unsafe {
            let groups = std::ptr::read(&this.groups);
            let raw = Box::into_raw(groups);
            raw.cast()
        }
    }

    unsafe fn from_ffi(ffi: *mut bindings::FiTasksWorkerGroupQuery) -> Self {
        // Safety: The query is an allocated box.
        unsafe {
            assert!(!ffi.is_null());

            // Count the length of the slice.
            let mut len = 0;
            let mut current = ffi;
            while !current.is_null() {
                len += 1;
                current = (*current).next;
            }

            let raw = std::ptr::slice_from_raw_parts_mut(ffi, len);
            let groups = Box::from_raw(raw);
            Self { groups }
        }
    }
}

struct RuntimeInner {
    rx: Receiver<RuntimeMessage>,
}

impl RuntimeInner {
    fn start(module: PreModule<'_, TasksModule<'_>>) -> (Sender<RuntimeMessage>, JoinHandle<()>) {
        let context = module.context().to_context();
        let (sx, rx) = crossbeam_channel::unbounded();
        let this = Self { rx };
        let thread = std::thread::spawn(move || {
            fimo_std::panic::abort_on_panic(|| {
                let _access = ThreadAccess::new(&context).expect("could not register thread");
                let _span = fimo_std::span_trace!(*context, "tasks runtime event loop");

                this.process_messages(&context);
            });
        });
        (sx, thread)
    }

    fn process_messages(self, context: &ContextView<'_>) {
        let mut exit = false;

        while !exit || !self.rx.is_empty() {
            let message = self.rx.recv().expect("could not receive message");
            match message {
                RuntimeMessage::Exit => {
                    fimo_std::emit_debug!(context, "exiting");
                    exit = true;
                }
                RuntimeMessage::ShutdownWorkerGroup(group) => {
                    fimo_std::emit_debug!(context, "shutting down worker group: {group:?}");
                    group.wait_for_close();
                }
            }
        }
    }
}
