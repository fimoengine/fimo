//! Implementation of an [`ArrayList`].

use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::NonNull,
    slice::SliceIndex,
};

use alloc::alloc::AllocError;
#[doc(hidden)]
pub use alloc::boxed::Box;

use crate::{
    allocator::FimoAllocator,
    bindings,
    error::{self, to_result_indirect, to_result_indirect_in_place, Error},
};

#[doc(hidden)]
#[macro_export]
macro_rules! __force_expr {
    ($e:expr) => {
        $e
    };
}

#[macro_export]
macro_rules! array_list {
    () => {
        $crate::__force_expr!(
            core::result::Result::<_, $crate::error::Error>::Ok(
                $crate::array_list::ArrayList::new()
            )
        )
    };
    ($elem:expr; $n:expr) => {
        $crate::__force_expr!($crate::array_list::from_elem($elem, $n))
    };
    ($($x:expr),+ $(,)?) => {
        $crate::__force_expr!($crate::array_list::from_box(
            $crate::array_list::Box::try_new_in(
                [$($x),+],
                $crate::allocator::FimoAllocator
            )
        ))
    };
}

/// A contiguous dynamically growing list of elements.
///
/// The `ArrayList` may only allocate up to [`isize::MAX`]
/// elements.
#[repr(transparent)]
pub struct ArrayList<T> {
    inner: bindings::FimoArrayList,
    _phantom: PhantomData<T>,
}

impl<T> ArrayList<T> {
    const T_SIZE: usize = core::mem::size_of::<T>();
    const T_ALIGN: usize = core::mem::align_of::<T>();
    const T_MOVE_FUNC: bindings::FimoArrayListMoveFunc = None;
    const T_DROP_FUNC: bindings::FimoArrayListDropFunc = Self::drop_func();

