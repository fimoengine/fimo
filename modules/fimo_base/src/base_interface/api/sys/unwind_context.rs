use emf_core_base_rs::extensions::unwind_internal::UnwindInternalContextRef;
use emf_core_base_rs::ffi::collections::Optional;
use emf_core_base_rs::ffi::errors::Error as ErrorFFI;
use emf_core_base_rs::ffi::extensions::unwind_internal::Context;
use emf_core_base_rs::ffi::TypeWrapper;
use emf_core_base_rs::ownership::Owned;
use emf_core_base_rs::Error;
use std::ptr::NonNull;

/// A shutdown signal.
#[derive(Debug)]
pub struct ShutdownSignal {}

/// A panic signal.
#[derive(Debug)]
pub struct PanicSignal {
    /// Error message of the panic.
    pub error: Option<Error<Owned>>,
}

extern "C-unwind" fn shutdown(_context: Option<NonNull<Context>>) -> ! {
    std::panic::panic_any(ShutdownSignal {})
}

extern "C-unwind" fn panic(_context: Option<NonNull<Context>>, error: Optional<ErrorFFI>) -> ! {
    std::panic::panic_any(PanicSignal {
        error: error.map(Error::from).into_rust(),
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
    use crate::base_interface::api::sys::unwind_context::{construct_context, PanicSignal, ShutdownSignal};
    use emf_core_base_rs::ffi::collections::Optional;
    use emf_core_base_rs::ffi::errors::StaticError;

    #[test]
    fn shutdown() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = std::panic::catch_unwind(|| {
            let context = construct_context();
            unsafe { (*context._shutdown)(Some(context._context)) }
        });
        std::panic::set_hook(hook);

        assert!(result.is_err());
        assert!(result.err().unwrap().is::<ShutdownSignal>());
    }

    #[test]
    fn panic() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = std::panic::catch_unwind(|| {
            let context = construct_context();
            unsafe { (*context._panic)(Some(context._context), Optional::None) }
        });
        std::panic::set_hook(hook);

        let err = result.err().unwrap();
        assert!(err.is::<PanicSignal>());
        assert_eq!(err.downcast::<PanicSignal>().unwrap().error, None);
    }

    #[test]
    fn panic_error() {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = std::panic::catch_unwind(|| {
            let context = construct_context();
            unsafe {
                (*context._panic)(
                    Some(context._context),
                    Optional::Some(From::from(StaticError::new("My error"))),
                )
            }
        });
        std::panic::set_hook(hook);

        let err = result.err().unwrap();
        assert!(err.is::<PanicSignal>());
        assert_eq!(
            format!("{:?}", StaticError::new("My error")),
            format!(
                "{:?}",
                err.downcast::<PanicSignal>().unwrap().error.unwrap()
            ),
        );
    }
}