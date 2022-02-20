use context::stack::{ProtectedFixedSizeStack, Stack};
use fimo_module::{Error, ErrorKind};
use log::{debug, error, trace};
use std::collections::{BTreeMap, VecDeque};
use std::ops::RangeFrom;

#[derive(Debug)]
pub(crate) struct StackAllocator {
    stack_size: usize,
    allocated_stacks: usize,
    free_slots: VecDeque<TaskSlot>,
    slot_iterator: RangeFrom<usize>,
    preferred_num_allocations: usize,
    slots: BTreeMap<TaskSlot, Option<StackWrapper>>,
}

#[derive(Debug)]
struct StackWrapper(ProtectedFixedSizeStack);

// The stack is basically a Box<u8>
unsafe impl Send for StackWrapper where Box<u8>: Send {}
unsafe impl Sync for StackWrapper where Box<u8>: Sync {}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct TaskSlot(usize);

impl StackAllocator {
    pub fn new(
        stack_size: usize,
        pre_allocated: usize,
        preferred_num_allocations: usize,
    ) -> Result<Self, Error> {
        trace!("Initializing the stack allocator");

        // bound the number of pre-allocated stacks to the number of preferred allocations.
        let pre_allocated = pre_allocated.min(preferred_num_allocations);

        debug!("Stack size: {}", stack_size);
        debug!("Number of pre-allocated stacks: {}", pre_allocated);
        debug!(
            "Preferred number or allocated stacks: {}",
            preferred_num_allocations
        );

        let free_slots: VecDeque<_> = (0..pre_allocated).map(TaskSlot).collect();

        debug!("Reserved task slots: {:?}", &free_slots);

        let mut slots = BTreeMap::new();
        for s in &free_slots {
            let stack = ProtectedFixedSizeStack::new(stack_size)
                .map_err(|e| Error::new(ErrorKind::ResourceExhausted, e))?;
            slots.insert(*s, Some(StackWrapper(stack)));
        }

        trace!(
            "Allocated {} stacks of size {} bytes",
            pre_allocated,
            stack_size
        );

        Ok(Self {
            stack_size,
            allocated_stacks: pre_allocated,
            free_slots,
            slot_iterator: pre_allocated..,
            preferred_num_allocations,
            slots,
        })
    }

    pub fn allocate(&mut self) -> Result<(TaskSlot, Stack), Error> {
        trace!("Allocating a new stack and slot");

        let slot = {
            trace!("Fetching a free task slot");

            // reuse an existing slot or generate a new one.
            if let Some(slot) = self.free_slots.pop_front() {
                trace!("Found free slot");
                slot
            } else if let Some(slot) = self.slot_iterator.next() {
                trace!("Allocating new slot");
                TaskSlot(slot)
            } else {
                error!(
                    "Can not allocate a new task slot. Maximum number of concurrent tasks reached."
                );
                let error = "Maximum number of concurrent tasks reached.";
                return Err(Error::new(ErrorKind::ResourceExhausted, error));
            }
        };
        debug!("Task slot: {:?}", slot);

        trace!("Fetching stack");
        // get or init the slot to `None`.
        let entry = self.slots.entry(slot);
        let stack = entry.or_insert(None);

        // allocate new stack if it isn't already.
        if stack.is_none() {
            trace!("Stack not allocated; allocating");
            let alloc = ProtectedFixedSizeStack::new(self.stack_size)
                .map_err(|e| Error::new(ErrorKind::ResourceExhausted, e))?;
            *stack = Some(StackWrapper(alloc));
            self.allocated_stacks += 1;
            trace!("Stack allocated");
            debug!("Allocated stacks {}", self.allocated_stacks);
        }

        let stack = stack.as_ref().expect("Shouldn't happen");
        let (top, bottom) = (stack.0.top(), stack.0.bottom());

        // safety: we know that the addresses are valid, because they
        // originate from an allocated stack, that won't be deallocated
        // until we free the slot.
        let stack = unsafe { Stack::new(top, bottom) };
        debug!("Stack: {:?}", stack);

        Ok((slot, stack))
    }

    pub fn deallocate(&mut self, slot: TaskSlot) -> Result<(), Error> {
        trace!("Deallocating task slot {:?}", slot);

        if let Some(stack) = self.slots.get_mut(&slot) {
            trace!("Found stack - deallocating");
            if stack.is_none() {
                error!("Stack already deallocated");
                let error = format!("Task slot already deallocated: {:?}", slot);
                return Err(Error::new(ErrorKind::InvalidArgument, error));
            }

            // deallocate the stack if there are more than the preferred number of allocations.
            if self.allocated_stacks > self.preferred_num_allocations {
                trace!(
                    "Number of allocated stacks {} exceeds the number of preferred allocations {}",
                    self.allocated_stacks,
                    self.preferred_num_allocations
                );
                *stack = None;
                self.allocated_stacks -= 1;
                debug!("Number of allocated stacks: {}", self.allocated_stacks);

                // push the slot to the back to prioritize
                // already allocated stacks
                trace!("Push slot {:?} to the back", slot);
                self.free_slots.push_back(slot);
            } else {
                // the stack remains allocated, so prioritize its reuse
                // by pushing the handle to the front of the list.
                trace!("Push slot {:?} to the front", slot);
                self.free_slots.push_front(slot);
            }

            Ok(())
        } else {
            error!("Stack not found");
            let error = format!("Invalid task slot: {:?}", slot);
            Err(Error::new(ErrorKind::InvalidArgument, error))
        }
    }
}
