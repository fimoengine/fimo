//! FFI helpers.

use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter, Pointer},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

/// A wrapper around a [`NonNull`] that only allows conversions from and to read-only pointers.
#[repr(transparent)]
pub struct ConstNonNull<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> ConstNonNull<T> {
    /// Creates a new `ConstNonNull` if `ptr` is non-null.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = ConstNonNull::<u32>::new(&raw const x).expect("ptr is null");
    ///
    /// if let Some(ptr) = ConstNonNull::<u32>::new(std::ptr::null()) {
    ///     unreachable!();
    /// }
    /// ```
    pub const fn new(ptr: *const T) -> Option<Self> {
        if !ptr.is_null() {
            unsafe { Some(Self::new_unchecked(ptr.cast_mut())) }
        } else {
            None
        }
    }

    /// Creates a new `ConstNonNull`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = unsafe { ConstNonNull::new_unchecked(&raw const x) };
    /// ```
    ///
    /// Incorrect usage of this function:
    ///
    /// ```rust,no_run
    /// use fimo_std::ffi::ConstNonNull;
    ///
    /// // NEVER DO THAT!!! This is undefined behavior. ⚠️
    /// let ptr = unsafe { ConstNonNull::<u32>::new_unchecked(std::ptr::null()) };
    /// ```
    pub const unsafe fn new_unchecked(ptr: *const T) -> Self {
        unsafe { Self(NonNull::new_unchecked(ptr.cast_mut())) }
    }

    /// Acquires the underlying `*const` ptr.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = ConstNonNull::<u32>::new(&raw const x).expect("ptr is null");
    ///
    /// let x_value = unsafe { *ptr.as_ptr() };
    /// assert_eq!(x_value, 0);
    /// ```
    pub const fn as_ptr(&self) -> *const T {
        self.0.as_ptr().cast_const()
    }

    /// Returns a shared reference to the value.
    ///
    /// # Safety
    ///
    /// Has the same requirements as [`NonNull::as_ref`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = ConstNonNull::<u32>::new(&raw const x).expect("ptr is null");
    ///
    /// let ref_x = unsafe { ptr.as_ref() };
    /// println!("{ref_x}");
    /// ```
    pub const unsafe fn as_ref<'a>(&self) -> &'a T {
        unsafe { self.0.as_ref() }
    }

    /// Casts to a pointer of another type.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = ConstNonNull::<u32>::new(&raw const x).expect("ptr is null");
    ///
    /// let casted_ptr = ptr.cast::<i8>();
    /// let raw_ptr: *const i8 = casted_ptr.as_ptr();
    /// ```
    pub const fn cast<U>(self) -> ConstNonNull<U> {
        ConstNonNull(self.0.cast())
    }
}

impl<T: ?Sized> Debug for ConstNonNull<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ConstNonNull").field(&self.as_ptr()).finish()
    }
}

impl<T: ?Sized> Pointer for ConstNonNull<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.as_ptr(), f)
    }
}

impl<T: ?Sized> Copy for ConstNonNull<T> {}

impl<T: ?Sized> Clone for ConstNonNull<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> PartialEq for ConstNonNull<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.as_ptr(), other.as_ptr())
    }
}

impl<T: ?Sized> Eq for ConstNonNull<T> {}

impl<T: ?Sized> PartialOrd for ConstNonNull<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for ConstNonNull<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .as_ptr()
            .cast::<()>()
            .cmp(&other.0.as_ptr().cast::<()>())
    }
}

impl<T: ?Sized> Hash for ConstNonNull<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> From<&'_ T> for ConstNonNull<T> {
    fn from(value: &'_ T) -> Self {
        unsafe { Self::new_unchecked(&raw const *value) }
    }
}

/// A pointer to a virtual function table.
#[repr(transparent)]
pub struct VTablePtr<T: Send + Sync>(ConstNonNull<T>);

impl<T: Send + Sync> VTablePtr<T> {
    /// Constructs a new pointer from a static reference.
    pub fn new(value: &'static T) -> Self {
        Self(ConstNonNull::from(value))
    }