    /// Constructs a new empty array.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let array = ArrayList::<u32>::new();
    /// assert!(array.is_empty());
    /// assert_eq!(array.capacity(), 0);
    /// ```
    pub fn new() -> Self {
        // Safety: The function is safe.
        let inner = unsafe { bindings::fimo_array_list_new() };
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Constructs a new empty array.
    ///
    /// The capacity of the array is set to at least `capacity`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let array = ArrayList::<u32>::with_capacity(10).unwrap();
    /// assert!(array.is_empty());
    /// assert!(array.capacity() >= 10);
    /// ```
    pub fn with_capacity(capacity: usize) -> Result<Self, Error> {
        // Safety: All pointers are valid.
        let inner = unsafe {
            to_result_indirect_in_place(|error, inner| {
                *error = bindings::fimo_array_list_with_capacity(
                    capacity,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    inner.as_mut_ptr(),
                );
            })?
        };
        Ok(Self {
            inner,
            _phantom: PhantomData,
        })
    }

    /// Constructs a new empty array.
    ///
    /// The capacity of the array is set to `capacity`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let array = ArrayList::<u32>::with_capacity_exact(10).unwrap();
    /// assert!(array.is_empty());
    /// assert_eq!(array.capacity(), 10);
    /// ```
    pub fn with_capacity_exact(capacity: usize) -> Result<Self, Error> {
        // Safety: All pointers are valid.
        let inner = unsafe {
            to_result_indirect_in_place(|error, inner| {
                *error = bindings::fimo_array_list_with_capacity_exact(
                    capacity,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    inner.as_mut_ptr(),
                );
            })?
        };
        Ok(Self {
            inner,
            _phantom: PhantomData,
        })
    }
}

impl<T> ArrayList<T> {
    /// Returns the capacity of the array.
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    /// Reserve capacity for at least `additional` more elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let mut array = ArrayList::<u32>::new();
    /// array.reserve(10).unwrap();
    /// assert!(array.capacity() >= 10);
    /// assert_eq!(array.len(), 0);
    /// ```
    pub fn reserve(&mut self, additional: usize) -> error::Result {
        // Safety: All pointers are valid.
        unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_reserve(
                    &mut self.inner,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    additional,
                    Self::T_MOVE_FUNC,
                );
            })
        }
    }

    /// Reserve capacity for exactly `additional` more elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let mut array = ArrayList::<u32>::new();
    /// array.reserve_exact(10).unwrap();
    /// assert_eq!(array.capacity(), 10);
    /// assert_eq!(array.len(), 0);
    /// ```
    pub fn reserve_exact(&mut self, additional: usize) -> error::Result {
        // Safety: All pointers are valid.
        unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_reserve_exact(
                    &mut self.inner,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    additional,
                    Self::T_MOVE_FUNC,
                );
            })
        }
    }

    /// Shrinks the capacity of the array as much as possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let mut array = ArrayList::<u32>::with_capacity(10).unwrap();
    /// array.push(1);
    /// array.push(2);
    /// array.push(3);
    /// assert!(array.capacity() >= 10);
    /// array.shrink_to_fit();
    /// assert!(array.capacity() == 3);
    /// ```
    pub fn shrink_to_fit(&mut self) -> error::Result {
        if self.len() < self.capacity() {
            self.set_capacity_exact(self.len())
        } else {
            Ok(())
        }
    }

    /// Shrinks the capacity of the array with a lower bound.
    ///
    /// The capacity will remain as large as both the length and the supplied value.
    /// If the current capacity is less than the lower limit, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let mut array = ArrayList::<u32>::with_capacity(10).unwrap();
    /// array.push(1);
    /// array.push(2);
    /// array.push(3);
    /// assert!(array.capacity() >= 10);
    /// array.shrink_to(4);
    /// assert!(array.capacity() >= 4);
    /// array.shrink_to(0);
    /// assert!(array.capacity() >= 0);
    /// ```
    pub fn shrink_to(&mut self, min_capacity: usize) -> error::Result {
        if self.capacity() < min_capacity {
            Ok(())
        } else {
            let min_capacity = min_capacity.min(self.len());
            self.set_capacity(min_capacity)
        }
    }

    /// Converts the array into a [`Box<[T]>`](Box).
    ///
    /// If the array has excess capacity, its items will be moved into
    /// a newly-allocated buffer with exactly the right capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let array = array_list![1, 2, 3].unwrap();
    ///
    /// let slice = array.into_boxed_slice().unwrap();
    /// assert_eq!(slice.len(), 3);
    /// ```
    pub fn into_boxed_slice(mut self) -> Result<Box<[T], FimoAllocator>, IntoBoxedSliceErr<T>> {
        if self.is_empty() {
            return Ok(Box::new_in([], FimoAllocator));
        }

        match self.shrink_to_fit() {
            Ok(_) => {}
            Err(e) => {
                return Err(IntoBoxedSliceErr {
                    error: e,
                    array: self,
                })
            }
        }

        let mut this = ManuallyDrop::new(self);
        let ptr = this.as_mut_ptr();
        let len = this.len();

        // Safety: This is safe, as we are transferring the ownership
        // of the array to the box.
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr, len);
            Ok(Box::from_raw_in(slice, FimoAllocator))
        }
    }

    /// Shortens the array, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater or equal to the array's current length, this has no effect.
    ///
    /// # Examples
    ///
    /// Truncating a five element array to two elements:
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut array = array_list![1, 2, 3, 4, 5].unwrap();
    /// array.truncate(2);
    /// assert_eq!(array, array_list![1, 2].unwrap());
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the array's current length:
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut array = array_list![1, 2, 3].unwrap();
    /// array.truncate(8);
    /// assert_eq!(array, array_list![1, 2, 3].unwrap());
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the
    /// [`clear`](ArrayList::clear) method.
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut array = array_list![1, 2, 3].unwrap();
    /// array.truncate(0);
    /// assert_eq!(array, []);
    /// ```
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            if !core::mem::needs_drop::<T>() {
                // Safety: We shortened the array, which still
                // only contains initialized objects.
                unsafe { self.set_len(len) };
            }

            let len_ptr = &mut self.inner.size as *mut usize;
            let mut array_len = self.len();
            let slice = self.uninit_slice_mut();
            let drop_range = slice.index_mut(len..array_len);
            for x in drop_range.iter_mut().rev() {
                array_len -= 1;

                // Safety: The length is not aliased.
                unsafe { len_ptr.write(array_len) };

                // Safety: We know that the element is initialized.
                unsafe { x.assume_init_drop() }
            }
        }
    }

    /// Extracts a slice containing the entire array.
    ///
    /// Equivalent to `&s[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let array = array_list![1, 2, 3, 5, 8].unwrap();
    /// assert_eq!(array.as_slice(), [1, 2, 3, 5, 8]);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        let slice = &self.uninit_slice()[..self.len()];

        // Safety: The elements `0..len` are initialized by the invariants of
        // the array list.
        unsafe { MaybeUninit::slice_assume_init_ref(slice) }
    }

    /// Extracts a mutable slice containing the entire array.
    ///
    /// Equivalent to `&mut s[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut array = array_list![0; 3].unwrap();
    /// array.as_mut_slice()[1] = 1;
    /// assert_eq!(array, [0, 1, 0]);
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        let slice = &mut self.uninit_slice_mut()[..len];

        // Safety: The elements `0..len` are initialized by the invariants of
        // the array list.
        unsafe { MaybeUninit::slice_assume_init_mut(slice) }
    }

    /// Returns a raw pointer to the array's buffer, or a dangling raw
    /// pointer valid for zero sized reads if the array didn't allocate.
    ///
    /// The caller must ensure that the array outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the array may cause its buffer to be reallocated,
    /// which would also make any pointer to it invalid.
    ///
    /// The caller must also ensure that the memory to the pointer
    /// (non-transitively) points to is never written to (except inside
    /// an `UnsafeCell`) using this pointer or any pointer derived from
    /// it. If you need to mutate the contents of the slice, use
    /// [`as_mut_ptr`](ArrayList::as_mut_ptr).
    ///
    /// This method guarantees that for the purpose of the aliasing
    /// model, this method does not materialize a reference to the
    /// underlying slice, and thus the returned pointer will remain
    /// valid when mixed with other calls to [`as_ptr`](ArrayList::as_ptr)
    /// and [`as_mut_ptr`](ArrayList::as_mut_ptr). Note that calling
    /// other methods that materialize mutable references to the slice,
    /// or mutable references to specific elements you are planning
    /// on accessing through this pointer, as well as writing to those
    /// elements, may still invalidate this pointer. See the second
    /// example below for how this guarantee can be used.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let x = array_list![1, 2, 4].unwrap();
    /// let x_ptr = x.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..x.len() {
    ///         assert_eq!(*x_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// ```
    ///
    /// Due to the aliasing guarantee, the following code is legal:
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// unsafe {
    ///     let mut a = array_list![0, 1, 2].unwrap();
    ///     let ptr1 = a.as_ptr();
    ///     let _ = ptr1.read();
    ///     let ptr2 = a.as_mut_ptr().offset(2);
    ///     ptr2.write(2);
    ///     // Notably, the write to `ptr2` did *not* invalidate `ptr1`
    ///     // because it mutated a different element:
    ///     let _ = ptr1.read();
    /// }
    /// ```
    pub fn as_ptr(&self) -> *const T {
        if self.capacity() == 0 || Self::T_SIZE == 0 {
            NonNull::dangling().as_ptr()
        } else {
            self.inner.elements.cast()
        }
    }

    /// Returns an unsafe mutable pointer to the array's buffer,
    /// or a dangling raw pointer valid for zero sized reads if
    /// the array didn't allocate.
    ///
    /// The caller must ensure that the array outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the array may cause its buffer to be reallocated,
    /// which would also make any pointer to it invalid.
    ///
    /// This method guarantees that for the purpose of the aliasing
    /// model, this method does not materialize a reference to the
    /// underlying slice, and thus the returned pointer will remain
    /// valid when mixed with other calls to [`as_ptr`](ArrayList::as_ptr)
    /// and [`as_mut_ptr`](ArrayList::as_mut_ptr). Note that calling
    /// other methods that materialize references to the slice, or
    /// references to specific elements you are planning on accessing
    /// through this pointer, may still invalidate this pointer.
    /// See the second example below for how this guarantee can be used.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list::ArrayList;
    ///
    /// // Allocate array big enough for 4 elements.
    /// let size = 4;
    /// let mut x: ArrayList<i32> = ArrayList::with_capacity(size).unwrap();
    /// let x_ptr = x.as_mut_ptr();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         *x_ptr.add(i) = i as i32;
    ///     }
    ///     x.set_len(size);
    /// }
    /// assert_eq!(&*x, &[0, 1, 2, 3]);
    /// ```
    ///
    /// Due to the aliasing guarantee, the following code is legal:
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// unsafe {
    ///     let mut a = array_list![0].unwrap();
    ///     let ptr1 = a.as_mut_ptr();
    ///     ptr1.write(1);
    ///     let ptr2 = a.as_mut_ptr();
    ///     ptr2.write(2);
    ///     // Notably, the write to `ptr2` did *not* invalidate `ptr1`:
    ///     ptr1.write(3);
    /// }
    /// ```
    pub fn as_mut_ptr(&mut self) -> *mut T {
        if self.capacity() == 0 || Self::T_SIZE == 0 {
            NonNull::dangling().as_ptr()
        } else {
            self.inner.elements.cast()
        }
    }

    /// Returns a reference to the underlying allocator.
    pub fn allocator(&self) -> &FimoAllocator {
        &FimoAllocator
    }

    /// Forces the length of the array to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the
    /// normal invariants of the type. Normally changing the
    /// length if an array is done using one of the safe operations
    /// instead, such as [`truncate`](ArrayList::truncate),
    /// [`resize`](ArrayList::resize), [`extend`](ArrayList::extend),
    /// or [`clear`](ArrayList::clear).
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity()`](ArrayList::capacity)
    /// - The elements at `old_len..new_len` must be initialized.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.inner.size = new_len;
    }

    /// Removes an element from the array and returns it.
    ///
    /// The removed element is replaced by the last element of the array.
    /// This does not preserve ordering, but is *O(1)*. If you need to
    /// preserve the element order, use [`remove`](ArrayList::remove)
    /// instead.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut a = array_list!["foo", "bar", "baz", "qux"].unwrap();
    ///
    /// assert_eq!(a.swap_remove(1).unwrap(), "bar");
    /// assert_eq!(a, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(a.swap_remove(0).unwrap(), "foo");
    /// assert_eq!(a, ["baz", "qux"]);
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> Result<T, Error> {
        if index >= self.len() {
            return Err(Error::EINVAL);
        }

        let last_idx = self.len() - 1;
        self.as_mut_slice().swap(index, last_idx);

        self.pop_back_internal()
    }

    /// Inserts an element at position `index` within the array,
    /// shifting all elements after it to the right.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// arr.insert(1, 4).unwrap();
    /// assert_eq!(arr, [1, 4, 2, 3]);
    /// arr.insert(4, 5).unwrap();
    /// assert_eq!(arr, [1, 4, 2, 3, 5]);
    /// ```
    pub fn insert(&mut self, index: usize, element: T) -> Result<(), InsertionErr<T>> {
        let mut element = ManuallyDrop::new(element);
        // Safety: The pointers are valid.
        match unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_insert(
                    &mut self.inner,
                    index,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    core::ptr::addr_of_mut!(element).cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        } {
            Ok(_) => Ok(()),
            Err(e) => Err(InsertionErr {
                error: e,
                element: ManuallyDrop::into_inner(element),
            }),
        }
    }

    /// Inserts an element at position `index` within the array, if there is
    /// sufficient spare capacity, otherwise an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// arr.set_capacity_exact(4).unwrap();
    /// arr.try_insert(1, 4).unwrap();
    /// assert_eq!(arr, [1, 4, 2, 3]);
    /// assert!(arr.try_insert(0, 5).is_err());
    /// ```
    pub fn try_insert(&mut self, index: usize, element: T) -> Result<(), InsertionErr<T>> {
        let mut element = ManuallyDrop::new(element);
        // Safety: The pointers are valid.
        match unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_try_insert(
                    &mut self.inner,
                    index,
                    Self::T_SIZE,
                    core::ptr::addr_of_mut!(element).cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        } {
            Ok(_) => Ok(()),
            Err(e) => Err(InsertionErr {
                error: e,
                element: ManuallyDrop::into_inner(element),
            }),
        }
    }

    /// Removes and returns the element at position `index` within the array,
    /// shifting all elements after it to the left.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// assert_eq!(arr.remove(1), Ok(2));
    /// assert_eq!(arr, [1, 3]);
    /// ```
    pub fn remove(&mut self, index: usize) -> Result<T, Error> {
        // Safety: The element is initialized or an error is written.
        unsafe {
            to_result_indirect_in_place::<T>(|error, element| {
                *error = bindings::fimo_array_list_remove(
                    &mut self.inner,
                    index,
                    Self::T_SIZE,
                    element.as_mut_ptr().cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Removes all elements `e` for which `f(&e)` returns `false`.
    /// This method operates in place, visiting each element exactly once
    /// in the original order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3, 4].unwrap();
    /// arr.retain(|&x| x % 2 == 0);
    /// assert_eq!(arr, [2, 4]);
    /// ```
    pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) -> error::Result {
        let mut i = 0;
        while i < self.len() {
            // Safety: We checked that `i` is in range.
            let element = unsafe { self.get_unchecked(i) };
            if f(element) {
                i += 1;
            } else if let Err(e) = self.remove(i) {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Removes all elements `e` for which `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once
    /// in the original order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3, 4].unwrap();
    /// arr.retain_mut(|x| {
    ///     if *x <= 3 {
    ///         *x += 1;
    ///         true
    ///     } else {
    ///         false
    ///     }
    /// });
    /// assert_eq!(arr, [2, 3, 4]);
    /// ```
    pub fn retain_mut(&mut self, mut f: impl FnMut(&mut T) -> bool) -> error::Result {
        let mut i = 0;
        while i < self.len() {
            // Safety: We checked that `i` is in range.
            let element = unsafe { self.get_unchecked_mut(i) };
            if f(element) {
                i += 1;
            } else if let Err(e) = self.remove(i) {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Appends an element to the back of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2].unwrap();
    /// arr.push(3).unwrap();
    /// assert_eq!(arr, [1, 2, 3]);
    /// ```
    pub fn push(&mut self, element: T) -> Result<(), InsertionErr<T>> {
        let mut element = ManuallyDrop::new(element);
        // Safety: The pointers are valid.
        if let Err(e) = unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_push(
                    &mut self.inner,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    core::ptr::addr_of_mut!(element).cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        } {
            Err(InsertionErr {
                error: e,
                element: ManuallyDrop::into_inner(element),
            })
        } else {
            Ok(())
        }
    }

    /// Appends an element to the back of the array, if there is sufficient
    /// capacity, otherwise an error is returned with the element.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2].unwrap();
    /// arr.set_capacity_exact(3).unwrap();
    /// arr.try_push(3).unwrap();
    /// assert!(arr.try_push(4).is_err());
    /// assert_eq!(arr, [1, 2, 3]);
    /// ```
    pub fn try_push(&mut self, element: T) -> Result<(), InsertionErr<T>> {
        let mut element = ManuallyDrop::new(element);
        // Safety: The pointers are valid.
        if let Err(e) = unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_try_push(
                    &mut self.inner,
                    Self::T_SIZE,
                    core::ptr::addr_of_mut!(element).cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        } {
            Err(InsertionErr {
                error: e,
                element: ManuallyDrop::into_inner(element),
            })
        } else {
            Ok(())
        }
    }

    fn pop_front_internal(&mut self) -> Result<T, Error> {
        // Safety: Elem is either initialized or an error is written.
        unsafe {
            to_result_indirect_in_place::<T>(|error, elem| {
                *error = bindings::fimo_array_list_pop_front(
                    &mut self.inner,
                    Self::T_SIZE,
                    elem.as_mut_ptr().cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        }
    }

    /// Removes the first element from an array and returns it, or
    /// [`None`] if it is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// assert_eq!(arr.pop_front(), Some(1));
    /// assert_eq!(arr, [2, 3]);
    /// ```
    pub fn pop_front(&mut self) -> Option<T> {
        self.pop_front_internal().ok()
    }

    fn pop_back_internal(&mut self) -> Result<T, Error> {
        // Safety: Elem is either initialized or an error is written.
        unsafe {
            to_result_indirect_in_place::<T>(|error, elem| {
                *error = bindings::fimo_array_list_pop_back(
                    &mut self.inner,
                    Self::T_SIZE,
                    elem.as_mut_ptr().cast(),
                    Self::T_MOVE_FUNC,
                );
            })
        }
    }

    /// Removes the last element from an array and returns it, or
    /// [`None`] if it is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// assert_eq!(arr.pop_back(), Some(3));
    /// assert_eq!(arr, [1, 2]);
    /// ```
    pub fn pop_back(&mut self) -> Option<T> {
        self.pop_back_internal().ok()
    }

    /// Moves all elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// let mut arr2 = array_list![4, 5, 6].unwrap();
    /// arr.append(&mut arr2).unwrap();
    /// assert_eq!(arr, [1, 2, 3, 4, 5, 6]);
    /// assert_eq!(arr2, []);
    /// ```
    pub fn append(&mut self, other: &mut Self) -> error::Result {
        let count = other.len();
        self.reserve(count)?;
        let len = self.len();

        // Safety: The copy is safe due to the aliasing guarantees
        // of Rust.
        unsafe {
            let this_ptr = self.as_mut_ptr().add(len);
            let other_ptr = other.as_ptr();
            core::ptr::copy_nonoverlapping(other_ptr, this_ptr, count);
            self.set_len(len + count);
            other.set_len(0);
        }

        Ok(())
    }

    /// Moves all elements of `other` into `self`, leaving `other` empty,
    /// if there is sufficient capacity, otherwise an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// let mut arr2 = array_list![4, 5, 6].unwrap();
    /// let mut arr3 = array_list![7, 8, 9].unwrap();
    /// arr.set_capacity_exact(8);
    /// arr.try_append(&mut arr2).unwrap();
    /// assert!(arr.try_append(&mut arr3).is_err());
    /// assert_eq!(arr, [1, 2, 3, 4, 5, 6]);
    /// assert_eq!(arr2, []);
    /// assert_eq!(arr3, [7, 8, 9]);
    /// ```
    pub fn try_append(&mut self, other: &mut Self) -> error::Result {
        let count = other.len();
        let len = self.len();
        let new_len = match len.checked_add(count) {
            Some(x) if isize::try_from(x).is_ok() && x < self.capacity() => x,
            Some(_) | None => return Err(Error::EINVAL),
        };

        // Safety: The copy is safe due to the aliasing guarantees
        // of Rust.
        unsafe {
            let this_ptr = self.as_mut_ptr().add(len);
            let other_ptr = other.as_ptr();
            core::ptr::copy_nonoverlapping(other_ptr, this_ptr, count);
            self.set_len(new_len);
            other.set_len(0);
        }

        Ok(())
    }

    /// Clears the array, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// arr.clear();
    /// assert_eq!(arr, []);
    /// ```
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Returns the number of elements contained in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// assert_eq!(arr.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.inner.size
    }

    /// Returns whether the array is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::array_list::ArrayList;
    ///
    /// let mut arr = ArrayList::new();
    /// assert!(arr.is_empty());
    ///
    /// arr.push(1);
    /// assert!(!arr.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Splits the array into two at the given index.
    ///
    /// Returns a newly allocated array containing the elements int the
    /// range `[at, len)`. After the call, the original array will be left
    /// containing the elements `[0, at)` with its previous capacity unchanged.
    ///
    /// - If you want to take ownership of the entire contents and capacity of the array, see
    ///   [`mem::take`](core::mem::take) or [`mem::replace`](core::mem::replace).
    /// - If you don't need the returned array at all, see [`ArrayList::truncate`].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// let arr2 = arr.split_off(1).unwrap();
    /// assert_eq!(arr, [1]);
    /// assert_eq!(arr2, [2, 3]);
    /// ```
    pub fn split_off(&mut self, at: usize) -> Result<ArrayList<T>, Error> {
        if at > self.len() {
            return Err(Error::EINVAL);
        }

        let count = self.len() - at;
        let mut other = ArrayList::with_capacity(count)?;

        // Safety: Due to `other` being a new allocation, we know that
        // the two ranges are not overlapping. We checked that the two
        // buffers are in bounds.
        unsafe {
            let this_ptr = self.as_ptr().add(at);
            let other_ptr = other.as_mut_ptr();
            core::ptr::copy_nonoverlapping(this_ptr, other_ptr, count);
            self.set_len(at);
            other.set_len(count);
        }

        Ok(other)
    }

    /// Resizes the array in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the array is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the array in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the array is simply truncated.
    ///
    /// This method uses a closure to create new values on every push.
    /// If you'd rather [`Clone`] a given value, use [`ArrayList::resize`].
    /// If you want to use the [`Default`] trait to generate values, you
    /// can pass [`Default::default`] as the second argument.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// arr.resize_with(5, Default::default);
    /// assert_eq!(arr, [1, 2, 3, 0, 0]);
    ///
    /// let mut arr = array_list![].unwrap();
    /// let mut p = 1;
    /// arr.resize_with(4, || {
    ///     p *= 2;
    ///     p
    /// })
    /// .unwrap();
    /// assert_eq!(arr, [2, 4, 8, 16]);
    /// ```
    pub fn resize_with(&mut self, new_len: usize, mut f: impl FnMut() -> T) -> Result<(), Error> {
        if new_len < self.len() {
            self.truncate(new_len);
            return Ok(());
        }

        let additional = new_len - self.len();
        self.reserve(additional)?;
        for _ in 0..additional {
            self.try_push(f()).map_err(|e| e.into_error())?;
        }

        Ok(())
    }

    /// Resizes the array in-place so that `len` is equal to `new_len`,
    /// if there is sufficient capacity, otherwise an error is returned.
    ///
    /// If `new_len` is greater than `len`, the array is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the array in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the array is simply truncated.
    ///
    /// This method uses a closure to create new values on every push.
    /// If you'd rather [`Clone`] a given value, use [`ArrayList::resize`].
    /// If you want to use the [`Default`] trait to generate values, you
    /// can pass [`Default::default`] as the second argument.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// arr.set_capacity(5).unwrap();
    /// arr.try_resize_with(5, Default::default).unwrap();
    /// assert_eq!(arr, [1, 2, 3, 0, 0]);
    ///
    /// let mut arr = array_list![].unwrap();
    /// let mut p = 1;
    /// assert!(arr
    ///     .try_resize_with(4, || {
    ///         p *= 2;
    ///         p
    ///     })
    ///     .is_err());
    /// assert_eq!(arr, []);
    /// ```
    pub fn try_resize_with(
        &mut self,
        new_len: usize,
        mut f: impl FnMut() -> T,
    ) -> Result<(), Error> {
        if new_len <= self.len() {
            self.truncate(new_len);
            return Ok(());
        }
        if new_len > self.capacity() {
            return Err(Error::EINVAL);
        }

        let additional = new_len - self.len();
        self.reserve(additional)?;
        for _ in 0..additional {
            self.try_push(f()).map_err(|e| e.into_error())?;
        }

        Ok(())
    }

    /// Consumes and leaks the array, returning a mutable reference to
    /// the contents, `&'a mut [T]`. Note that the type `T` must outlive
    /// the chosen lifetime `'a`. If the type has only static references,
    /// or none at all, then this may be chosen to be `'static`.
    ///
    /// The leaked allocation may include unused capacity that is not
    /// part of the returned slice.
    ///
    /// This function is mainly useful for data that lives for the
    /// remainder of the program's life. Dropping the returned reference
    /// will cause a memory leak.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 2, 3].unwrap();
    /// let static_ref: &'static mut [usize] = arr.leak();
    /// static_ref[0] += 1;
    /// assert_eq!(static_ref, &[2, 2, 3]);
    /// ```
    pub fn leak<'a>(self) -> &'a mut [T] {
        let mut this = ManuallyDrop::new(self);

        // Safety: Sound, as we know that `[0, len)` is initialized.
        unsafe { core::slice::from_raw_parts_mut(this.as_mut_ptr(), this.len()) }
    }

    /// Returns the remaining spare capacity of the array as a slice
    /// of `MaybeUninit<T>`.
    ///
    /// The returned slice can be used to fill the array with data
    /// (e.g. by reading from a file) before marking the data as
    /// initialized using the [`ArrayList::set_len`] method.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list::ArrayList;
    ///
    /// // Allocate an array big enough for 10 elements.
    /// let mut arr = ArrayList::with_capacity(10).unwrap();
    ///
    /// // Fill in the first 3 elements.
    /// let uninit = arr.spare_capacity_mut();
    /// uninit[0].write(0);
    /// uninit[1].write(1);
    /// uninit[2].write(2);
    ///
    /// // Mark the first 3 elements of the array as being initialized.
    /// unsafe {
    ///     arr.set_len(3);
    /// }
    ///
    /// assert_eq!(arr, [0, 1, 2]);
    /// ```
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let len = self.len();
        let slice = self.uninit_slice_mut();
        &mut slice[len..]
    }

    /// Returns the array content as a slice of `T`, along with the remaining
    /// spare capacity of the array as a slice of `MaybeUninit<T>`.
    ///
    /// The returned spare capacity can be used to fill the array with data
    /// (e.g. by reading from a file) before marking the data as initialized
    /// using the [`ArrayList::set_len`] method.
    ///
    /// Note that this is a low-level API, which should be used with care for
    /// optimization purposes. If you need to append data to an `ArrayList`
    /// you can use [`push`](ArrayList::push), [`extend`](ArrayList::extend),
    /// [`insert`](ArrayList::insert), [`append`](ArrayList::append),
    /// [`resize`](ArrayList::resize), [`resize_with`](ArrayList::resize_with),
    /// depending on your exact needs.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list![1, 1, 2].unwrap();
    ///
    /// // Reserve additional space big enough for 10 elements.
    /// arr.reserve(10).unwrap();
    ///
    /// let (init, uninit) = arr.split_at_spare_mut();
    /// let sum = init.iter().copied().sum::<u32>();
    ///
    /// // Fill in the next 4 elements.
    /// uninit[0].write(sum);
    /// uninit[1].write(sum * 2);
    /// uninit[2].write(sum * 3);
    /// uninit[3].write(sum * 4);
    ///
    /// // Mark the first 3 elements of the array as being initialized.
    /// unsafe {
    ///     let len = arr.len();
    ///     arr.set_len(len + 4);
    /// }
    ///
    /// assert_eq!(arr, [1, 1, 2, 4, 8, 12, 16]);
    /// ```
    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [MaybeUninit<T>]) {
        let len = self.len();
        let slice = self.uninit_slice_mut();
        let (init, uninit) = slice.split_at_mut(len);

        // Safety: We know that `[0, len)` is initialized.
        let init = unsafe { MaybeUninit::slice_assume_init_mut(init) };

        (init, uninit)
    }
}

impl<T> ArrayList<T> {
    /// Resizes the allocation of the array, so that it can contain
    /// at least `capacity` elements.
    ///
    /// If the new capacity of the array is smaller than the length,
    /// this function will truncate the array appropriately.
    pub fn set_capacity(&mut self, capacity: usize) -> error::Result {
        // Safety: All pointers are valid.
        unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_set_capacity(
                    &mut self.inner,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    capacity,
                    Self::T_MOVE_FUNC,
                    Self::T_DROP_FUNC,
                );
            })
        }
    }

    pub fn set_capacity_exact(&mut self, capacity: usize) -> error::Result {
        // Safety: All pointers are valid.
        unsafe {
            to_result_indirect(|error| {
                *error = bindings::fimo_array_list_set_capacity_exact(
                    &mut self.inner,
                    Self::T_SIZE,
                    Self::T_ALIGN,
                    capacity,
                    Self::T_MOVE_FUNC,
                    Self::T_DROP_FUNC,
                );
            })
        }
    }

    /// Returns a slice to the entire allocated memory of the array.
    fn uninit_slice(&self) -> &[MaybeUninit<T>] {
        if self.capacity() == 0 {
            &[]
        } else if Self::T_SIZE == 0 {
            // Safety: This operation is safe on ZST types.
            unsafe { core::slice::from_raw_parts(NonNull::dangling().as_ptr(), self.capacity()) }
        } else {
            // Safety: The pointer is properly aligned and has the right size.
            unsafe { core::slice::from_raw_parts(self.inner.elements.cast(), self.capacity()) }
        }
    }

    /// Returns a mutable slice to the entire allocated memory of the array.
    fn uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        if self.capacity() == 0 {
            &mut []
        } else if Self::T_SIZE == 0 {
            // Safety: This operation is safe on ZST types.
            unsafe {
                core::slice::from_raw_parts_mut(NonNull::dangling().as_ptr(), self.capacity())
            }
        } else {
            // Safety: The pointer is properly aligned and has the right size.
            unsafe { core::slice::from_raw_parts_mut(self.inner.elements.cast(), self.capacity()) }
        }
    }

    /// Retrieves the drop function.
    const fn drop_func() -> bindings::FimoArrayListDropFunc {
        unsafe extern "C" fn cleanup_fn<T>(ptr: *mut core::ffi::c_void) {
            let ptr = if core::mem::size_of::<T>() == 0 {
                NonNull::<T>::dangling().as_ptr()
            } else {
                ptr.cast()
            };

            // Safety: The element at `ptr` is initialized and will be deallocated directly
            // after.
            unsafe { core::ptr::drop_in_place(ptr) };
        }
        if core::mem::needs_drop::<T>() {
            Some(cleanup_fn::<T> as _)
        } else {
            None
        }
    }

    fn extend_desugared(&mut self, mut iterator: impl Iterator<Item = T>) -> error::Result {
        while let Some(element) = iterator.next() {
            let len = self.len();
            if len == self.capacity() {
                let (lower, _) = iterator.size_hint();
                self.reserve(lower.saturating_add(1))?;
            }

            // Safety: The buffer is in bounds.
            unsafe {
                core::ptr::write(self.as_mut_ptr().add(len), element);
                self.set_len(len + 1);
            }
        }

        Ok(())
    }
}

impl<T> ArrayList<T>
where
    T: Clone,
{
    /// Resizes the array in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the array is extended by the
    /// difference, with each additional slot filled with `value`. If
    /// `new_len` is less than `len`, the array is simply truncated.
    ///
    /// This method requires `T` to implement [`Clone`], in order to
    /// be able to clone the passed value. If you need more flexibility
    /// (or want to rely on [`Default`] instead of [`Clone`]), use
    /// [`ArrayList::resize_with`]. If you only need to resize to a
    /// smaller size, use [`ArrayList::truncate`].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list!["hello"].unwrap();
    /// arr.resize(3, "world").unwrap();
    /// assert_eq!(arr, ["hello", "world", "world"]);
    ///
    /// let mut arr = array_list![1, 2, 3, 4].unwrap();
    /// arr.resize(2, 0).unwrap();
    /// assert_eq!(arr, [1, 2]);
    /// ```
    pub fn resize(&mut self, new_len: usize, value: T) -> Result<(), InsertionErr<T>> {
        let len = self.len();

        if new_len > len {
            let additional = new_len - len;
            if let Err(e) = self.reserve(additional) {
                return Err(InsertionErr {
                    error: e,
                    element: value,
                });
            }

            // Safety: Is sound, as we are initializing the range `[len, new_len)`.
            unsafe {
                let mut ptr = self.as_mut_ptr().add(self.len());

                for _ in 1..additional {
                    core::ptr::write(ptr, value.clone());
                    ptr = ptr.add(1);
                    // Increment the length in every step in case clone() panics
                    self.inner.size += 1;
                }

                if additional > 0 {
                    core::ptr::write(ptr, value);
                    self.inner.size += 1;
                }
            }
        } else {
            self.truncate(new_len);
        }

        Ok(())
    }

    /// Resizes the array in-place so that `len` is equal to `new_len`,
    /// if there is sufficient capacity, otherwise an error is returned.
    ///
    /// If `new_len` is greater than `len`, the array is extended by the
    /// difference, with each additional slot filled with `value`. If
    /// `new_len` is less than `len`, the array is simply truncated.
    ///
    /// This method requires `T` to implement [`Clone`], in order to
    /// be able to clone the passed value. If you need more flexibility
    /// (or want to rely on [`Default`] instead of [`Clone`]), use
    /// [`ArrayList::resize_with`]. If you only need to resize to a
    /// smaller size, use [`ArrayList::truncate`].
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use fimo_std::array_list;
    ///
    /// let mut arr = array_list!["hello"].unwrap();
    /// arr.set_capacity(3).unwrap();
    /// arr.try_resize(3, "world").unwrap();
    /// assert_eq!(arr, ["hello", "world", "world"]);
    ///
    /// let mut arr = array_list![1].unwrap();
    /// arr.set_capacity_exact(3).unwrap();
    /// assert!(arr.try_resize(4, 0).is_err());
    /// assert_eq!(arr, [1]);
    /// ```
    pub fn try_resize(&mut self, new_len: usize, value: T) -> Result<(), InsertionErr<T>> {
        let len = self.len();

        if new_len > len {
            if new_len > self.capacity() {
                return Err(InsertionErr {
                    error: Error::EINVAL,
                    element: value,
                });
            }

            let additional = new_len - len;

            // Safety: Is sound, as we are initializing the range `[len, new_len)`.
            unsafe {
                let mut ptr = self.as_mut_ptr().add(self.len());

                for _ in 1..additional {
                    core::ptr::write(ptr, value.clone());
                    ptr = ptr.add(1);
                    // Increment the length in every step in case clone() panics
                    self.inner.size += 1;
                }

                if additional > 0 {
                    core::ptr::write(ptr, value);
                    self.inner.size += 1;
                }
            }
        } else {
            self.truncate(new_len);
        }

        Ok(())
    }
}

impl<T> AsRef<[T]> for ArrayList<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for ArrayList<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> AsRef<ArrayList<T>> for ArrayList<T> {
    fn as_ref(&self) -> &ArrayList<T> {
        self
    }
}

impl<T> AsMut<ArrayList<T>> for ArrayList<T> {
    fn as_mut(&mut self) -> &mut ArrayList<T> {
        self
    }
}

impl<T> Borrow<[T]> for ArrayList<T> {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T> BorrowMut<[T]> for ArrayList<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> Clone for ArrayList<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut array = Self::with_capacity(self.len()).expect("not enough memory available");
        for x in &**self {
            let _ = array.try_push(x.clone());
        }
        array
    }
}

impl<T> Debug for ArrayList<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T> Default for ArrayList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for ArrayList<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for ArrayList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Drop for ArrayList<T> {
    fn drop(&mut self) {
        // Safety: We own the array and made sure to drop all elements.
        unsafe {
            bindings::fimo_array_list_free(
                &mut self.inner,
                Self::T_SIZE,
                Self::T_ALIGN,
                Self::T_DROP_FUNC,
            );
        }
    }
}

impl<'a, T> Extend<&'a T> for ArrayList<T>
where
    T: Copy + 'a,
{
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend_desugared(iter.into_iter().copied())
            .expect("array should not run out of memory");
    }

    #[inline]
    fn extend_one(&mut self, &item: &'a T) {
        self.push(item).expect("array should not run out of memory");
    }

    #[inline]
    fn extend_reserve(&mut self, additional: usize) {
        self.reserve(additional)
            .expect("array should not run out of memory");
    }
}

impl<T> Extend<T> for ArrayList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend_desugared(iter.into_iter())
            .expect("array should not run out of memory");
    }

    #[inline]
    fn extend_one(&mut self, item: T) {
        self.push(item).expect("array should not run out of memory");
    }

    #[inline]
    fn extend_reserve(&mut self, additional: usize) {
        self.reserve(additional)
            .expect("array should not run out of memory");
    }
}

