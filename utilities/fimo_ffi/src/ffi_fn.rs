//! FFI safe closure implementations.

use crate::tuple::ReprRust;
use crate::ObjBox;
use std::alloc::{Allocator, Global};
use std::fmt::{Debug, Formatter};
use std::marker::{PhantomData, Unsize};
use std::mem::{forget, MaybeUninit};

#[repr(C)]
struct RawFfiFnInner<T: ?Sized, Meta> {
    ptr: *mut (),
    metadata: MaybeUninit<Meta>,
    drop: unsafe extern "C" fn(*mut (), Meta),
    call_once: unsafe extern "C" fn(*mut (), Meta, *const (), *mut ()),
    call_mut: Option<unsafe extern "C" fn(*mut (), *mut Meta, *const (), *mut ())>,
    call: Option<unsafe extern "C" fn(*mut (), *const Meta, *const (), *mut ())>,
    _marker: PhantomData<T>,
}

impl<T: ?Sized, Meta> RawFfiFnInner<T, Meta> {
    #[inline]
    unsafe fn new(
        ptr: *mut (),
        metadata: Meta,
        drop: unsafe extern "C" fn(*mut (), Meta),
        call_once: unsafe extern "C" fn(*mut (), Meta, *const (), *mut ()),
        call_mut: Option<unsafe extern "C" fn(*mut (), *mut Meta, *const (), *mut ())>,
        call: Option<unsafe extern "C" fn(*mut (), *const Meta, *const (), *mut ())>,
    ) -> Self {
        Self {
            ptr,
            drop,
            call_once,
            call_mut,
            call,
            metadata: MaybeUninit::new(metadata),
            _marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn call_once(self, args: *const (), output: *mut ()) {
        (self.call_once)(self.ptr, self.metadata.assume_init_read(), args, output);
        forget(self)
    }

    #[inline]
    unsafe fn call_mut(&mut self, args: *const (), output: *mut ()) {
        (self.call_mut.unwrap_unchecked())(self.ptr, self.metadata.as_mut_ptr(), args, output)
    }

    #[inline]
    unsafe fn call(&self, args: *const (), output: *mut ()) {
        (self.call.unwrap_unchecked())(self.ptr, self.metadata.as_ptr(), args, output)
    }
}

unsafe impl<T: Send + ?Sized, Meta: Send> Send for RawFfiFnInner<T, Meta> {}

unsafe impl<T: Sync + ?Sized, Meta: Sync> Sync for RawFfiFnInner<T, Meta> {}

impl<T: ?Sized, Meta> Drop for RawFfiFnInner<T, Meta> {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.drop)(self.ptr as *mut _, self.metadata.assume_init_read()) }
    }
}

impl<T: ?Sized, Meta> Debug for RawFfiFnInner<T, Meta> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawFfiFnInner")
            .field("ptr", &self.ptr)
            .field("drop", &self.drop)
            .field("call", &self.call)
            .finish()
    }
}

/// A ffi-safe closure without any lifetime.
#[repr(transparent)]
pub struct RawFfiFn<T: ?Sized, Meta = Global> {
    inner: RawFfiFnInner<T, Meta>,
}

