use crate::{Context, bindings};
use fimo_std::error::{AnyError, to_result_indirect, to_result_indirect_in_place};
use std::{ffi::CStr, fmt::Formatter, marker::PhantomData, num::NonZeroUsize};

/// A unique identifier for a [`WorkerGroup`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerGroupId(pub usize);

/// A unique identifier for a thread managed by a [`WorkerGroup`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(pub usize);

/// A group of workers.
///
/// Each `WorkerGroup` owns a pool of threads and manages the scheduling and execution of tasks on
/// those threads. Tasks are specified by enqueueing [`CommandBuffer`](crate::CommandBuffer)s onto
/// the execution queue of the worker. Owning a handle to a `WorkerGroup` does not ensure the
/// ability to enqueue not commands, as a user may request the termination of the group.
#[repr(transparent)]
pub struct WorkerGroup<'ctx>(
    pub(super) bindings::FiTasksWorkerGroup,
    pub(super) PhantomData<fn() -> &'ctx ()>,
);

impl WorkerGroup<'_> {
    /// Returns the unique id of the worker group.
    pub fn id(&self) -> WorkerGroupId {
        // Safety: FFI call is safe
        let id = unsafe { self.vtable().v0.id.unwrap_unchecked()(self.data()) };
        WorkerGroupId(id)
    }

    /// Checks whether the worker group is open to receive new commands.
    pub fn is_open(&self) -> bool {
        // Safety: FFI call is safe
        unsafe { self.vtable().v0.is_open.unwrap_unchecked()(self.data()) }
    }

    /// Checks whether the current thread is a worker thread of the group.
    pub fn is_worker(&self) -> bool {
        // Safety: FFI call is safe
        unsafe { self.vtable().v0.is_worker.unwrap_unchecked()(self.data()) }
    }

    /// Returns the name of the worker.
    pub fn name(&self) -> &CStr {
        // Safety: FFI call is safe
        let name = unsafe { self.vtable().v0.name.unwrap_unchecked()(self.data()) };

        // Safety: The function is guaranteed to return a null-terminated string.
        unsafe { CStr::from_ptr(name) }
    }

    /// Requests that the worker group stops accepting new commands.
    ///
    /// If successful, the worker group will close its side of the channel and stop accepting new
    /// commands. The currently enqueued commands will be run to completion.
    pub fn request_close(&self) -> Result<(), AnyError> {
        // Safety: FFI call is safe
        unsafe {
            to_result_indirect(|err| {
                *err = self.vtable().v0.request_close.unwrap_unchecked()(self.data());
            })
        }
    }

    /// Fetches a list of worker ids available in the worker group.
    pub fn workers(&self) -> Result<Box<[WorkerId]>, AnyError> {
        let mut num_workers = 0;
        // Safety: FFI call is safe
        let workers = unsafe {
            to_result_indirect_in_place(|err, workers| {
                *err = self.vtable().v0.workers.unwrap_unchecked()(
                    self.data(),
                    workers.as_mut_ptr(),
                    &mut num_workers,
                );
            })?
        };

        // We can cast the pointer to an `WorkerId` pointer, since the two types
        // have the same layout.
        let workers = workers.cast::<WorkerId>();

        // The API guarantees that we are returned a contiguous range of memory containing the ids.
        let workers = std::ptr::slice_from_raw_parts_mut(workers, num_workers);

        // Safety: According to the API, the slice has been allocated with the fimo allocator,
        // therefore we are allowed to construct a box with the same allocator.
        unsafe { Ok(Box::from_raw(workers)) }
    }

    /// Fetches a list of stack sizes available in the worker group.
    ///
    /// When spawning new tasks, they will be assigned one stack which matches the requirements
    /// specified in the commands.
    pub fn stack_sizes(&self) -> Result<Box<[usize]>, AnyError> {
        let mut num_stacks = 0;
        // Safety: FFI call is safe
        let sizes = unsafe {
            to_result_indirect_in_place(|err, sizes| {
                *err = self.vtable().v0.stack_sizes.unwrap_unchecked()(
                    self.data(),
                    sizes.as_mut_ptr(),
                    &mut num_stacks,
                );
            })?
        };

        // The API guarantees that we are returned a contiguous range of memory containing the
        // sizes.
        let sizes = std::ptr::slice_from_raw_parts_mut(sizes, num_stacks);

        // Safety: According to the API, the slice has been allocated with the fimo allocator,
        // therefore we are allowed to construct a box with the same allocator.
        unsafe { Ok(Box::from_raw(sizes)) }
    }

    #[inline(always)]
    pub(super) fn data(&self) -> *mut std::ffi::c_void {
        self.0.data
    }

    #[inline(always)]
    pub(super) fn vtable(&self) -> &bindings::FiTasksWorkerGroupVTable {
        // Safety: The VTable is always initialized
        unsafe { &*self.0.vtable }
    }
}

