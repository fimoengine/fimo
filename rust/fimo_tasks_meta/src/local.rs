// Some of the documentation in this module was copied from the Rust std library, licensed under the
// MIT and Apache 2.0 dual license.

use crate::{bindings, Context};
use fimo_std::error::{to_result_indirect, to_result_indirect_in_place, AnyError};

/// Declare a new task-specific storage key of type [`TssKey`].
#[macro_export]
macro_rules! task_specific {
    // empty (base case for the recursion)
    () => {};

    // process multiple declarations
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => (
        $crate::task_specific_inner!($(#[$attr])* $vis $name, $t, $init);
        $crate::task_specific!($($rest)*);
    );

    // handle a single declaration
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => (
        $crate::task_specific_inner!($(#[$attr])* $vis $name, $t, $init);
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! task_specific_inner {
    ($(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $init:expr) => (
        $(#[$attr])*
        $vis static $name: $crate::TssKey<$t> = {
            fn init() -> std::boxed::Box<$t> {
                std::boxed::Box::new($init)
            }

            $crate::TssKey::new_private(init)
        };
    );
}

/// A task-specific storage key which owns its contents.
///
/// It is instantiated with the [`task_specific`] macro and the primary method is the
/// [`with`](TssKey::with) method, though there are helpers to make working with
/// [`Cell`](std::cell::Cell) types easier.
///
/// The [`with`](TssKey::with) method yields a reference to the contained value which cannot outlive
/// the current task or escape the given closure.
///
/// # Initialization and Destruction
///
/// Initialization is dynamically performed on the first call to a setter (e.g.
/// [`with`](TssKey::with)) within a task, and values that implement [`Drop`] get destructed when a
/// task exits.
///
/// A `TssKey`'s initializer cannot recursively depend on itself. Using a `TssKey` in this way may
/// cause panics, aborts or infinite recursion on the first call to [`with`](TssKey::with).
///
/// # Single-thread Synchronization
///
/// Though there is no potential race with other threads, it is still possible to obtain multiple
/// references to the task-specific data in different places on the call stack. For this reason,
/// only shared (`&T`) references may be obtained.
///
/// To allow obtaining an exclusive mutable reference (`&mut T`), typically a
/// [`Cell`](std::cell::Cell) or [`RefCell`](std::cell::RefCell) is used (see [`std::cell`] for more
/// information on how exactly this works). To make this easier there are specialized
/// implementations for `TssKey<Cell<T>>` and `TssKey<RefCell<T>>`.
#[derive(Debug)]
pub struct TssKey<T: 'static> {
    init: fn() -> Box<T>,
}

impl<T: 'static> TssKey<T> {
    #[doc(hidden)]
    pub const fn new_private(init: fn() -> Box<T>) -> Self {
        Self { init }
    }

    /// Acquires a reference to the value in this TSS key.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn with<R>(&'static self, ctx: &Context, f: impl FnOnce(&T) -> R) -> R {
        self.with_inner(ctx, f, self.init)
    }

    /// Acquires a reference to the value in this TSS key.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet. If this
    /// function is unable to access or initialize the key, this function returns an error.
    ///
    /// # Panics
    ///
    /// This function will still `panic!()` if the key is uninitialized and the key’s initializer
    /// panics.
    pub fn try_with<R>(&'static self, ctx: &Context, f: impl FnOnce(&T) -> R) -> Result<R, AnyError> {
        self.try_with_inner(ctx, f, self.init)
    }

    fn with_inner<R>(
        &'static self,
        ctx: &Context,
        f: impl FnOnce(&T) -> R,
        init: impl FnOnce() -> Box<T>,
    ) -> R {
        self.try_with_inner(ctx, f, init).expect(
            "cannot access a TssKey value during construction \
                    or destruction of the task, or from outside of a task",
        )
    }

    fn try_with_inner<R>(
        &'static self,
        ctx: &Context,
        f: impl FnOnce(&T) -> R,
        init: impl FnOnce() -> Box<T>,
    ) -> Result<R, AnyError> {
        // `self` is static so it is guaranteed to be unique and outlive the task.
        let tss_key = std::ptr::from_ref(self).cast::<bindings::FiTasksTssKey_>();

        // Safety: FFI call is safe
        let tss_value = unsafe {
            to_result_indirect_in_place(|err, tss_value| {
                *err = ctx.vtable().v0.tss_get.unwrap_unchecked()(
                    ctx.data(),
                    tss_key,
                    tss_value.as_mut_ptr(),
                );
            })?
        };

        let tss_value = if tss_value.is_null() {
            let mut v = init();
            extern "C" fn drop_box<T>(ptr: *mut std::ffi::c_void) {
                fimo_std::panic::abort_on_panic(|| {
                    let ptr = ptr.cast::<T>();
                    assert!(!ptr.is_null());

                    // Safety: The value was allocated through a box.
                    unsafe {
                        drop(Box::from_raw(ptr));
                    }
                });
            }

            // Safety: FFI call is safe
            unsafe {
                to_result_indirect(|err| {
                    *err = ctx.vtable().v0.tss_set.unwrap_unchecked()(
                        ctx.data(),
                        tss_key,
                        std::ptr::from_mut(&mut *v).cast(),
                        Some(drop_box::<T>),
                    );
                })?;
            }

            Box::into_raw(v).cast_const()
        } else {
            tss_value.cast::<T>().cast_const()
        };

        // Safety: The value is not aliased mutably and is initialized.
        unsafe { Ok(f(&*tss_value)) }
    }
}

impl<T: 'static> TssKey<std::cell::Cell<T>> {
    /// Sets or initializes the contained value.
    ///
    /// Unlike the other methods, this will not run the lazy initializer of the task specific.
    /// Instead, it will be directly initialized with the given value if it wasn’t initialized yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn set(&'static self, ctx: &Context, value: T) {
        self.with_inner(ctx, |_| {}, move || Box::new(std::cell::Cell::new(value)));
    }

    /// Returns a copy of the contained value.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn get(&'static self, ctx: &Context) -> T
    where
        T: Copy,
    {
        self.with(ctx, |x| x.get())
    }

    /// Takes the contained value, leaving [`Default::default`] in its place.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn take(&'static self, ctx: &Context) -> T
    where
        T: Default,
    {
        self.with(ctx, |x| x.take())
    }

    /// Replaces the contained value, returning the old value.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn replace(&'static self, ctx: &Context, value: T) -> T {
        self.with(ctx, |x| x.replace(value))
    }
}

impl<T: 'static> TssKey<std::cell::RefCell<T>> {
    /// Acquires a reference to the contained value.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn with_borrow<R>(&'static self, ctx: &Context, f: impl FnOnce(&T) -> R) -> R {
        self.with(ctx, move |x| f(&*x.borrow()))
    }

    /// Acquires a mutable reference to the contained value.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn with_borrow_mut<R>(&'static self, ctx: &Context, f: impl FnOnce(&mut T) -> R) -> R {
        self.with(ctx, move |x| f(&mut *x.borrow_mut()))
    }

    /// Sets or initializes the contained value.
    ///
    /// Unlike the other methods, this will not run the lazy initializer of the task specific.
    /// Instead, it will be directly initialized with the given value if it wasn’t initialized yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn set(&'static self, ctx: &Context, value: T) {
        self.with_inner(
            ctx,
            |_| {},
            move || Box::new(std::cell::RefCell::new(value)),
        );
    }

    /// Takes the contained value, leaving [`Default::default`] in its place.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn take(&'static self, ctx: &Context) -> T
    where
        T: Default,
    {
        self.with(ctx, |x| x.take())
    }

    /// Replaces the contained value, returning the old value.
    ///
    /// This will lazily initialize the value if this task has not referenced this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// This function will `panic!()` if the key currently has its destructor running, it was called
    /// from outside a task, or the task is being initialized or destroyed.
    pub fn replace(&'static self, ctx: &Context, value: T) -> T {
        self.with(ctx, |x| x.replace(value))
    }
}
