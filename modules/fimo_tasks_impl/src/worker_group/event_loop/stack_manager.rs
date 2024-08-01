use crate::worker_group::{
    command_buffer::CommandBufferHandleImpl,
    task::{AcquiredStack, StackMemory},
};
use fimo_std::error::Error;
use fimo_tasks::TaskId;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct StackDescriptor {
    pub min_size: usize,
    pub preallocated: usize,
    pub target_allocated: usize,
    pub max_allocated: usize,
    pub overflow_protection: bool,
}

#[derive(Debug)]
pub struct StackManager {
    default_stack_size: usize,
    allocators: Vec<StackAllocator>,
}

impl StackManager {
    pub fn new(default_stack_size: usize, mut stacks: Vec<StackDescriptor>) -> Self {
        // Sort by ascending stack size.
        stacks.sort_by_key(|s| s.min_size);

        let allocators = stacks
            .into_iter()
            .enumerate()
            .map(|(id, stack)| {
                let StackDescriptor {
                    min_size,
                    preallocated,
                    target_allocated,
                    max_allocated,
                    overflow_protection,
                } = stack;

                StackAllocator::new(
                    id,
                    min_size,
                    preallocated,
                    target_allocated,
                    max_allocated,
                    overflow_protection,
                )
            })
            .collect();

        Self {
            default_stack_size,
            allocators,
        }
    }

    pub fn default_stack_size(&self) -> usize {
        self.default_stack_size
    }

    pub fn has_allocator(&self, size: usize) -> bool {
        self.allocator_by_size(size).is_some()
    }

    pub fn allocator_by_size(&self, size: usize) -> Option<&StackAllocator> {
        let idx = match self
            .allocators
            .binary_search_by_key(&size, |alloc| alloc.size)
        {
            Ok(idx) | Err(idx) => idx,
        };
        self.allocators.get(idx)
    }

    pub fn allocator_by_size_mut(&mut self, size: usize) -> Option<&mut StackAllocator> {
        let idx = match self
            .allocators
            .binary_search_by_key(&size, |alloc| alloc.size)
        {
            Ok(idx) | Err(idx) => idx,
        };
        self.allocators.get_mut(idx)
    }

    pub fn allocator_by_id_mut(&mut self, id: usize) -> Option<&mut StackAllocator> {
        self.allocators.get_mut(id)
    }
}

#[derive(Debug)]
pub struct StackAllocator {
    id: usize,
    size: usize,
    protected: bool,
    num_acquired: usize,
    max_num_allocated: usize,
    deallocation_threshold: usize,
    free_list: Vec<StackMemory>,
    waiting_tasks: VecDeque<(Arc<CommandBufferHandleImpl>, TaskId)>,
}

impl StackAllocator {
    fn new(
        id: usize,
        size: usize,
        preallocated: usize,
        target_allocated: usize,
        max_allocated: usize,
        overflow_protection: bool,
    ) -> Self {
        let mut this = Self {
            id,
            size,
            protected: overflow_protection,
            num_acquired: 0,
            max_num_allocated: max_allocated,
            deallocation_threshold: target_allocated,
            free_list: vec![],
            waiting_tasks: Default::default(),
        };

        // Preallocate stacks.
        for _ in 0..preallocated {
            let stack = this.acquire_stack().expect("could not preallocate stack");
            this.release_stack(stack);
        }

        this
    }

    pub fn acquire_stack(&mut self) -> Result<AcquiredStack, Error> {
        if self.num_acquired == self.max_num_allocated {
            return Err(Error::EBUSY);
        }

        if let Some(memory) = self.free_list.pop() {
            self.num_acquired += 1;
            return Ok(AcquiredStack::new(self.id, memory));
        }

        let stack = if self.protected {
            let stack = context::stack::ProtectedFixedSizeStack::new(self.size)
                .map_err(|_e| Error::EUNKNOWN)?;
            StackMemory::Protected(stack)
        } else {
            let stack =
                context::stack::FixedSizeStack::new(self.size).map_err(|_e| Error::EUNKNOWN)?;
            StackMemory::Unprotected(stack)
        };

        self.num_acquired += 1;
        Ok(AcquiredStack::new(self.id, stack))
    }

    pub fn release_stack(&mut self, stack: AcquiredStack) {
        let (id, memory) = stack.into_raw_parts();
        debug_assert!(id == self.id);

        let num_allocated = self.num_acquired + self.free_list.len();
        if num_allocated <= self.deallocation_threshold {
            self.free_list.push(memory);
        } else {
            drop(memory);
        }

        self.num_acquired -= 1;
    }

    pub fn register_waiter(&mut self, command_buffer: Arc<CommandBufferHandleImpl>, task: TaskId) {
        assert!(
            command_buffer.completion_status().is_none(),
            "command buffer already completed"
        );
        self.waiting_tasks.push_back((command_buffer, task));
    }

    pub fn pop_waiter(&mut self) -> Option<(Arc<CommandBufferHandleImpl>, TaskId, AcquiredStack)> {
        while let Some((handle, _)) = self.waiting_tasks.front() {
            // Remove aborted waiters.
            if handle.completion_status().is_some() {
                self.waiting_tasks.pop_front();
                continue;
            }

            return match self.acquire_stack() {
                Ok(stack) => {
                    let (handle, task_id) = self.waiting_tasks.pop_front().unwrap();
                    Some((handle, task_id, stack))
                }
                Err(_) => None,
            };
        }

        None
    }
}