// Safety: Sound by invariant
unsafe impl Send for WorkerGroup<'_> {}

// Safety: Sound by invariant
unsafe impl Sync for WorkerGroup<'_> {}

impl std::fmt::Debug for WorkerGroup<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerGroup")
            .field("id", &self.id())
            .field("name", &self.name())
            .finish_non_exhaustive()
    }
}

impl Clone for WorkerGroup<'_> {
    fn clone(&self) -> Self {
        // Safety: We own the reference therefore we can acquire another one.
        unsafe { self.vtable().v0.acquire.unwrap_unchecked()(self.data()) }
        Self(self.0, PhantomData)
    }
}

impl Drop for WorkerGroup<'_> {
    fn drop(&mut self) {
        // Safety: We own the reference therefore we can release it.
        unsafe { self.vtable().v0.release.unwrap_unchecked()(self.data()) }
    }
}

/// Result of a [`WorkerGroup`] query.
pub struct WorkerGroupQuery<'ctx> {
    pub(super) query: *mut bindings::FiTasksWorkerGroupQuery,
    pub(super) ctx: &'ctx Context,
}

impl<'ctx> WorkerGroupQuery<'ctx> {
    pub fn iter(&self) -> WorkerGroupIter<'_, 'ctx> {
        WorkerGroupIter {
            current: self.query,
            _phantom: PhantomData,
        }
    }
}

// Safety: Sound by invariant
unsafe impl Send for WorkerGroupQuery<'_> {}

// Safety: Sound by invariant
unsafe impl Sync for WorkerGroupQuery<'_> {}

impl<'a, 'ctx> IntoIterator for &'a WorkerGroupQuery<'ctx> {
    type Item = &'a WorkerGroup<'ctx>;
    type IntoIter = WorkerGroupIter<'a, 'ctx>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl std::fmt::Debug for WorkerGroupQuery<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Drop for WorkerGroupQuery<'_> {
    fn drop(&mut self) {
        // Safety: FFI call is safe
        unsafe {
            if to_result_indirect(|err| {
                *err = self
                    .ctx
                    .vtable()
                    .v0
                    .release_worker_group_query
                    .unwrap_unchecked()(self.ctx.data(), self.query);
            })
            .is_err()
            {
                std::process::abort()
            }
        };
    }
}

/// Iterator of a [`WorkerGroupQuery`].
#[derive(Debug)]
pub struct WorkerGroupIter<'a, 'ctx> {
    current: *mut bindings::FiTasksWorkerGroupQuery,
    _phantom: PhantomData<&'a [WorkerGroup<'ctx>]>,
}

impl<'a, 'ctx> Iterator for WorkerGroupIter<'a, 'ctx> {
    type Item = &'a WorkerGroup<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        if current.is_null() {
            return None;
        }

        // Safety: The API ensures that the pointer is either `null` or valid.
        unsafe {
            let current = &*current;
            let group = &current.grp;
            self.current = current.next;

            // Safety: `WorkerGroup` has a `transparent` layout.
            let group =
                std::mem::transmute::<&'_ bindings::FiTasksWorkerGroup, &'_ WorkerGroup<'_>>(group);
            Some(group)
        }
    }
}

// Safety: Sound by invariant
unsafe impl Send for WorkerGroupIter<'_, '_> {}

// Safety: Sound by invariant
unsafe impl Sync for WorkerGroupIter<'_, '_> {}

/// A builder for a [`WorkerGroup`].
#[derive(Debug)]
pub struct WorkerGroupBuilder<'a> {
    name: &'a CStr,
    stacks: &'a [WorkerGroupStackDescriptor],
    default_stack: usize,
    worker_count: Option<NonZeroUsize>,
    is_queryable: bool,
}

impl<'a> WorkerGroupBuilder<'a> {
    /// Constructs a new `WorkerGroupBuilder`.
    pub fn new(
        name: &'a CStr,
        stacks: &'a [WorkerGroupStackDescriptor],
        default_stack: Option<usize>,
    ) -> Self {
        if stacks.is_empty() {
            panic!("a WorkerGroupBuilder requires at least one stack descriptor")
        }

        let default_stack = default_stack.unwrap_or(0);
        if default_stack >= stacks.len() {
            panic!("the index of the default stack descriptor is out of bounds")
        }

        Self {
            name,
            stacks,
            default_stack,
            worker_count: None,
            is_queryable: false,
        }
    }

    /// Sets the number of worker threads to start in the new [`WorkerGroup`].
    ///
    /// A value of `None` starts one worker for each logical core present in the system.
    ///
    /// Defaults to `None`.
    pub fn with_worker_count(mut self, count: Option<NonZeroUsize>) -> Self {
        self.worker_count = count;
        self
    }