impl<T: ?Sized, Meta> RawFfiFn<T, Meta> {
    /// Constructs a new `RawFfiFn` with the provided [`Box`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let f: RawFfiFn<dyn FnOnce(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f.call_once((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub fn r#box<F: FnOnce<Args, Output = Output> + Unsize<T>, Args: ReprRust, Output>(
        f: Box<F, Meta>,
    ) -> Self
    where
        Meta: Allocator,
    {
        let (f, alloc) = Box::into_raw_with_allocator(f);

        // optimization: use call_mut and call directly without going through the Box impl
        let info = <(Box<F, Meta>, Args, Output) as GetFnTraits<Meta>>::get();
        let ref_info = <(F, Args, Output) as GetFnTraits<Meta>>::get();

        unsafe {
            Self {
                inner: RawFfiFnInner::new(
                    f as *mut (),
                    alloc,
                    info.drop,
                    info.call_once,
                    ref_info.call_mut,
                    ref_info.call,
                ),
            }
        }
    }

    /// Constructs a new `RawFfiFn` with the provided [`ObjBox`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use fimo_ffi::ObjBox;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = ObjBox::new(|num| n.replace(n.get() + num));
    /// let f: RawFfiFn<dyn FnOnce(usize) -> usize> = RawFfiFn::obj_box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f.call_once((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub fn obj_box<F: FnOnce<Args, Output = Output> + Unsize<T>, Args: ReprRust, Output>(
        f: ObjBox<F, Meta>,
    ) -> Self
    where
        Meta: Allocator,
    {
        let (f, alloc) = ObjBox::into_raw_parts(f);

        // optimization: use call_mut and call directly without going through the ObjBox impl
        let info = <(ObjBox<F, Meta>, Args, Output) as GetFnTraits<Meta>>::get();
        let ref_info = <(F, Args, Output) as GetFnTraits<Meta>>::get();

        unsafe {
            Self {
                inner: RawFfiFnInner::new(
                    f as *mut (),
                    alloc,
                    info.drop,
                    info.call_once,
                    ref_info.call_mut,
                    ref_info.call,
                ),
            }
        }
    }

    /// Constructs a new `RawFfiFn` with the provided mutable closure reference and metadata.
    ///
    /// The ownership of the closure is logically passed to the `RawFfiFn`.
    ///
    /// # Safety
    ///
    /// The closure must be initialize and the reference must outlive the `RawFfiFn`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::mem::MaybeUninit;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let mut f = MaybeUninit::new(|num| n.replace(n.get() + num));
    ///
    /// unsafe {
    ///     let mut f: RawFfiFn<dyn FnMut(usize) -> usize, ()> = RawFfiFn::new_value_in(&mut f, ());
    ///     assert_eq!(f.call_mut((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn new_value_in<
        F: FnOnce<Args, Output = Output> + Unsize<T>,
        Args: ReprRust,
        Output,
    >(
        f: &mut MaybeUninit<F>,
        m: Meta,
    ) -> Self {
        let info = <(F, Args, Output) as GetFnTraits<Meta>>::get();
        Self {
            inner: RawFfiFnInner::new(
                f.as_mut_ptr() as *mut _,
                m,
                info.drop,
                info.call_once,
                info.call_mut,
                info.call,
            ),
        }
    }

    /// Calls the closure and consumes it.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped closure
    /// has not been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let f: RawFfiFn<dyn FnOnce(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f.call_once((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe extern "rust-call" fn call_once<Args: ReprRust, O>(self, args: Args) -> O
    where
        T: FnOnce<Args, Output = O>,
    {
        let mut res = MaybeUninit::uninit();
        let args = MaybeUninit::new(args.into_c());
        self.inner
            .call_once(args.as_ptr() as *const _, res.as_mut_ptr() as *mut _);
        res.assume_init()
    }

    /// Calls the closure by mutable reference.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped closure
    /// has not been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let mut f: RawFfiFn<dyn FnMut(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f.call_mut((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    ///
    /// Can only be called if the wrapped closure supports it.
    ///
    /// ```compile_fail
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(0);
    /// let f = Box::new(|num| n.set(n.get() + num));
    /// let mut f: RawFfiFn<dyn FnOnce(usize)> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     f.call_mut((5,));
    /// }
    ///
    /// assert_eq!(n.get(), 5);
    /// ```
    #[inline]
    pub unsafe extern "rust-call" fn call_mut<Args: ReprRust, O>(&mut self, args: Args) -> O
    where
        T: FnMut<Args, Output = O>,
    {
        let mut res = MaybeUninit::uninit();
        let args = MaybeUninit::new(args.into_c());
        self.inner
            .call_mut(args.as_ptr() as *const _, res.as_mut_ptr() as *mut _);
        res.assume_init()
    }

    /// Calls the closure by reference.
    ///
    /// # Safety
    ///
    /// This function is unsafe, because one must ensure, that the wrapped closure
    /// has not been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let f: RawFfiFn<dyn Fn(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f.call((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    ///
    /// Can only be called if the wrapped closure supports it.
    ///
    /// ```compile_fail
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(0);
    /// let f = Box::new(|num| n.set(n.get() + num));
    /// let mut f: RawFfiFn<dyn FnMut(usize)> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     f.call((5,));
    /// }
    ///
    /// assert_eq!(n.get(), 5);
    /// ```
    #[inline]
    pub unsafe extern "rust-call" fn call<Args: ReprRust, O>(&self, args: Args) -> O
    where
        T: Fn<Args, Output = O>,
    {
        let mut res = MaybeUninit::uninit();
        let args = MaybeUninit::new(args.into_c());
        self.inner
            .call(args.as_ptr() as *const _, res.as_mut_ptr() as *mut _);
        res.assume_init()
    }

    /// Assert that the contained closure is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the contained closure hasn't actually been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let f: RawFfiFn<dyn FnOnce(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     // we took ownership of the box so it is always valid.
    ///     let f = f.assume_valid();
    ///     assert_eq!(f(5), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn assume_valid<'a>(self) -> FfiFn<'a, T, Meta> {
        FfiFn {
            raw: self,
            _phantom: Default::default(),
        }
    }

    /// Assert that the contained closure is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the contained closure hasn't actually been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let mut f: RawFfiFn<dyn FnMut(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     // we took ownership of the box so it is always valid.
    ///     let f = f.assume_valid_ref_mut();
    ///     assert_eq!(f(5), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn assume_valid_ref_mut(&mut self) -> &mut FfiFn<'_, T, Meta> {
        &mut *(self as *mut _ as *mut _)
    }

    /// Assert that the contained closure is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that the contained closure hasn't actually been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let f = Box::new(|num| n.replace(n.get() + num));
    /// let f: RawFfiFn<dyn Fn(usize) -> usize> = RawFfiFn::r#box(f);
    ///
    /// unsafe {
    ///     // we took ownership of the box so it is always valid.
    ///     let f = f.assume_valid_ref();
    ///     assert_eq!(f(5), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn assume_valid_ref(&self) -> &FfiFn<'_, T, Meta> {
        &*(self as *const _ as *const _)
    }
}

