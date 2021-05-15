use crate::base_api::{DataGuard, Locked, Unlocked};
use emf_core_base_rs::extensions::unwind_internal::UnwindInternalContextRef;
use emf_core_base_rs::ffi::collections::NonNullConst;
use emf_core_base_rs::ffi::extensions::unwind_internal::{Context, PanicFn, ShutdownFn};
use emf_core_base_rs::ffi::{CBaseFn, FnId};
use emf_core_base_rs::sys::sync_handler::{SyncHandler, SyncHandlerAPI};
use parking_lot::RwLock;
use std::cell::Cell;
use std::ffi::{c_void, CStr, CString};
use std::mem::swap;
use std::ptr::NonNull;
use thread_local::ThreadLocal;

mod sync;
mod unwind_context;

/// Exit status of the interface.
#[derive(Debug, Eq, PartialEq)]
pub enum ExitStatus {
    Ok,
    Shutdown,
    Panic(Option<CString>),
    Other,
}

/// Implementation of the sys api.
#[derive(Debug)]
pub struct SysAPI {
    sync_handler: RwLock<SyncHandler<'static>>,
    default_sync: sync::DefaultSync,
    unwind_contexts: ThreadLocal<Cell<Option<UnwindInternalContextRef>>>,
}

impl Default for SysAPI {
    fn default() -> Self {
        Self::new()
    }
}

impl SysAPI {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        let default_sync = sync::DefaultSync::new();

