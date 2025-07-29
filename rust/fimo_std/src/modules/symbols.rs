//! Utilities for defining and working with symbols.
use crate::{utils::ConstNonNull, version::Version};
use std::{
    ffi::CStr,
    fmt::{Debug, Display, Pointer},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// Marker trait for types that can safely pass the instance boundary.
///
/// The runtime assumes that instances are isolated entities, with the imports and exports serving
/// as the interface between instances. Under this assumption, the runtime can prevent
/// use-after-free errors by tracking dependencies at the instance level, instead of tracking
/// individual resources of each instance. Strictly speaking, by default, interfaces only depend on
/// the imported symbols of other instances, and not on the instances themselves. As a consequence,
/// the runtime is allowed to silently replace any instance if, under the assumption of instance
/// isolation, the action would not be visible by other instances. The only way to ensure that an
/// instance is not unloaded is to acquire a strong reference to the instance in question with
/// the [`GenericInstance::to_owned_instance`][to_owned_instance] or
/// [`InfoView::try_ref_instance_strong`][try_ref_instance_strong] methods.
///
/// This trait marks whether a type is safe to pass the instance boundary without breaking the
/// assumption of isolation. Notably, this can not be guaranteed for pointer-like types, as they
/// potentially point to data owned by the instance. An exception to this are `'static` references,
/// as they must outlive the programm.
///
/// [to_owned_instance]: crate::module::instance::GenericInstance::to_owned_instance
/// [try_ref_instance_strong]: crate::module::info::InfoView::try_ref_instance_strong
///
/// # Safety
///
/// The implementor must ensure that, once passed the instance boundary, the value remains valid
/// irrespective of the liveness of the originating instance. One possible way to accomplish this is
/// to prevent the runtime from unloading the owning instance while the value is alive.
pub unsafe auto trait Share {}

impl<T> !Share for *const T {}
impl<T> !Share for *mut T {}

unsafe impl<T: Share + ?Sized> Share for &'static T {}
unsafe impl<T: Share + ?Sized> Share for &'static mut T {}

macro_rules! impl_share_fn_peel {
    () => {};
    ($head:ident $(, $($tail:ident),*)?) => {
        impl_share_fn!($($($tail),*)?);
    };
}
macro_rules! impl_share_fn {
    ($($T:ident),*) => {
        unsafe impl<Ret: Share, $($T: Share),*> Share for extern "C" fn($($T),*) -> Ret {}
        unsafe impl<Ret: Share, $($T: Share),*> Share for unsafe extern "C" fn($($T),*) -> Ret {}

        impl_share_fn_peel!($($T),*);
    };
}
impl_share_fn!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

/// Unsafe wrapper type implementing [`Share`].
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssertSharable<T>(T);

impl<T> AssertSharable<T> {
    /// Constructs a new instance.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `value` remains valid even after the destruction of the owning
    /// instance, or ensure that the owning instance can't be unloaded while `value` is used.
    #[inline(always)]
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }

    /// Extracts the contained value.
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

unsafe impl<T> Share for AssertSharable<T> {}

impl<T> Deref for AssertSharable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AssertSharable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Display> Display for AssertSharable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

mod slice_private {
    use super::Share;

    #[const_trait]
    pub trait SliceLength: Share + Copy + Eq {
        fn to_len(self) -> usize;
        fn from_len(x: usize) -> Self;
    }

    // usize >= 16bit
    impl const SliceLength for u8 {
        fn to_len(self) -> usize {
            self as usize
        }
        fn from_len(x: usize) -> Self {
            if x > Self::MAX as usize {
                panic!("overflow detected")
            }
            x as Self
        }
    }
    impl const SliceLength for u16 {
        fn to_len(self) -> usize {
            self as usize
        }
        fn from_len(x: usize) -> Self {
            if x > Self::MAX as usize {
                panic!("overflow detected")
            }
            x as Self
        }
    }

    // usize < 32bit
    #[cfg(target_pointer_width = "16")]
    impl const SliceLength for u32 {
        fn to_len(self) -> usize {
            if self > usize::MAX as Self {
                panic!("overflow detected")
            }
            self as usize
        }
        fn from_len(x: usize) -> Self {
            x as Self
        }
    }
    // usize >= 32bit
    #[cfg(not(target_pointer_width = "16"))]
    impl const SliceLength for u32 {
        fn to_len(self) -> usize {
            self as usize
        }
        fn from_len(x: usize) -> Self {
            if x > Self::MAX as usize {
                panic!("overflow detected")
            }
            x as Self
        }
    }

    // usize < 64bit
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    impl const SliceLength for u64 {
        fn to_len(self) -> usize {
            if self > usize::MAX as Self {
                panic!("overflow detected")
            }
            self as usize
        }
        fn from_len(x: usize) -> Self {
            x as Self
        }
    }
    // usize >= 64bit
    #[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))]
    impl const SliceLength for u64 {
        fn to_len(self) -> usize {
            self as usize
        }
        fn from_len(x: usize) -> Self {
            if x > Self::MAX as usize {
                panic!("overflow detected")
            }
            x as Self
        }
    }

    impl const SliceLength for usize {
        fn to_len(self) -> usize {
            self
        }
        fn from_len(x: usize) -> Self {
            x
        }
    }
}

/// An ffi-safe slice reference.
#[repr(C)]
pub struct SliceRef<'a, T, U: slice_private::SliceLength = usize> {
    ptr: Option<ConstNonNull<T>>,
    len: U,
    _phantom: PhantomData<&'a [T]>,
}