    /// Sets whether to make the new [`WorkerGroup`] queryable through the [`Context`].
    ///
    /// Specifying this does not stop others to acquire a reference to the worker group through its
    /// [`WorkerGroupId`].
    pub fn with_queryable(mut self, queryable: bool) -> Self {
        self.is_queryable = queryable;
        self
    }

    /// Creates a new [`WorkerGroup`].
    pub fn build(self, ctx: &Context) -> Result<WorkerGroup<'_>, AnyError> {
        // Safety: `WorkerGroupStackDescriptor` has a `transparent` layout.
        let stacks = unsafe {
            std::mem::transmute::<
                &'_ [WorkerGroupStackDescriptor],
                &'_ [bindings::FiTasksWorkerGroupConfigStack],
            >(self.stacks)
        };

        let config = bindings::FiTasksWorkerGroupConfig {
            next: std::ptr::null_mut(),
            name: self.name.as_ptr(),
            stacks: stacks.as_ptr(),
            num_stacks: self.stacks.len(),
            default_stack_index: self.default_stack,
            number_of_workers: self.worker_count.map_or(0, |x| x.get()),
            is_queryable: self.is_queryable,
        };

        // Safety: FFI call is safe
        let group = unsafe {
            to_result_indirect_in_place(|err, group| {
                *err = ctx.vtable().v0.create_worker_group.unwrap_unchecked()(
                    ctx.data(),
                    config,
                    group.as_mut_ptr(),
                );
            })?
        };

        Ok(WorkerGroup(group, PhantomData))
    }
}

/// Descriptor for a stack of a [`WorkerGroup`].
#[repr(transparent)]
pub struct WorkerGroupStackDescriptor {
    config: bindings::FiTasksWorkerGroupConfigStack,
}

impl WorkerGroupStackDescriptor {
    /// Constructs a new `WorkerGroupStackDescriptor`.
    pub fn new() -> Self {
        Self {
            config: bindings::FiTasksWorkerGroupConfigStack {
                next: std::ptr::null_mut(),
                size: 0,
                starting_residency: 0,
                residency_target: 0,
                max_residency: 0,
                enable_stack_overflow_protection: true,
            },
        }
    }

    /// Sets the minimum size of the stack.
    ///
    /// The size may be rounded up to a multiple of the page size, and may also include an
    /// additional guard page. A value of `None` indicates to use the default stack size for the
    /// system.
    ///
    /// Defaults to `None`.
    pub fn with_size(&mut self, min_size: Option<NonZeroUsize>) -> &mut Self {
        let size = min_size.map_or(0, |x| x.get());
        self.config.size = size;
        self
    }

    /// Sets the number of stacks to preallocate.
    ///
    /// Defaults to `0`.
    pub fn with_starting_residency(&mut self, residency: usize) -> &mut Self {
        self.config.starting_residency = residency;
        self
    }

    /// Sets the number of resident stacks to target at any point in time.
    ///
    /// If the number of resident stacks is lower than the indicated target, the worker group may
    /// cache unassigned stacks. A value of `None` indicates that no residency number should be
    /// targeted.
    ///
    /// Defaults to `None`.
    pub fn with_residency_target(&mut self, residency: Option<NonZeroUsize>) -> &mut Self {
        let residency = residency.map_or(0, |x| x.get());
        self.config.residency_target = residency;
        self
    }

    /// Sets the maximum number of stacks that may be allocated at any point in time.
    ///
    /// If more stacks are required, the tasks will be put on hold, until they can be acquired.
    /// This, may lead to a deadlock under some circumstances. A value of `None` indicates no upper
    /// limit to the number of resident stacks.
    ///
    /// Defaults to `None`.
    pub fn with_max_residency(&mut self, residency: Option<NonZeroUsize>) -> &mut Self {
        let residency = residency.map_or(0, |x| x.get());
        self.config.max_residency = residency;
        self
    }

    /// Sets whether to enable the overflow protection for the stack.
    ///
    /// Enabling the overflow protection may require the allocation of an additional guard page for
    /// each allocated stack. Enabling this option may marginally increase the allocation time for
    /// each stack, but is advised, as a task may otherwise overwrite foreign memory.
    ///
    /// Defaults to `true`.
    ///
    /// # Safety
    ///
    /// Disabling the overflow protection may result in undefined behavior in case a stack overflow
    /// occurs.
    pub unsafe fn with_stack_overflow_protection(&mut self, enabled: bool) -> &mut Self {
        self.config.enable_stack_overflow_protection = enabled;
        self
    }
}

impl Default for WorkerGroupStackDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WorkerGroupStackDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerGroupStackDescriptor")
            .field("size", &self.config.size)
            .field("starting_residency", &self.config.starting_residency)
            .field("residency_target", &self.config.residency_target)
            .field("max_residency", &self.config.max_residency)
            .field(
                "stack_overflow_protection",
                &self.config.enable_stack_overflow_protection,
            )
            .finish_non_exhaustive()
    }
}