impl<T: ?Sized, Meta: Default> RawFfiFn<T, Meta> {
    /// Constructs a new `RawFfiFn` with the provided mutable closure reference.
    ///
    /// The ownership of the closure is logically passed to the `RawFfiFn`.
    ///
    /// # Safety
    ///
    /// The closure must be initialize and the reference must outlive the `RawFfiFn`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::RawFfiFn;
    /// use std::mem::MaybeUninit;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let mut f = MaybeUninit::new(|num| n.replace(n.get() + num));
    ///
    /// unsafe {
    ///     let mut f: RawFfiFn<dyn FnMut(usize) -> usize> = RawFfiFn::new_value(&mut f);
    ///     assert_eq!(f.call_mut((5,)), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn new_value<
        F: FnOnce<Args, Output = Output> + Unsize<T>,
        Args: ReprRust,
        Output,
    >(
        f: &mut MaybeUninit<F>,
    ) -> Self {
        Self::new_value_in(f, Default::default())
    }
}

impl<'a, T: ?Sized + 'a, Meta: 'a> Debug for RawFfiFn<T, Meta> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawFfiFn")
            .field("raw", &self.inner)
            .finish()
    }
}

/// A safe alternative to an [`RawFfiFn`].
#[repr(transparent)]
pub struct FfiFn<'a, T: ?Sized, Meta = Global> {
    raw: RawFfiFn<T, Meta>,
    _phantom: PhantomData<&'a mut T>,
}

