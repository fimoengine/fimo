//! Implementation of the fimo_tasks symbols.
//!
//! The main and only task of this crate is the implementation and export of a fimo module which
//! exposes the symbols declared by the [`fimo_tasks`] interface.
//!
//! # Module info
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

use crate::module_export::TasksModule;
use crossbeam_channel::{Receiver, Sender};
use fimo_std::{
    allocator::FimoAllocator,
    error::Error,
    module::{Module, PreModule},
    tracing::ThreadAccess,
};
use std::thread::JoinHandle;

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
    Shutdown,
}

#[derive(Debug)]
struct Runtime {
    sx: Sender<RuntimeMessage>,
    inner_thread: Option<JoinHandle<()>>,
}

impl Runtime {
    fn new(module: PreModule<'_, TasksModule<'_>>) -> Result<Self, Error> {
        let _span = fimo_std::span_trace!(
            module.context(),
            "module constructor, module: {}",
            module.module_info()
        );
        fimo_std::emit_info!(
            module.context(),
            "initializing module, info: {:?}",
            module.module_info()
        );
        fimo_std::emit_info!(module.context(), "initializing module");
        fimo_std::emit_info!(
            module.context(),
            "default_stack_size: {}",
            module.parameters().default_stack_size().read(&module)?
        );

        let (sx, inner_thread) = RuntimeInner::start(module);

        Ok(Self {
            sx,
            inner_thread: Some(inner_thread),
        })
    }

    fn shutdown(&mut self, module: PreModule<'_, TasksModule<'_>>) {
        let _span = fimo_std::span_trace!(
            module.context(),
            "module destructor, module: {}",
            module.module_info()
        );
        fimo_std::emit_info!(module.context(), "destroying module");

        fimo_std::emit_info!(module.context(), "shutting down inner thread");
        self.sx
            .send(RuntimeMessage::Shutdown)
            .expect("could not send runtime message");
        self.inner_thread
            .take()
            .expect("inner thread already joined")
            .join()
            .expect("inner thread panicked");
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        assert!(
            self.inner_thread.is_none(),
            "runtime thread should be joined"
        );
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

                this.process_messages();
            });
        });
        (sx, thread)
    }

    fn process_messages(self) {
        let mut exit = false;

        while !exit {
            let message = self.rx.recv().expect("could not receive message");
            match message {
                RuntimeMessage::Shutdown => exit = true,
            }
        }
    }
}