impl<T> From<&[T]> for ArrayList<T>
where
    T: Clone,
{
    fn from(value: &[T]) -> Self {
        from_slice(value).expect("system should not run out of memory")
    }
}

impl<T> From<&mut [T]> for ArrayList<T>
where
    T: Clone,
{
    fn from(value: &mut [T]) -> Self {
        from_slice(value).expect("system should not run out of memory")
    }
}

impl<T, const N: usize> From<&[T; N]> for ArrayList<T>
where
    T: Clone,
{
    fn from(value: &[T; N]) -> Self {
        ArrayList::from(value.as_slice())
    }
}

impl<T, const N: usize> From<&mut [T; N]> for ArrayList<T>
where
    T: Clone,
{
    fn from(value: &mut [T; N]) -> Self {
        ArrayList::from(value.as_slice())
    }
}

impl<T, const N: usize> From<[T; N]> for ArrayList<T> {
    fn from(value: [T; N]) -> Self {
        let b = Box::try_new_in(value, FimoAllocator);
        from_box(b).expect("system should not run out of memory")
    }
}

impl From<&str> for ArrayList<u8> {
    fn from(value: &str) -> Self {
        ArrayList::from(value.as_bytes())
    }
}

impl<T> From<Box<[T], FimoAllocator>> for ArrayList<T> {
    fn from(value: Box<[T], FimoAllocator>) -> Self {
        from_box_slice(value).expect("system should not run out of memory")
    }
}