impl<'a, T: ?Sized, Meta> FfiFn<'a, T, Meta> {
    /// Constructs a new `FfiFn` with the provided [`Box`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::FfiFn;
    /// use std::cell::Cell;
    /// use std::sync::Arc;
    ///
    /// let n = Arc::new(Cell::new(5));
    /// let n2 = n.clone();
    ///
    /// let f = Box::new(move |num| n2.replace(n2.get() + num));
    /// let f: FfiFn<'static, dyn Fn(usize) -> usize> = FfiFn::r#box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f(5), 5);
    ///     assert_eq!(f(5), 10);
    /// }
    ///
    /// assert_eq!(n.get(), 15);
    /// ```
    #[inline]
    pub fn r#box<F: FnOnce<Args, Output = Output> + Unsize<T>, Args: ReprRust, Output>(
        f: Box<F, Meta>,
    ) -> FfiFn<'static, T, Meta>
    where
        Meta: Allocator,
    {
        FfiFn {
            raw: RawFfiFn::r#box(f),
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `FfiFn` with the provided [`ObjBox`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::FfiFn;
    /// use fimo_ffi::ObjBox;
    /// use std::cell::Cell;
    /// use std::sync::Arc;
    ///
    /// let n = Arc::new(Cell::new(5));
    /// let n2 = n.clone();
    ///
    /// let f = ObjBox::new(move |num| n2.replace(n2.get() + num));
    /// let f: FfiFn<'static, dyn Fn(usize) -> usize> = FfiFn::obj_box(f);
    ///
    /// unsafe {
    ///     assert_eq!(f(5), 5);
    ///     assert_eq!(f(5), 10);
    /// }
    ///
    /// assert_eq!(n.get(), 15);
    /// ```
    #[inline]
    pub fn obj_box<F: FnOnce<Args, Output = Output> + Unsize<T>, Args: ReprRust, Output>(
        f: ObjBox<F, Meta>,
    ) -> FfiFn<'static, T, Meta>
    where
        Meta: Allocator,
    {
        FfiFn {
            raw: RawFfiFn::obj_box(f),
            _phantom: Default::default(),
        }
    }

    /// Constructs a new `FfiFn` with the provided mutable closure reference and metadata.
    ///
    /// The ownership of the closure is logically passed to the `FfiFn`.
    ///
    /// # Safety
    ///
    /// The closure must be initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::FfiFn;
    /// use std::mem::MaybeUninit;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let mut f = MaybeUninit::new(|num| n.replace(n.get() + num));
    ///
    /// unsafe {
    ///     let mut f: FfiFn<dyn FnMut(usize) -> usize, ()> = FfiFn::new_value_in(&mut f, ());
    ///     assert_eq!(f(5), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn new_value_in<
        F: FnOnce<Args, Output = Output> + Unsize<T>,
        Args: ReprRust,
        Output,
    >(
        f: &'a mut MaybeUninit<F>,
        m: Meta,
    ) -> Self {
        Self {
            raw: RawFfiFn::new_value_in(f, m),
            _phantom: Default::default(),
        }
    }
}

impl<'a, T: ?Sized, Meta: Default> FfiFn<'a, T, Meta> {
    /// Constructs a new `FfiFn` with the provided mutable closure reference.
    ///
    /// The ownership of the closure is logically passed to the `FfiFn`.
    ///
    /// # Safety
    ///
    /// The closure must be initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::ffi_fn::FfiFn;
    /// use std::mem::MaybeUninit;
    /// use std::cell::Cell;
    ///
    /// let mut n = Cell::new(5);
    /// let mut f = MaybeUninit::new(|num| n.replace(n.get() + num));
    ///
    /// unsafe {
    ///     let mut f: FfiFn<dyn FnMut(usize) -> usize> = FfiFn::new_value(&mut f);
    ///     assert_eq!(f(5), 5);
    /// }
    ///
    /// assert_eq!(n.get(), 10);
    /// ```
    #[inline]
    pub unsafe fn new_value<
        F: FnOnce<Args, Output = Output> + Unsize<T>,
        Args: ReprRust,
        Output,
    >(
        f: &'a mut MaybeUninit<F>,
    ) -> Self {
        Self::new_value_in(f, Default::default())
    }

    /// Extracts the contained [`RawFfiFn`].
    #[inline]
    pub fn into_raw(self) -> RawFfiFn<T, Meta> {
        self.raw
    }
}

impl<'a, T: ?Sized + 'a, Meta: 'a> Debug for FfiFn<'a, T, Meta> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiFn").field("raw", &self.raw).finish()
    }
}

