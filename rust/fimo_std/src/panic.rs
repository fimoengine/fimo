//! Panic utilities.

use crate::{error::AnyError, modules::symbols::Share};
use std::panic::AssertUnwindSafe;

/// Logs an error and aborts the process.
#[macro_export]
macro_rules! abort_error {
    () => {
        $crate::abort_error!("process was aborted due to an unknown reason")
    };
    ($($arg:tt)*) => {
        $crate::utils::abort_on_panic(|| {
            eprintln!($($arg)*);
        });
        std::process::abort();
    };
    ($ctx:expr, $($arg:tt)*) => {
        $crate::utils::abort_on_panic(|| {
            fimo_std::emit_error!($ctx, $($arg)*);
            fimo_std::tracing::TracingSubsystem::flush($ctx).expect("could not flush");
        });
        std::process::abort();
    };
}

/// Invokes a closure, aborting the process if a panic occurs.
pub fn abort_on_panic<R>(f: impl FnOnce() -> R) -> R {
    std::panic::catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_e| std::process::abort())
}

/// Invokes a closure, returning an [`AnyError`] if a panic occurs.
pub fn catch_unwind<R>(f: impl FnOnce() -> R) -> Result<R, AnyError<dyn Send + Share>> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).map_err(|e| match e.downcast::<&'static str>() {
        Ok(e) => AnyError::new_send(e),
        Err(e) => match e.downcast::<String>() {
            Ok(e) => AnyError::new_send(e),
            Err(_e) => AnyError::from_string(c"unknown error"),
        },
    })
}
