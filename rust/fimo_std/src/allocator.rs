//! Fimo memory allocator.

use crate::error::AnyResult;
use std::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout, handle_alloc_error},
    mem::MaybeUninit,
    ptr::NonNull,
};

unsafe extern "C" {
    #[allow(clashing_extern_declarations)]
    fn fimo_aligned_alloc(
        alignment: usize,
        size: usize,
        error: Option<&mut MaybeUninit<AnyResult>>,
    ) -> Option<NonNull<u8>>;

    #[allow(clashing_extern_declarations)]
    fn fimo_aligned_alloc_sized(
        alignment: usize,
        size: usize,
        error: Option<&mut MaybeUninit<AnyResult>>,
    ) -> AllocBuffer<u8>;

    #[allow(clashing_extern_declarations)]
    fn fimo_free_aligned_sized(ptr: Option<NonNull<u8>>, alignment: usize, size: usize);
}

#[repr(C)]
struct AllocBuffer<T> {
    ptr: Option<NonNull<T>>,
    size: usize,
}

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

unsafe impl GlobalAlloc for FimoAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        let ptr = unsafe { fimo_aligned_alloc(align, size, None) };
        match ptr {
            None => handle_alloc_error(layout),
            Some(ptr) => ptr.as_ptr(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        let align = layout.align();
        unsafe { fimo_free_aligned_sized(NonNull::new(ptr), align, size) }
    }
}

unsafe impl Allocator for FimoAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let size = layout.size();
        let align = layout.align();

        // A size of zero ignores the alignment and returns `null_mut`.
        if size == 0 {
            return Err(AllocError);
        }

        let buffer = unsafe { fimo_aligned_alloc_sized(align, size, None) };
        match buffer.ptr {
            None => {
                debug_assert!(buffer.size == 0);
                handle_alloc_error(layout);
            }
            Some(ptr) => {
                debug_assert!(buffer.size != 0);
                unsafe {
                    Ok(NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                        ptr.as_ptr(),
                        buffer.size,
                    )))
                }
            }
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let size = layout.size();
        let align = layout.align();
        unsafe { fimo_free_aligned_sized(Some(ptr), align, size) }
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