impl<'a, T, U> SliceRef<'a, T, U>
where
    U: slice_private::SliceLength,
{
    /// Constructs a new `SliceRef`.
    pub const fn new(value: &'a [T]) -> Self
    where
        U: [const] slice_private::SliceLength,
    {
        if value.is_empty() {
            Self {
                ptr: None,
                len: U::from_len(0),
                _phantom: PhantomData,
            }
        } else {
            Self {
                ptr: ConstNonNull::new(value.as_ptr()),
                len: U::from_len(value.len()),
                _phantom: PhantomData,
            }
        }
    }

    /// Constructs a `&[T]` from the `SliceRef`.
    pub const fn as_slice(&self) -> &[T]
    where
        U: [const] slice_private::SliceLength,
    {
        match self.ptr {
            Some(ptr) => {
                let len = self.len.to_len();
                let ptr = ptr.as_ptr();
                unsafe { std::slice::from_raw_parts(ptr, len) }
            }
            None => &[],
        }
    }

    /// Constructs a `&[T]` from the `SliceRef`.
    pub const fn into_slice(self) -> &'a [T]
    where
        U: [const] slice_private::SliceLength,
    {
        match self.ptr {
            Some(ptr) => {
                let len = self.len.to_len();
                let ptr = ptr.as_ptr();
                unsafe { std::slice::from_raw_parts(ptr, len) }
            }
            None => &mut [],
        }
    }
}

unsafe impl<T: Send, U: slice_private::SliceLength + Send> Send for SliceRef<'_, T, U> {}
unsafe impl<T: Sync, U: slice_private::SliceLength + Sync> Sync for SliceRef<'_, T, U> {}
unsafe impl<T: Share, U: slice_private::SliceLength> Share for SliceRef<'static, T, U> {}

impl<T, U: slice_private::SliceLength> Copy for SliceRef<'_, T, U> {}

impl<T, U: slice_private::SliceLength> Clone for SliceRef<'_, T, U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Debug, U: slice_private::SliceLength> Debug for SliceRef<'_, T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'a, T, U: slice_private::SliceLength> From<&'a [T]> for SliceRef<'a, T, U> {
    fn from(value: &'a [T]) -> Self {
        Self::new(value)
    }
}

impl<T, U: slice_private::SliceLength> Deref for SliceRef<'_, T, U> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// An ffi-safe mutable slice reference.
#[repr(C)]
pub struct SliceRefMut<'a, T, U: const slice_private::SliceLength> {
    ptr: Option<NonNull<T>>,
    len: U,
    _phantom: PhantomData<&'a mut [T]>,
}

impl<'a, T, U: const slice_private::SliceLength> SliceRefMut<'a, T, U> {
    /// Constructs a new `SliceRefMut`.
    pub const fn new(value: &'a mut [T]) -> Self {
        if value.is_empty() {
            Self {
                ptr: None,
                len: U::from_len(0),
                _phantom: PhantomData,
            }
        } else {
            Self {
                ptr: NonNull::new(value.as_mut_ptr()),
                len: U::from_len(value.len()),
                _phantom: PhantomData,
            }
        }
    }