impl<T> From<ArrayList<T>> for Box<[T], FimoAllocator> {
    fn from(mut value: ArrayList<T>) -> Self {
        value
            .shrink_to_fit()
            .expect("system should not run out of memory");
        let raw = value.leak() as *mut _;

        // Safety: The buffer is valid and unowned.
        unsafe { Box::from_raw_in(raw, FimoAllocator) }
    }
}

impl<T> From<alloc::vec::Vec<T, FimoAllocator>> for ArrayList<T> {
    fn from(value: alloc::vec::Vec<T, FimoAllocator>) -> Self {
        let (ptr, len, cap) = value.into_raw_parts();
        if Self::T_SIZE == 0 {
            Self {
                inner: bindings::FimoArrayList {
                    elements: core::ptr::null_mut(),
                    size: len,
                    capacity: len,
                },
                _phantom: PhantomData,
            }
        } else if cap == 0 {
            Self {
                inner: bindings::FimoArrayList {
                    elements: core::ptr::null_mut(),
                    size: 0,
                    capacity: 0,
                },
                _phantom: PhantomData,
            }
        } else {
            Self {
                inner: bindings::FimoArrayList {
                    elements: ptr.cast(),
                    size: len,
                    capacity: cap,
                },
                _phantom: PhantomData,
            }
        }
    }
}