    /// Constructs a new pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that `value` can be dereferenced for the lifetime of the constructed
    /// instance.
    pub unsafe fn new_unchecked(value: *const T) -> Self {
        unsafe { Self(ConstNonNull::new_unchecked(value.cast_mut())) }
    }
}

impl<T: Send + Sync> Deref for VTablePtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

unsafe impl<T: Send + Sync> Send for VTablePtr<T> {}
unsafe impl<T: Send + Sync> Sync for VTablePtr<T> {}

impl<T: Send + Sync> Debug for VTablePtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VTablePtr").field(&self.0).finish()
    }
}

impl<T: Send + Sync> Pointer for VTablePtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.0, f)
    }
}

impl<T: Send + Sync> Copy for VTablePtr<T> {}

impl<T: Send + Sync> Clone for VTablePtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync> PartialEq for VTablePtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Send + Sync> Eq for VTablePtr<T> {}

impl<T: Send + Sync> PartialOrd for VTablePtr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Send + Sync> Ord for VTablePtr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Send + Sync> Hash for VTablePtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Internal handle to some opaque data.
#[repr(transparent)]
pub struct OpaqueHandle<T: ?Sized = *mut ()>(NonNull<std::ffi::c_void>, PhantomData<T>);

impl<T: ?Sized> OpaqueHandle<T> {
    /// Creates a new `OpaqueHandle` if `ptr` is non-null.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::OpaqueHandle;
    ///
    /// let mut x = 0u32;
    /// let ptr = <OpaqueHandle>::new(&raw mut x).expect("ptr is null");
    ///
    /// if let Some(ptr) = <OpaqueHandle>::new::<u32>(std::ptr::null_mut::<u32>()) {
    ///     unreachable!();
    /// }
    /// ```
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub const fn new<U>(ptr: *mut U) -> Option<Self> {
        if !ptr.is_null() {
            unsafe { Some(Self::new_unchecked(ptr)) }
        } else {
            None
        }
    }

    /// Creates a new `OpaqueHandle`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::OpaqueHandle;
    ///
    /// let mut x = 0u32;
    /// let ptr = unsafe { <OpaqueHandle>::new_unchecked(&raw mut x) };
    /// ```
    ///
    /// Incorrect usage of this function:
    ///
    /// ```rust,no_run
    /// use fimo_std::ffi::OpaqueHandle;
    ///
    /// // NEVER DO THAT!!! This is undefined behavior. ⚠️
    /// let ptr = unsafe { <OpaqueHandle>::new_unchecked(std::ptr::null_mut::<u32>()) };
    /// ```
    pub const unsafe fn new_unchecked<U>(ptr: *mut U) -> Self {
        unsafe { Self(NonNull::new_unchecked(ptr.cast()), PhantomData) }
    }

    /// Acquires the underlying `*const` ptr.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::ffi::OpaqueHandle;
    ///
    /// let mut x = 0u32;
    /// let ptr = <OpaqueHandle>::new(&raw mut x).expect("ptr is null");
    ///
    /// let x_value = unsafe { *ptr.as_ptr::<u32>() };
    /// assert_eq!(x_value, 0);
    /// ```
    pub const fn as_ptr<U>(&self) -> *mut U {
        self.0.as_ptr().cast()
    }
}

unsafe impl<T: Send> Send for OpaqueHandle<T> {}
unsafe impl<T: Sync> Sync for OpaqueHandle<T> {}

impl<T: ?Sized> Debug for OpaqueHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpaqueHandle").field(&self.0).finish()
    }
}

impl<T: ?Sized> Pointer for OpaqueHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.0, f)
    }
}

impl<T: ?Sized> Copy for OpaqueHandle<T> {}

impl<T: ?Sized> Clone for OpaqueHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> PartialEq for OpaqueHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: ?Sized> Eq for OpaqueHandle<T> {}

impl<T: ?Sized> PartialOrd for OpaqueHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for OpaqueHandle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for OpaqueHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[doc(hidden)]
pub struct SendSyncHelper<T>(PhantomData<T>);