    /// Constructs a `&[T]` from the `SliceRefMut`.
    pub const fn as_slice(&self) -> &[T] {
        match self.ptr {
            Some(ptr) => {
                let len = self.len.to_len();
                let ptr = ptr.as_ptr();
                unsafe { std::slice::from_raw_parts(ptr, len) }
            }
            None => &[],
        }
    }

    /// Constructs a `&mut [T]` from the `SliceRefMut`.
    pub const fn as_slice_mut(&mut self) -> &mut [T] {
        match self.ptr {
            Some(ptr) => {
                let len = self.len.to_len();
                let ptr = ptr.as_ptr();
                unsafe { std::slice::from_raw_parts_mut(ptr, len) }
            }
            None => &mut [],
        }
    }

    /// Constructs a `&mut [T]` from the `SliceRefMut`.
    pub const fn into_slice(self) -> &'a mut [T] {
        match self.ptr {
            Some(ptr) => {
                let len = self.len.to_len();
                let ptr = ptr.as_ptr();
                unsafe { std::slice::from_raw_parts_mut(ptr, len) }
            }
            None => &mut [],
        }
    }
}

unsafe impl<T: Send, U: const slice_private::SliceLength + Send> Send for SliceRefMut<'_, T, U> {}
unsafe impl<T: Sync, U: const slice_private::SliceLength + Sync> Sync for SliceRefMut<'_, T, U> {}
unsafe impl<T: Share, U: const slice_private::SliceLength> Share for SliceRefMut<'static, T, U> {}

impl<T: Debug, U: const slice_private::SliceLength> Debug for SliceRefMut<'_, T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'a, T, U: const slice_private::SliceLength> From<&'a mut [T]> for SliceRefMut<'a, T, U> {
    fn from(value: &'a mut [T]) -> Self {
        Self::new(value)
    }
}

impl<T, U: const slice_private::SliceLength> Deref for SliceRefMut<'_, T, U> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, U: const slice_private::SliceLength> DerefMut for SliceRefMut<'_, T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

/// A null-terminated string reference.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct StrRef<'a>(ConstNonNull<u8>, PhantomData<&'a [u8]>);

impl<'a> StrRef<'a> {
    /// Constructs a new `StrRef` from a `CStr`.
    pub const fn new(string: &'a CStr) -> Self {
        unsafe { Self::new_unchecked(ConstNonNull::new_unchecked(string.as_ptr().cast())) }
    }

    /// Constructs a new `ConstCStr` from a raw ptr.
    ///
    /// # Safety
    ///
    /// The string must be null-terminated.
    pub const unsafe fn new_unchecked(string: ConstNonNull<u8>) -> Self {
        Self(string, PhantomData)
    }

    /// Returns the inner pointer to the string.
    pub const fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    /// Returns a reference to the contained `CStr`.
    ///
    /// # Safety
    ///
    /// See [`CStr::from_ptr`].
    pub const unsafe fn as_ref(&self) -> &'a CStr {
        unsafe { CStr::from_ptr(self.as_ptr().cast()) }
    }
}

unsafe impl Send for StrRef<'_> {}
unsafe impl Sync for StrRef<'_> {}
unsafe impl Share for StrRef<'static> {}

impl<'a> From<&'a CStr> for StrRef<'a> {
    fn from(value: &'a CStr) -> Self {
        Self::new(value)
    }
}

impl Default for StrRef<'_> {
    fn default() -> Self {
        Self::new(c"")
    }
}

/// A function pointer.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct FunctionPtr<'a, T>(T, PhantomData<&'a ()>);

unsafe impl<T: Share> Share for FunctionPtr<'static, T> {}