impl<T> From<ArrayList<T>> for alloc::vec::Vec<T, FimoAllocator> {
    fn from(value: ArrayList<T>) -> Self {
        let mut value = ManuallyDrop::new(value);
        if ArrayList::<T>::T_SIZE == 0 {
            let mut v = alloc::vec::Vec::new_in(FimoAllocator);
            // Safety: `T` is a ZST.
            unsafe { v.set_len(value.len()) };
            v
        } else if value.capacity() == 0 {
            alloc::vec::Vec::new_in(FimoAllocator)
        } else {
            let ptr = value.as_mut_ptr();
            let len = value.len();
            let cap = value.capacity();

            // Safety: Is sound, as we transfer the ownership of the entire buffer.
            unsafe { alloc::vec::Vec::from_raw_parts_in(ptr, len, cap, FimoAllocator) }
        }
    }
}

impl<T> FromIterator<T> for ArrayList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = ArrayList::new();
        array.extend(iter);
        array
    }
}

impl<T> Hash for ArrayList<T>
where
    T: Hash,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state);
    }
}

impl<T, I> Index<I> for ArrayList<T>
where
    I: SliceIndex<[T]>,
{
    type Output = <I as SliceIndex<[T]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T, I> IndexMut<I> for ArrayList<T>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<'a, T> IntoIterator for &'a ArrayList<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ArrayList<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for ArrayList<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T, FimoAllocator>;

    fn into_iter(self) -> Self::IntoIter {
        let vec = alloc::vec::Vec::from(self);
        vec.into_iter()
    }
}