impl<T: FnOnce<Args, Output = Output> + ?Sized, Args: ReprRust, Output, Meta> FnOnce<Args>
    for FfiFn<'_, T, Meta>
{
    type Output = Output;

    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        unsafe { self.raw.call_once(args) }
    }
}

impl<T: FnMut<Args, Output = Output> + ?Sized, Args: ReprRust, Output, Meta> FnMut<Args>
    for FfiFn<'_, T, Meta>
{
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        unsafe { self.raw.call_mut(args) }
    }
}

impl<T: Fn<Args, Output = Output> + ?Sized, Args: ReprRust, Output, Meta> Fn<Args>
    for FfiFn<'_, T, Meta>
{
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        unsafe { self.raw.call(args) }
    }
}

struct FnInfo<Meta> {
    pub call_once: unsafe extern "C" fn(*mut (), Meta, *const (), *mut ()),
    pub call_mut: Option<unsafe extern "C" fn(*mut (), *mut Meta, *const (), *mut ())>,
    pub call: Option<unsafe extern "C" fn(*mut (), *const Meta, *const (), *mut ())>,
    pub drop: unsafe extern "C" fn(*mut (), Meta),
}

trait GetFnTraits<Meta> {
    fn get() -> FnInfo<Meta>;
}

impl<T: FnOnce<Args, Output = O>, Args: ReprRust, O, Meta> GetFnTraits<Meta> for (T, Args, O) {
    default fn get() -> FnInfo<Meta> {
        FnInfo {
            call_once: <T as FnTraits<Meta>>::call_once::<Args, O>,
            call_mut: None,
            call: None,
            drop: <T as FnTraits<Meta>>::drop,
        }
    }
}

impl<T: FnMut<Args, Output = O>, Args: ReprRust, O, Meta> GetFnTraits<Meta> for (T, Args, O) {
    default fn get() -> FnInfo<Meta> {
        FnInfo {
            call_once: <T as FnTraits<Meta>>::call_once::<Args, O>,
            call_mut: Some(<T as FnTraits<Meta>>::call_mut::<Args, O>),
            call: None,
            drop: <T as FnTraits<Meta>>::drop,
        }
    }
}

impl<T: Fn<Args, Output = O>, Args: ReprRust, O, Meta> GetFnTraits<Meta> for (T, Args, O) {
    default fn get() -> FnInfo<Meta> {
        FnInfo {
            call_once: <T as FnTraits<Meta>>::call_once::<Args, O>,
            call_mut: Some(<T as FnTraits<Meta>>::call_mut::<Args, O>),
            call: Some(<T as FnTraits<Meta>>::call::<Args, O>),
            drop: <T as FnTraits<Meta>>::drop,
        }
    }
}