macro_rules! impl_function_ptr_peel {
    () => {};
    ($head:ident $(, $($tail:ident),*)?) => {
        impl_function_ptr_fn!($($($tail),*)?);
    };
}
macro_rules! impl_function_ptr_fn {
    ($($T:ident),*) => {
        impl<Ret, $($T),*> FunctionPtr<'_, extern "C" fn($($T),*) -> Ret> {
            /// Constructs a new `FunctionPtr`.
            #[inline(always)]
            pub const fn new(func: extern "C" fn($($T),*) -> Ret) -> Self {
                Self(func, PhantomData)
            }

            /// Extracts the contained function pointer.
            ///
            /// # Safety
            ///
            /// This method is unsafe, as currently `fn` does not keep track of the lifetime of the
            /// function it points to. This is not a problem in most cases but needs to be tracked
            /// for JIT function to prevent use-after-free errors.
            #[inline(always)]
            pub const unsafe fn into_raw(self) -> extern "C" fn($($T),*) -> Ret {
                self.0
            }

            /// Extracts the address of the function pointer.
            #[inline(always)]
            pub const fn into_addr(self) -> ConstNonNull<()> {
                unsafe {
                    ConstNonNull::new_unchecked(self.0 as *const _)
                }
            }

            /// Constructs a new `FunctionPtr` from an address.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `addr` points to a function of the correct type and
            /// that it outlives the lifetime of the `FunctionPtr` instance.
            #[inline(always)]
            pub const unsafe fn from_addr(addr: ConstNonNull<()>) -> Self {
                unsafe {
                    let func: extern "C" fn($($T),*) -> Ret = std::mem::transmute(addr.as_ptr());
                    Self(func, PhantomData)
                }
            }

            /// Invokes the function with the provided arguments.
            #[inline(always)]
            #[allow(non_snake_case, clippy::too_many_arguments)]
            pub fn call(self, $($T: $T),*) -> Ret {
                (self.0)($($T),*)
            }
        }

        impl<Ret, $($T),*> FunctionPtr<'_, unsafe extern "C" fn($($T),*) -> Ret> {
            /// Constructs a new `FunctionPtr`.
            #[inline(always)]
            pub const fn new(func: unsafe extern "C" fn($($T),*) -> Ret) -> Self {
                Self(func, PhantomData)
            }

            /// Extracts the contained function pointer.
            ///
            /// # Safety
            ///
            /// This method is unsafe, as currently `fn` does not keep track of the lifetime of the
            /// function it points to. This is not a problem in most cases but needs to be tracked
            /// for JIT function to prevent use-after-free errors.
            #[inline(always)]
            pub const unsafe fn into_raw(self) -> unsafe extern "C" fn($($T),*) -> Ret {
                self.0
            }

            /// Extracts the address of the function pointer.
            #[inline(always)]
            pub const fn into_addr(self) -> ConstNonNull<()> {
                unsafe {
                    ConstNonNull::new_unchecked(self.0 as *const _)
                }
            }

            /// Constructs a new `FunctionPtr` from an address.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `addr` points to a function of the correct type and
            /// that it outlives the lifetime of the `FunctionPtr` instance.
            #[inline(always)]
            pub const unsafe fn from_addr(addr: ConstNonNull<()>) -> Self {
                unsafe {
                    let func: unsafe extern "C" fn($($T),*) -> Ret = std::mem::transmute(addr.as_ptr());
                    Self(func, PhantomData)
                }
            }

            /// Invokes the function with the provided arguments.
            #[inline(always)]
            #[allow(non_snake_case, clippy::too_many_arguments, clippy::missing_safety_doc)]
            pub unsafe fn call(self, $($T: $T),*) -> Ret {
                unsafe{ (self.0)($($T),*) }
            }
        }

        impl_function_ptr_peel!($($T),*);
    };
}
impl_function_ptr_fn!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

/// Marker trait for all types that can safely be exported as symbols.
///
/// # Safety
///
/// This trait has the same safety requirements like the [`Share`] trait, but additionally function
/// pointers where the parameters and the return types implement [`Share`].
pub unsafe trait SymbolSafe: Send + Sync {}

unsafe impl<T> SymbolSafe for T where T: Share + Send + Sync + 'static {}

/// Marker trait for `*const T` and `fn(...) -> ...`.
#[const_trait]
pub trait SymbolPointer: private::Sealed + 'static {
    /// Type of the pointee.
    type Target<'a>: Copy;

    /// Dereferences the pointer and returns a reference.
    ///
    /// # Safety
    ///
    /// The pointer must be dereferencable.
    unsafe fn target_from_ptr<'a>(ptr: ConstNonNull<()>) -> Self::Target<'a>;

    /// Dereferences the pointer and returns a reference.
    ///
    /// # Safety
    ///
    /// The pointer must be dereferencable.
    unsafe fn target_ref_from_ptr<'a>(ptr: &ConstNonNull<()>) -> &Self::Target<'a>;

    fn ptr_from_target(target: Self::Target<'_>) -> ConstNonNull<()>;
}