impl<T, U> PartialEq<&[U]> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &&[U]) -> bool {
        &**self == *other
    }
}

impl<T, U, const N: usize> PartialEq<&[U; N]> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &&[U; N]) -> bool {
        &**self == *other
    }
}

impl<T, U> PartialEq<&mut [U]> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &&mut [U]) -> bool {
        &**self == *other
    }
}

impl<T, U, const N: usize> PartialEq<&mut [U; N]> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &&mut [U; N]) -> bool {
        &**self == *other
    }
}

impl<T, U> PartialEq<[U]> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &[U]) -> bool {
        &**self == other
    }
}

impl<T, U, const N: usize> PartialEq<[U; N]> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &[U; N]) -> bool {
        &**self == other
    }
}

impl<T, U> PartialEq<ArrayList<U>> for &[T]
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &ArrayList<U>) -> bool {
        *self == &**other
    }
}

impl<T, U> PartialEq<ArrayList<U>> for &mut [T]
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &ArrayList<U>) -> bool {
        *self == &**other
    }
}

impl<T, U> PartialEq<ArrayList<U>> for [T]
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &ArrayList<U>) -> bool {
        self == &**other
    }
}

impl<T, U> PartialEq<ArrayList<U>> for ArrayList<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &ArrayList<U>) -> bool {
        **self == **other
    }
}