trait FnTraits<Meta> {
    unsafe extern "C" fn call_once<Args: ReprRust, Output>(
        f: *mut (),
        meta: Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: FnOnce<Args, Output = Output>;

    unsafe extern "C" fn call_mut<Args: ReprRust, Output>(
        f: *mut (),
        meta: *mut Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: FnMut<Args, Output = Output>;

    unsafe extern "C" fn call<Args: ReprRust, Output>(
        f: *mut (),
        meta: *const Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: Fn<Args, Output = Output>;

    unsafe extern "C" fn drop(f: *mut (), meta: Meta);
}

impl<T, Meta> FnTraits<Meta> for T {
    default unsafe extern "C" fn call_once<Args: ReprRust, Output>(
        f: *mut (),
        _meta: Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: FnOnce<Args, Output = Output>,
    {
        let f = (f as *mut Self).read();
        let args = Args::from_c((args as *const Args::T).read());
        let output = output as *mut Output;
        output.write(<Self as FnOnce<Args>>::call_once(f, args))
    }

    default unsafe extern "C" fn call_mut<Args: ReprRust, Output>(
        f: *mut (),
        _meta: *mut Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: FnMut<Args, Output = Output>,
    {
        let f = &mut *(f as *mut Self);
        let args = Args::from_c((args as *const Args::T).read());
        let output = output as *mut Output;
        output.write(<Self as FnMut<Args>>::call_mut(f, args))
    }

    default unsafe extern "C" fn call<Args: ReprRust, Output>(
        f: *mut (),
        _meta: *const Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: Fn<Args, Output = Output>,
    {
        let f = &*(f as *mut Self);
        let args = Args::from_c((args as *const Args::T).read());
        let output = output as *mut Output;
        output.write(<Self as Fn<Args>>::call(f, args))
    }

    default unsafe extern "C" fn drop(f: *mut (), _meta: Meta) {
        let f = f as *mut Self;
        std::ptr::drop_in_place(f);
    }
}

impl<T, Meta: Allocator> FnTraits<Meta> for Box<T, Meta> {
    #[inline]
    unsafe extern "C" fn call_once<Args: ReprRust, Output>(
        f: *mut (),
        meta: Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: FnOnce<Args, Output = Output>,
    {
        let f = f as *mut T;
        let args = Args::from_c((args as *const Args::T).read());
        let output = output as *mut Output;

        let f = Box::from_raw_in(f, meta);
        output.write(<Self as FnOnce<Args>>::call_once(f, args))
    }

    unsafe extern "C" fn call_mut<Args: ReprRust, Output>(
        _f: *mut (),
        _meta: *mut Meta,
        _args: *const (),
        _output: *mut (),
    ) where
        Self: FnMut<Args, Output = Output>,
    {
        unimplemented!()
    }

    unsafe extern "C" fn call<Args: ReprRust, Output>(
        _f: *mut (),
        _meta: *const Meta,
        _args: *const (),
        _output: *mut (),
    ) where
        Self: Fn<Args, Output = Output>,
    {
        unimplemented!()
    }

    unsafe extern "C" fn drop(f: *mut (), meta: Meta) {
        let f = f as *mut T;
        let b = Box::from_raw_in(f, meta);
        drop(b)
    }
}

impl<T, Meta: Allocator> FnTraits<Meta> for ObjBox<T, Meta> {
    #[inline]
    unsafe extern "C" fn call_once<Args: ReprRust, Output>(
        f: *mut (),
        meta: Meta,
        args: *const (),
        output: *mut (),
    ) where
        Self: FnOnce<Args, Output = Output>,
    {
        let f = f as *mut T;
        let args = Args::from_c((args as *const Args::T).read());
        let output = output as *mut Output;

        let f = ObjBox::from_raw_parts(f, meta);
        output.write(<Self as FnOnce<Args>>::call_once(f, args))
    }

    unsafe extern "C" fn call_mut<Args: ReprRust, Output>(
        _f: *mut (),
        _meta: *mut Meta,
        _args: *const (),
        _output: *mut (),
    ) where
        Self: FnMut<Args, Output = Output>,
    {
        unimplemented!()
    }

    unsafe extern "C" fn call<Args: ReprRust, Output>(
        _f: *mut (),
        _meta: *const Meta,
        _args: *const (),
        _output: *mut (),
    ) where
        Self: Fn<Args, Output = Output>,
    {
        unimplemented!()
    }

    unsafe extern "C" fn drop(f: *mut (), meta: Meta) {
        let f = f as *mut T;
        let b = ObjBox::from_raw_parts(f, meta);
        drop(b)
    }
}