mod private {
    pub trait Sealed {}
}

impl<T> private::Sealed for *const T {}
impl<T> const SymbolPointer for *const T
where
    T: SymbolSafe + 'static,
{
    type Target<'a> = &'a T;

    #[inline(always)]
    unsafe fn target_from_ptr<'a>(ptr: ConstNonNull<()>) -> Self::Target<'a> {
        let ptr = ptr.cast::<T>();
        unsafe { &*ptr.as_ptr() }
    }

    #[inline(always)]
    unsafe fn target_ref_from_ptr<'a>(ptr: &ConstNonNull<()>) -> &Self::Target<'a> {
        let ptr = &raw const *ptr;
        let ptr = ptr.cast::<&'a T>();
        unsafe { &*ptr }
    }

    #[inline(always)]
    fn ptr_from_target(target: Self::Target<'_>) -> ConstNonNull<()> {
        unsafe { ConstNonNull::new_unchecked(target).cast() }
    }
}

macro_rules! impl_symbol_ptr_fn_peel {
    () => {};
    ($head:ident $(, $($tail:ident),*)?) => {
        impl_symbol_ptr_fn!($($($tail),*)?);
    };
}
macro_rules! impl_symbol_ptr_fn {
    ($($T:ident),*) => {
        impl<Ret, $($T),*> private::Sealed for extern "C" fn($($T),*) -> Ret
            where
                Self: SymbolSafe
        {}
        impl<Ret, $($T),*> private::Sealed for unsafe extern "C" fn($($T),*) -> Ret
            where
                Self: SymbolSafe
        {}

        impl<Ret, $($T),*> const SymbolPointer for extern "C" fn($($T),*) -> Ret
        where
            Self: SymbolSafe + 'static
        {
            type Target<'a> = FunctionPtr<'a, Self>;

            #[inline(always)]
            unsafe fn target_from_ptr<'a>(ptr: ConstNonNull<()>) -> Self::Target<'a> {
                unsafe { Self::Target::from_addr(ptr) }
            }

            #[inline(always)]
            unsafe fn target_ref_from_ptr<'a>(ptr: &ConstNonNull<()>) -> &Self::Target<'a> {
                let ptr = &raw const *ptr;
                let ptr = ptr.cast::<FunctionPtr<'a, Self>>();
                unsafe { &*ptr }
            }

            #[inline(always)]
            fn ptr_from_target(target: Self::Target<'_>) -> ConstNonNull<()> {
                target.into_addr()
            }
        }
        impl<Ret, $($T),*> const SymbolPointer for unsafe extern "C" fn($($T),*) -> Ret
        where
            Self: SymbolSafe + 'static
        {
            type Target<'a> = FunctionPtr<'a, Self>;

            #[inline(always)]
            unsafe fn target_from_ptr<'a>(ptr: ConstNonNull<()>) -> Self::Target<'a> {
                unsafe { Self::Target::from_addr(ptr) }
            }

            #[inline(always)]
            unsafe fn target_ref_from_ptr<'a>(ptr: &ConstNonNull<()>) -> &Self::Target<'a> {
                let ptr = &raw const *ptr;
                let ptr = ptr.cast::<FunctionPtr<'a, Self>>();
                unsafe { &*ptr }
            }

            #[inline(always)]
            fn ptr_from_target(target: Self::Target<'_>) -> ConstNonNull<()> {
                target.into_addr()
            }
        }

        impl_symbol_ptr_fn_peel!($($T),*);
    };
}
impl_symbol_ptr_fn!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

/// Trait providing info about a symbol.
///
/// # Implementation
///
/// Is implemented automatically by the [`symbol!`](crate::symbol) macro.
pub trait SymbolInfo: Debug + Copy + PartialEq + Eq + PartialOrd + Ord + std::hash::Hash {
    /// Type of the symbol.
    type Type: SymbolPointer;
    /// Name of the symbol.
    const NAME: &'static CStr;
    /// Namespace of the symbol.
    const NAMESPACE: &'static CStr = c"";
    /// Version of the symbol.
    const VERSION: Version<'static>;
}