impl<T> PartialOrd<ArrayList<T>> for ArrayList<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &ArrayList<T>) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T> Eq for ArrayList<T> where T: Eq {}

impl<T> Ord for ArrayList<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}

// Safety: `ArrayList` is a owned `[T]`
unsafe impl<T> Send for ArrayList<T> where T: Send {}

// Safety: `ArrayList` is a owned `[T]`
unsafe impl<T> Sync for ArrayList<T> where T: Sync {}

/// An error caused by the [`ArrayList<T>::into_boxed_slice`] method.
pub struct IntoBoxedSliceErr<T> {
    error: Error,
    array: ArrayList<T>,
}

impl<T> IntoBoxedSliceErr<T> {
    /// Returns the contained error value.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Extracts the contained error value.
    pub fn into_error(self) -> Error {
        self.error
    }

    /// Returns a reference to the [`ArrayList<T>`] that caused the error.
    pub fn array(&self) -> &ArrayList<T> {
        &self.array
    }

    /// Extracts the [`ArrayList<T>`] that caused the error.
    pub fn into_array(self) -> ArrayList<T> {
        self.array
    }
}

impl<T> core::fmt::Debug for IntoBoxedSliceErr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IntoBoxedSliceErr")
            .field("error", &self.error)
            .finish()
    }
}