        Self {
            sync_handler: RwLock::new(default_sync.as_interface()),
            default_sync,
            unwind_contexts: ThreadLocal::with_capacity(8),
        }
    }

    /// Terminates the interface.
    #[inline]
    pub fn shutdown(&self) -> ! {
        if let Some(context) = self.unwind_contexts.get() {
            if let Some(context) = context.get() {
                unsafe { (*context._shutdown)(Some(context._context)) }
            }
        }

        panic!("Unable to shutdown. Interface entered improperly.")
    }

    /// Panics the interface.
    #[inline]
    pub fn panic(&self, error: Option<&CStr>) -> ! {
        if let Some(context) = self.unwind_contexts.get() {
            if let Some(context) = context.get() {
                unsafe {
                    (*context._panic)(
                        Some(context._context),
                        error.map(|e| NonNullConst::from(e.to_bytes_with_nul())),
                    )
                }
            }
        }

        panic!("Unable to panic. Interface entered improperly.")
    }

    /// Enters the interface from a new thread.
    #[inline]
    pub fn enter_interface_from_thread(
        &self,
        context: Option<NonNull<c_void>>,
        func: extern "C-unwind" fn(Option<NonNull<c_void>>),
    ) -> ExitStatus {
        // Initialize a default context
        let default_context = self.unwind_contexts.get_or(|| Cell::new(None));
        if default_context.get() != None {
            panic!("Interface entered twice.");
        }

        default_context.set(Some(unwind_context::construct_context()));

        // Disable outputting the error the stderr
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        // Call the function
        let result = std::panic::catch_unwind(move || func(context));

        // Reset
        std::panic::set_hook(default_hook);
        default_context.set(None);

        match result {
            Ok(_) => ExitStatus::Ok,
            Err(err) => {
                if err.is::<unwind_context::ShutdownSignal>() {
                    ExitStatus::Shutdown
                } else if err.is::<unwind_context::PanicSignal>() {
                    let panic_sig = err.downcast_ref::<unwind_context::PanicSignal>().unwrap();
                    ExitStatus::Panic(panic_sig.error.clone())
                } else {
                    ExitStatus::Other
                }
            }
        }
    }

    /// Checks if a function is implemented.
    #[inline]
    pub fn has_fn(&self, id: FnId) -> bool {
        matches!(
            id,
            FnId::SysShutdown
                | FnId::SysPanic
                | FnId::SysHasFunction
                | FnId::SysGetFunction
                | FnId::SysLock
                | FnId::SysTryLock
                | FnId::SysUnlock
                | FnId::SysGetSyncHandler
                | FnId::VersionNewShort
                | FnId::VersionNewLong
                | FnId::VersionNewFull
                | FnId::VersionFromString
                | FnId::VersionStringLengthShort
                | FnId::VersionStringLengthLong
                | FnId::VersionStringLengthFull
                | FnId::VersionAsStringShort
                | FnId::VersionAsStringLong
                | FnId::VersionAsStringFull
                | FnId::VersionStringIsValid
                | FnId::VersionCompare
                | FnId::VersionCompareWeak
                | FnId::VersionCompareStrong
                | FnId::VersionIsCompatible
        )
    }

    /// Fetches a function.
    #[inline]
    pub fn get_fn(&self, id: FnId) -> Option<CBaseFn> {
        use crate::base_interface::{sys_bindings, version_bindings};

        unsafe {
            match id {
                FnId::SysShutdown => Some(std::mem::transmute(
                    sys_bindings::shutdown as unsafe extern "C-unwind" fn(_) -> _,
                )),
                FnId::SysPanic => Some(std::mem::transmute(
                    sys_bindings::panic as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::SysHasFunction => Some(std::mem::transmute(
                    sys_bindings::has_fn as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::SysGetFunction => Some(std::mem::transmute(
                    sys_bindings::get_fn as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::SysLock => Some(std::mem::transmute(
                    sys_bindings::lock as unsafe extern "C-unwind" fn(_) -> _,
                )),
                FnId::SysTryLock => Some(std::mem::transmute(
                    sys_bindings::try_lock as unsafe extern "C-unwind" fn(_) -> _,
                )),
                FnId::SysUnlock => Some(std::mem::transmute(
                    sys_bindings::unlock as unsafe extern "C-unwind" fn(_) -> _,
                )),
                FnId::SysGetSyncHandler => Some(std::mem::transmute(
                    sys_bindings::get_sync_handler as unsafe extern "C-unwind" fn(_) -> _,
                )),
                FnId::SysSetSyncHandler => Some(std::mem::transmute(
                    sys_bindings::set_sync_handler as unsafe extern "C-unwind" fn(_, _) -> _,
                )),

                FnId::VersionNewShort => Some(std::mem::transmute(
                    version_bindings::new_short as unsafe extern "C-unwind" fn(_, _, _, _) -> _,
                )),
                FnId::VersionNewLong => Some(std::mem::transmute(
                    version_bindings::new_long
                        as unsafe extern "C-unwind" fn(_, _, _, _, _, _) -> _,
                )),
                FnId::VersionNewFull => Some(std::mem::transmute(
                    version_bindings::new_full
                        as unsafe extern "C-unwind" fn(_, _, _, _, _, _, _) -> _,
                )),
                FnId::VersionFromString => Some(std::mem::transmute(
                    version_bindings::from_string as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::VersionStringLengthShort => Some(std::mem::transmute(
                    version_bindings::string_length_short as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::VersionStringLengthLong => Some(std::mem::transmute(
                    version_bindings::string_length_long as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::VersionStringLengthFull => Some(std::mem::transmute(
                    version_bindings::string_length_full as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::VersionAsStringShort => Some(std::mem::transmute(
                    version_bindings::as_string_short as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                FnId::VersionAsStringLong => Some(std::mem::transmute(
                    version_bindings::as_string_long as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                FnId::VersionAsStringFull => Some(std::mem::transmute(
                    version_bindings::as_string_full as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                FnId::VersionStringIsValid => Some(std::mem::transmute(
                    version_bindings::string_is_valid as unsafe extern "C-unwind" fn(_, _) -> _,
                )),
                FnId::VersionCompare => Some(std::mem::transmute(
                    version_bindings::compare as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                FnId::VersionCompareWeak => Some(std::mem::transmute(
                    version_bindings::compare_weak as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                FnId::VersionCompareStrong => Some(std::mem::transmute(
                    version_bindings::compare_strong as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                FnId::VersionIsCompatible => Some(std::mem::transmute(
                    version_bindings::is_compatible as unsafe extern "C-unwind" fn(_, _, _) -> _,
                )),
                _ => None,
            }
        }
    }

    /// Locks the interface.
    #[inline]
    pub fn lock(&self) {
        unsafe { self.sync_handler.read().lock() }
    }

    /// Tries to lock the interface.
    #[inline]
    pub fn try_lock(&self) -> bool {
        unsafe { self.sync_handler.read().try_lock() }
    }

    /// Unlocks the interface.
    #[inline]
    pub fn unlock(&self) {
        unsafe { self.sync_handler.read().unlock() }
    }

    /// Fetches the active sync handler.
    #[inline]
    pub fn get_sync_handler(&self) -> SyncHandler<'static> {
        *self.sync_handler.read()
    }

    /// Sets the active sync handler.
    ///
    /// # Safety
    ///
    /// Modifying the sync handler may cause unintended side-effects.
    #[inline]
    pub unsafe fn set_sync_handler(&mut self, s: Option<SyncHandler<'static>>) {
        match s {
            None => {
                let mut new = self.default_sync.as_interface();

                let mut old = self.sync_handler.write();

                if *old != new {
                    new.lock();
                    swap(&mut new, &mut *old);
                    new.unlock();
                }
            }
            Some(mut new) => {
                let mut old = self.sync_handler.write();

                if *old != new {
                    new.lock();
                    swap(&mut new, &mut *old);
                    new.unlock();
                }
            }
        }
    }

    /// Sets a new unwind context.
    ///
    /// # Safety
    ///
    /// Improper usage will lead to undefined behaviour.
    /// This function is intended to always be used in conjunction with
    /// [SysAPI::set_unwind_shutdown] and [SysAPI::set_unwind_panic].
    #[inline]
    pub unsafe fn set_unwind_context(&mut self, context: Option<NonNull<Context>>) {
        if let Some(old) = self.unwind_contexts.get() {
            if let Some(mut new) = old.get() {
                if let Some(context) = context {
                    new._context = context;
                } else {
                    new._context = unwind_context::construct_context()._context;
                }

                return;
            }
        }

        self.panic(Some(
            CStr::from_bytes_with_nul(b"Unable to set context! Interface entered improperly.\0")
                .unwrap(),
        ))
    }

    /// Fetches the current unwind context.
    #[inline]
    pub fn get_unwind_context(&self) -> Option<NonNull<Context>> {
        if let Some(context) = self.unwind_contexts.get() {
            if let Some(context) = context.get() {
                return Some(context._context);
            }
        }

        self.panic(Some(
            CStr::from_bytes_with_nul(b"Unable to get context! Interface entered improperly.\0")
                .unwrap(),
        ))
    }

    /// Sets a new unwind shutdown function.
    ///
    /// # Safety
    ///
    /// Improper usage will lead to undefined behaviour.
    /// This function is intended to always be used in conjunction with
    /// [SysAPI::set_unwind_context] and [SysAPI::set_unwind_panic].
    #[inline]
    pub unsafe fn set_unwind_shutdown(&mut self, shutdown_fn: Option<ShutdownFn>) {
        if let Some(old) = self.unwind_contexts.get() {
            if let Some(mut new) = old.get() {
                if let Some(shutdown) = shutdown_fn {
                    new._shutdown = shutdown;
                } else {
                    new._shutdown = unwind_context::construct_context()._shutdown;
                }

                return;
            }
        }

        self.panic(Some(
            CStr::from_bytes_with_nul(
                b"Unable to set context shutdown! Interface entered improperly.\0",
            )
            .unwrap(),
        ))
    }

    /// Fetches the current unwind shutdown function.
    #[inline]
    pub fn get_unwind_shutdown(&self) -> Option<ShutdownFn> {
        if let Some(context) = self.unwind_contexts.get() {
            if let Some(context) = context.get() {
                return Some(context._shutdown);
            }
        }

        self.panic(Some(
            CStr::from_bytes_with_nul(
                b"Unable to get context shutdown! Interface entered improperly.\0",
            )
            .unwrap(),
        ))
    }

    /// Sets a new unwind panic function.
    ///
    /// # Safety
    ///
    /// Improper usage will lead to undefined behaviour.
    /// This function is intended to always be used in conjunction with
    /// [SysAPI::set_unwind_context] and [SysAPI::set_unwind_shutdown].
    #[inline]
    pub unsafe fn set_unwind_panic(&mut self, panic_fn: Option<PanicFn>) {
        if let Some(old) = self.unwind_contexts.get() {
            if let Some(mut new) = old.get() {
                if let Some(panic) = panic_fn {
                    new._panic = panic;
                } else {
                    new._panic = unwind_context::construct_context()._panic;
                }

                return;
            }
        }

        self.panic(Some(
            CStr::from_bytes_with_nul(
                b"Unable to set context panic! Interface entered improperly.\0",
            )
            .unwrap(),
        ))
    }

    /// Fetches the current unwind shutdown function.
    #[inline]
    pub fn get_unwind_panic(&self) -> Option<PanicFn> {
        if let Some(context) = self.unwind_contexts.get() {
            if let Some(context) = context.get() {
                return Some(context._panic);
            }
        }

        self.panic(Some(
            CStr::from_bytes_with_nul(
                b"Unable to get context panic! Interface entered improperly.\0",
            )
            .unwrap(),
        ))
    }
}

impl<'a> DataGuard<'a, SysAPI, Unlocked> {
    /// Terminates the interface.
    #[inline]
    pub fn shutdown(&self) -> ! {
        self.data.shutdown()
    }

    /// Panics the interface.
    #[inline]
    pub fn panic(&self, error: Option<&CStr>) -> ! {
        self.data.panic(error)
    }

    /// Enters the interface from a new thread.
    #[inline]
    pub fn enter_interface_from_thread(
        &self,
        context: Option<NonNull<c_void>>,
        func: extern "C-unwind" fn(Option<NonNull<c_void>>),
    ) -> ExitStatus {
        self.data.enter_interface_from_thread(context, func)
    }

    /// Locks the interface.
    #[inline]
    pub fn lock(self) -> DataGuard<'a, SysAPI, Locked> {
        self.data.lock();
        unsafe { self.assume_locked() }
    }

    /// Tries to lock the interface.
    #[inline]
    pub fn try_lock(self) -> Result<DataGuard<'a, SysAPI, Locked>, Self> {
        if self.data.try_lock() {
            Ok(unsafe { self.assume_locked() })
        } else {
            Err(self)
        }
    }
}

impl<'a> DataGuard<'a, SysAPI, Locked> {
    /// Terminates the interface.
    #[inline]
    pub fn shutdown(&self) -> ! {
        self.data.shutdown()
    }

    /// Panics the interface.
    #[inline]
    pub fn panic(&self, error: Option<&CStr>) -> ! {
        self.data.panic(error)
    }

    /// Unlocks the interface.
    #[inline]
    pub fn unlock(self) -> DataGuard<'a, SysAPI, Unlocked> {
        self.data.unlock();
        unsafe { self.assume_unlocked() }
    }

    /// Checks if a function is implemented.
    #[inline]
    pub fn has_fn(&self, id: FnId) -> bool {
        self.data.has_fn(id)
    }

    /// Fetches a function.
    #[inline]
    pub fn get_fn(&self, id: FnId) -> Option<CBaseFn> {
        self.data.get_fn(id)
    }

    /// Fetches the active sync handler.
    #[inline]
    pub fn get_sync_handler(&self) -> SyncHandler<'a> {
        self.data.get_sync_handler()
    }

    /// Sets the active sync handler.
    ///
    /// # Safety
    ///
    /// Modifying the sync handler may cause unintended side-effects.
    #[inline]
    pub unsafe fn set_sync_handler(&mut self, s: Option<SyncHandler<'static>>) {
        self.data.set_sync_handler(s)
    }

    /// Sets a new unwind context.
    ///
    /// # Safety
    ///
    /// Improper usage will lead to undefined behaviour.
    /// This function is intended to always be used in conjunction with
    /// [Self::set_unwind_shutdown] and [Self::set_unwind_panic].
    #[inline]
    pub unsafe fn set_unwind_context(&mut self, context: Option<NonNull<Context>>) {
        self.data.set_unwind_context(context)
    }

    /// Fetches the current unwind context.
    #[inline]
    pub fn get_unwind_context(&self) -> Option<NonNull<Context>> {
        self.data.get_unwind_context()
    }

    /// Sets a new unwind shutdown function.
    ///
    /// # Safety
    ///
    /// Improper usage will lead to undefined behaviour.
    /// This function is intended to always be used in conjunction with
    /// [Self::set_unwind_context] and [Self::set_unwind_panic].
    #[inline]
    pub unsafe fn set_unwind_shutdown(&mut self, shutdown_fn: Option<ShutdownFn>) {
        self.data.set_unwind_shutdown(shutdown_fn)
    }

    /// Fetches the current unwind shutdown function.
    #[inline]
    pub fn get_unwind_shutdown(&self) -> Option<ShutdownFn> {
        self.data.get_unwind_shutdown()
    }

    /// Sets a new unwind panic function.
    ///
    /// # Safety
    ///
    /// Improper usage will lead to undefined behaviour.
    /// This function is intended to always be used in conjunction with
    /// [Self::set_unwind_context] and [Self::set_unwind_shutdown].
    #[inline]
    pub unsafe fn set_unwind_panic(&mut self, panic_fn: Option<PanicFn>) {
        self.data.set_unwind_panic(panic_fn)
    }

    /// Fetches the current unwind shutdown function.
    #[inline]
    pub fn get_unwind_panic(&self) -> Option<PanicFn> {
        self.data.get_unwind_panic()
    }
}

#[cfg(test)]
mod tests {
    use crate::base_api::sys::ExitStatus;
    use crate::base_api::SysAPI;
    use std::cell::Cell;
    use std::ffi::{c_void, CString};
    use std::marker::PhantomData;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};

    struct Callback<F> {
        pub context: NonNull<c_void>,
        pub callback: extern "C-unwind" fn(Option<NonNull<c_void>>),
        _phantom: PhantomData<F>,
    }

    impl<F> Drop for Callback<F> {
        fn drop(&mut self) {
            drop(unsafe { Box::<F>::from_raw(self.context.cast().as_ptr()) });
        }
    }

    impl<F> Callback<F>
    where
        F: FnOnce(),
    {
        pub fn new(f: F) -> Self {
            Self {
                context: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(f))) }.cast(),
                callback: Self::callback,
                _phantom: PhantomData,
            }
        }

        pub unsafe fn take(
            self,
        ) -> (
            NonNull<c_void>,
            extern "C-unwind" fn(Option<NonNull<c_void>>),
        ) {
            let res = (self.context, self.callback);
            std::mem::forget(self);
            res
        }

        extern "C-unwind" fn callback(context: Option<NonNull<c_void>>) {
            let f: Box<F> = unsafe { Box::from_raw(context.unwrap().cast().as_ptr()) };
            f();
        }
    }

    #[test]
    fn normal_exit() {
        let sys = Arc::new(SysAPI::new());

        let callback = { Callback::new(move || {}) };

        let (context, callback) = unsafe { callback.take() };
        let result = sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(result, ExitStatus::Ok);
    }

    #[test]
    fn abnormal_error() {
        let sys = Arc::new(SysAPI::new());

        let callback = { Callback::new(move || panic!("Hey!")) };

        let (context, callback) = unsafe { callback.take() };
        let result = sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(result, ExitStatus::Other);
    }

    #[test]
    fn shutdown() {
        let sys = Arc::new(SysAPI::new());

        let callback = {
            let sys_c = Arc::clone(&sys);
            Callback::new(move || sys_c.shutdown())
        };

        let (context, callback) = unsafe { callback.take() };
        let result = sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(result, ExitStatus::Shutdown);
    }

    #[test]
    fn panic() {
        let sys = Arc::new(SysAPI::new());

        let callback = {
            let sys_c = Arc::clone(&sys);
            Callback::new(move || sys_c.panic(None))
        };

        let (context, callback) = unsafe { callback.take() };
        let result = sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(result, ExitStatus::Panic(None));
    }

    #[test]
    fn panic_error() {
        let sys = Arc::new(SysAPI::new());
        let error = CString::new("error").unwrap();

        let callback = {
            let sys_c = Arc::clone(&sys);
            let error_c = error.clone();
            Callback::new(move || sys_c.panic(Some(error_c.as_c_str())))
        };

        let (context, callback) = unsafe { callback.take() };
        let result = sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(result, ExitStatus::Panic(Some(error)));
    }

    #[test]
    fn lock() {
        struct CellSend<T>(Cell<T>);

        unsafe impl<T> Send for CellSend<T> {}
        unsafe impl<T> Sync for CellSend<T> {}

        const ITERATIONS: usize = 10000;

        let sys = Arc::new(SysAPI::new());
        let data = Arc::new(CellSend(Cell::new(0usize)));

        let callback = {
            let sys_c = Arc::clone(&sys);
            let data_c = Arc::clone(&data);
            Callback::new(move || {
                let mut threads = Vec::new();
                for _ in 0..ITERATIONS {
                    let sys_thr = Arc::clone(&sys_c);
                    let data_thr = Arc::clone(&data_c);

                    threads.push(std::thread::spawn(move || {
                        sys_thr.lock();
                        data_thr.0.set(data_thr.0.get() + 1);
                        sys_thr.unlock();
                    }));
                }

                // Await for all threads to finish.
                for t in threads {
                    t.join().unwrap();
                }
            })
        };

        let (context, callback) = unsafe { callback.take() };
        sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(data.0.get(), ITERATIONS);
    }

    #[test]
    fn try_lock() {
        let sys = Arc::new(SysAPI::new());
        let data = Arc::new(AtomicBool::new(false));

        let callback = {
            let sys_c = Arc::clone(&sys);
            let data_c = Arc::clone(&data);
            Callback::new(move || {
                let barrier = Arc::new(Barrier::new(2));
                assert_eq!(sys_c.try_lock(), true);

                let t = {
                    let sys_t = Arc::clone(&sys_c);
                    let data_t = Arc::clone(&data_c);
                    let barrier_t = Arc::clone(&barrier);

                    std::thread::spawn(move || {
                        assert_eq!(sys_t.try_lock(), false);
                        barrier_t.wait();

                        sys_t.lock();
                        data_t.store(true, Ordering::Release);
                        sys_t.unlock();
                    })
                };

                barrier.wait();
                assert_eq!(data_c.load(Ordering::Acquire), false);
                sys_c.unlock();

                t.join().unwrap();
            })
        };

        let (context, callback) = unsafe { callback.take() };
        sys.enter_interface_from_thread(Some(context), callback);
        assert_eq!(data.load(Ordering::Acquire), true);
    }
}
