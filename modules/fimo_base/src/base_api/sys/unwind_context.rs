use emf_core_base_rs::extensions::unwind_internal::UnwindInternalContextRef;
use emf_core_base_rs::ffi::collections::NonNullConst;
use emf_core_base_rs::ffi::extensions::unwind_internal::Context;
use emf_core_base_rs::ffi::TypeWrapper;
use std::ffi::{CStr, CString};
use std::ptr::NonNull;

/// A shutdown signal.
#[derive(Debug)]
pub struct ShutdownSignal {}

/// A panic signal.
#[derive(Debug)]
pub struct PanicSignal {
    /// Error message of the panic.
    pub error: Option<CString>,
}

extern "C-unwind" fn shutdown(_context: Option<NonNull<Context>>) -> ! {
    std::panic::panic_any(ShutdownSignal {})
}

extern "C-unwind" fn panic(
    _context: Option<NonNull<Context>>,
    error: Option<NonNullConst<u8>>,
) -> ! {
    std::panic::panic_any(PanicSignal {
        error: error.map(|e| unsafe { CStr::from_ptr(e.cast().as_ptr()) }.to_owned()),
    })
}

pub fn construct_context() -> UnwindInternalContextRef {
    UnwindInternalContextRef {
        _context: NonNull::dangling(),
        _shutdown: TypeWrapper(shutdown),
        _panic: TypeWrapper(panic),
    }
}

#[cfg(test)]
mod tests {
    use crate::base_api::sys::unwind_context::{construct_context, PanicSignal, ShutdownSignal};
    use emf_core_base_rs::ffi::collections::NonNullConst;
    use std::ffi::CStr;

    #[test]
    fn shutdown() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = std::panic::catch_unwind(|| {
            let context = construct_context();
            unsafe { (*context._shutdown)(Some(context._context)) }
        });
        std::panic::set_hook(hook);

        assert_eq!(result.is_err(), true);
        assert_eq!(result.err().unwrap().is::<ShutdownSignal>(), true);
    }

    #[test]
    fn panic() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = std::panic::catch_unwind(|| {
            let context = construct_context();
            unsafe { (*context._panic)(Some(context._context), None) }
        });
        std::panic::set_hook(hook);

        let err = result.err().unwrap();
        assert_eq!(err.is::<PanicSignal>(), true);
        assert_eq!(err.downcast::<PanicSignal>().unwrap().error, None);
    }

    #[test]
    fn panic_error() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let error = CStr::from_bytes_with_nul(b"My error\0").unwrap();

        let result = std::panic::catch_unwind(|| {
            let context = construct_context();
            unsafe {
                (*context._panic)(
                    Some(context._context),
                    Some(NonNullConst::from(error.to_bytes_with_nul())),
                )
            }
        });
        std::panic::set_hook(hook);

        let err = result.err().unwrap();
        assert_eq!(err.is::<PanicSignal>(), true);
        assert_eq!(
            err.downcast::<PanicSignal>().unwrap().error,
            Some(error.to_owned())
        );
    }
}