impl<T> core::fmt::Display for IntoBoxedSliceErr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.error)
    }
}

/// An error caused by an insertion operation.
pub struct InsertionErr<T> {
    error: Error,
    element: T,
}

impl<T> InsertionErr<T> {
    /// Returns the contained error value.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Extracts the contained error value.
    pub fn into_error(self) -> Error {
        self.error
    }

    /// Returns a reference to the element that caused the error.
    pub fn element(&self) -> &T {
        &self.element
    }

    /// Extracts the element that caused the error.
    pub fn into_element(self) -> T {
        self.element
    }
}

impl<T> core::fmt::Debug for InsertionErr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InsertionErr")
            .field("error", &self.error)
            .finish()
    }
}

impl<T> core::fmt::Display for InsertionErr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.error)
    }
}

#[doc(hidden)]
pub fn from_elem<T: Clone>(elem: T, n: usize) -> Result<ArrayList<T>, Error> {
    // This is a suboptimal implementation, as we have to insert
    // each element in sequence, even if `T` is `Copy`. In that
    // case it would be faster to insert the elements with a
    // memcpy.
    if n > isize::MAX as usize {
        return Err(Error::EINVAL);
    }

    if n == 0 {
        return Ok(ArrayList::new());
    }

    let mut array = ArrayList::with_capacity(n)?;
    for _ in 0..n - 1 {
        let _ = array.try_push(elem.clone());
    }
    let _ = array.try_push(elem);

    Ok(array)
}

#[doc(hidden)]
pub fn from_slice<T: Clone>(elems: &[T]) -> Result<ArrayList<T>, Error> {
    // This is a suboptimal implementation, as we have to insert
    // each element in sequence, even if `T` is `Copy`. In that
    // case it would be faster to insert the elements with a
    // memcpy.
    if elems.len() > isize::MAX as usize {
        return Err(Error::EINVAL);
    }

    if elems.is_empty() {
        return Ok(ArrayList::new());
    }

    let mut array = ArrayList::<T>::with_capacity(elems.len())?;
    array.extend_desugared(elems.iter().cloned())?;

    Ok(array)
}

#[doc(hidden)]
pub fn from_box<T, const N: usize>(
    elems: Result<Box<[T; N], FimoAllocator>, AllocError>,
) -> Result<ArrayList<T>, Error> {
    let elems = elems.map_err(|_e| <Error>::ENOMEM)?;
    from_box_slice(elems)
}

#[doc(hidden)]
pub fn from_box_slice<T>(elems: Box<[T], FimoAllocator>) -> Result<ArrayList<T>, Error> {
    if elems.is_empty() {
        Ok(ArrayList::new())
    } else {
        if elems.len() > isize::MAX as usize {
            return Err(Error::EINVAL);
        }

        let len = elems.len();
        let capacity = elems.len();
        let ptr = Box::into_raw(elems).cast();

        let inner = bindings::FimoArrayList {
            elements: ptr,
            size: len,
            capacity,
        };
        Ok(ArrayList {
            inner,
            _phantom: PhantomData,
        })
    }
}