unsafe impl<T> Send for SendSyncHelper<T> {}
unsafe impl<T> Sync for SendSyncHelper<T> {}

/// Creates a new handle type.
///
/// # Examples
///
/// ```
/// use fimo_std::handle;
///
/// handle!(handle Foo: Sync);
///
/// let mut x = 0u32;
/// let handle = Foo::new(&raw mut x).expect("ptr is null");
///
/// if let Some(ptr) = Foo::new::<u32>(std::ptr::null_mut()) {
///     unreachable!()
/// }
/// ```
#[macro_export]
macro_rules! handle {
    ($vis:vis handle $ident:ident $(: $bound:ident $(+ $bound_rest:ident)*)?) => {
        /// An opaque handle to some data.
        #[repr(transparent)]
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $ident(core::ptr::NonNull<()> $(, core::marker::PhantomData<dyn $bound $(+ $bound_rest)*>)?);

        impl $ident {
            #[doc=concat!("Creates a new `", stringify!($ident), "` if `ptr` is non-null.")]
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            pub const fn new<T>(ptr: *mut T) -> Option<Self> $(where T: $bound $(+ $bound_rest)* )? {
                if !ptr.is_null() {
                    unsafe { Some(Self::new_unchecked(ptr)) }
                } else {
                    None
                }
            }

            #[doc=concat!("Creates a new `", stringify!($ident), "`.")]
            ///
            /// # Safety
            ///
            /// `ptr` must be non-null.
            pub const unsafe fn new_unchecked<T>(ptr: *mut T) -> Self $(where T: $bound $(+ $bound_rest)* )? {
                unsafe { Self(core::ptr::NonNull::new_unchecked(ptr.cast()), core::marker::PhantomData) }
            }

            /// Acquires the underlying `*mut` ptr.
            pub const fn as_ptr<T>(&self) -> *mut T $(where T: $bound $(+ $bound_rest)* )? {
                self.0.as_ptr().cast()
            }
        }

        $(
        unsafe impl Send for $ident where $crate::ffi::SendSyncHelper<&'static (dyn $bound $(+ $bound_rest)*)>: Send {}
        unsafe impl Sync for $ident where $crate::ffi::SendSyncHelper<&'static (dyn $bound $(+ $bound_rest)*)>: Sync {}
        )?

        impl core::fmt::Debug for $ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($ident)).field(&self.0).finish()
            }
        }

        impl core::fmt::Pointer for $ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Pointer::fmt(&self.0, f)
            }
        }
    };
}

/// Helper trait for types that can be borrowed.
pub trait Viewable<Output: View> {
    /// Borrows a view to the data.
    fn view(&self) -> Output;
}

/// Marker trait for all view types.
pub trait View: Copy {}

impl<T> Viewable<T> for T
where
    T: View,
{
    #[inline(always)]
    fn view(&self) -> T {
        *self
    }
}

/// Used to transfer ownership to and from a ffi interface.
///
/// The ownership of a type is transferred by calling [`Self::into_ffi`] and
/// is transferred back by calling [`Self::from_ffi`].
pub trait FFITransferable<FfiType: Sized> {
    /// Transfers the ownership from a Rust type to a ffi type.
    fn into_ffi(self) -> FfiType;

    /// Assumes ownership of a ffi type.
    ///
    /// # Safety
    ///
    /// The caller must ensure to have the ownership of the ffi type.
    unsafe fn from_ffi(ffi: FfiType) -> Self;
}

/// Used to share ownership with and from a ffi interface.
///
/// The ownership of a type is shared by calling [`Self::share_to_ffi`] and
/// is borrowed by calling [`Self::borrow_from_ffi`].
pub trait FFISharable<FfiType: Sized> {
    type BorrowedView<'a>: 'a;

    /// Shares the value of a Rust type with a ffi type.
    fn share_to_ffi(&self) -> FfiType;

    /// Borrows the ownership of a ffi type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all invariants of the type are conserved.
    unsafe fn borrow_from_ffi<'a>(ffi: FfiType) -> Self::BorrowedView<'a>;
}