/// Declares new symbols.
///
/// # Examples
///
/// ```
/// use fimo_std::symbol;
///
/// symbol! {
///     symbol Foo @ Version("0.0.2") = foo: *const f32;
///     symbol Bar @ Version("1.0.0") = "my_mod"::bar: extern "C" fn(i32, i32) -> i32;
/// }
/// ```
#[macro_export]
macro_rules! symbol {
    () => {};
    (
        symbol $sym:ident @ Version($version:literal) = $($ns:literal::)? $name:ident : $ty:ty;
        $($rest:tt)*
    ) => {
        #[doc = core::concat!("Marker type for the `", core::stringify!($name), "` symbol", $(" in the `", $ns, "` namespace",)? ".")]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $sym {}
        impl $crate::modules::symbols::SymbolInfo for $sym {
            type Type = $ty;
            const NAME: &'static core::ffi::CStr = {
                let string = core::concat!(core::stringify!($name), '\0');
                match core::ffi::CStr::from_bytes_with_nul(string.as_bytes()) {
                    Ok(x) => x,
                    Err(_) => unreachable!(),
                }
            };
            const NAMESPACE: &'static core::ffi::CStr = {
                let string = core::concat!($($ns,)? '\0');
                match core::ffi::CStr::from_bytes_with_nul(string.as_bytes()) {
                    Ok(x) => x,
                    Err(_) => unreachable!(),
                }
            };
            const VERSION: $crate::version::Version<'static> = $crate::version::version!($version);
        }

        $crate::symbol!($($rest)*);
    };
}

/// A reference to a symbol.
#[repr(transparent)]
pub struct SymbolRef<'a, T>(ConstNonNull<()>, PhantomData<&'a T::Type>)
where
    T: SymbolInfo,
    T::Type: const SymbolPointer;

impl<'this, T> SymbolRef<'this, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    /// Constructs a new `SymbolRef` from a symbol pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is dereferencable for the lifetime `'a` and points
    /// to a value of the type `T::Type`.
    pub const unsafe fn from_opaque_ptr<'a>(ptr: ConstNonNull<()>) -> SymbolRef<'a, T> {
        SymbolRef(ptr, PhantomData)
    }

    /// Extracts the contained pointer to the symbol.
    pub const fn into_opaque_ptr(self) -> ConstNonNull<()> {
        self.0
    }

    #[inline(always)]
    const fn as_deref_target(&self) -> &<T::Type as SymbolPointer>::Target<'this> {
        unsafe { T::Type::target_ref_from_ptr(&self.0) }
    }
}

unsafe impl<'a, T> Send for SymbolRef<'a, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
    <T::Type as SymbolPointer>::Target<'a>: Send,
{
}

unsafe impl<'a, T> Sync for SymbolRef<'a, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
    <T::Type as SymbolPointer>::Target<'a>: Sync,
{
}

unsafe impl<'a, T> Share for SymbolRef<'a, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
    <T::Type as SymbolPointer>::Target<'a>: Share,
{
}

impl<T> Copy for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<T> Clone for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T> Eq for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
}

impl<T> PartialOrd for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<'a, T> Debug for SymbolRef<'a, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
    <T::Type as SymbolPointer>::Target<'a>: Debug,
{
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SymbolRef").field(&**self).finish()
    }
}

impl<'a, T> Pointer for SymbolRef<'a, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
    <T::Type as SymbolPointer>::Target<'a>: Pointer,
{
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(self.as_deref_target(), f)
    }
}

impl<'a, T> Deref for SymbolRef<'a, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    type Target = <T::Type as SymbolPointer>::Target<'a>;

    fn deref(&self) -> &Self::Target {
        self.as_deref_target()
    }
}

/// Helper trait for types that can provide access to some specific symbol.
pub trait SymbolProvider<T>: Sized + Copy
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    fn access<'a>(self) -> SymbolRef<'a, T>
    where
        Self: 'a;
}

impl<T> SymbolProvider<T> for SymbolRef<'_, T>
where
    T: SymbolInfo,
    T::Type: const SymbolPointer,
{
    fn access<'a>(self) -> SymbolRef<'a, T>
    where
        Self: 'a,
    {
        self
    }
}
