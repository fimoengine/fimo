//! Fimo memory allocator.

use alloc::alloc::handle_alloc_error;
use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};

use crate::{bindings, error::to_result_indirect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FimoAllocator;

impl FimoAllocator {
    /// Default alignment of the allocator, when no value is specified.
    #[cfg(windows)]
    #[allow(unused)]
    pub(crate) const DEFAULT_ALIGNMENT: usize = 16;

    /// Default alignment of the allocator, when no value is specified.
    #[cfg(any(unix, target_family = "wasm"))]
    #[allow(unused)]
    pub(crate) const DEFAULT_ALIGNMENT: usize = core::mem::align_of::<libc::max_align_t>();
}

// Safety: We follow the specified contract
unsafe impl GlobalAlloc for FimoAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Safety: error is a valid pointer.
        match unsafe {
            to_result_indirect(|error| bindings::fimo_aligned_alloc(align, size, error))
        } {
            Ok(ptr) => {
                debug_assert!(
                    !ptr.is_null(),
                    "the allocation is null but no error was emitted"
                );
                ptr.cast()
            }
            Err(_) => handle_alloc_error(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        let align = layout.align();

        // Safety: By the contract of dealloc this is sound.
        unsafe { bindings::fimo_free_aligned_sized(ptr.cast(), align, size) }
    }
}

// Safety:
unsafe impl Allocator for FimoAllocator {
    fn allocate(&self, layout: Layout) -> Result<core::ptr::NonNull<[u8]>, AllocError> {
        let size = layout.size();
        let align = layout.align();

        // A size of zero ignores the alignment and returns `null_mut`.
        if size == 0 {
            return Err(AllocError);
        }

        // Safety: error is a valid pointer.
        match unsafe {
            to_result_indirect(|error| bindings::fimo_aligned_alloc_sized(align, size, error))
        } {
            Ok(buffer) => {
                debug_assert!(
                    !buffer.ptr.is_null(),
                    "the allocation is null but no error was emitted"
                );
                debug_assert!(
                    !buffer.buff_size > 0,
                    "the allocation is null but no error was emitted"
                );

                // Safety: We know that the allocation function returns an aligned pointer with a
                // length of `buff_size` bytes.
                unsafe {
                    Ok(core::ptr::NonNull::new_unchecked(
                        core::ptr::slice_from_raw_parts_mut(buffer.ptr.cast(), buffer.buff_size),
                    ))
                }
            }
            Err(_) => handle_alloc_error(layout),
        }
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, _layout: Layout) {
        // Safety: According to the contract of `deallocate` the pointer must have been allocated by
        // `FimoAllocator`. Therefore we are allowed to free it,
        unsafe { bindings::fimo_free(ptr.cast().as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use core::hint::black_box;

    use crate::allocator::FimoAllocator;

    #[test]
    fn allocator() {
        let x = black_box(Box::new_in(55, FimoAllocator));
        assert_eq!(*x, 55);

        let mut x = Vec::new_in(FimoAllocator);
        x.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(*x, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }
}
