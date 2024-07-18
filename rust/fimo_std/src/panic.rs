use crate::error::Error;
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

/// Invokes a closure, returning an [`Error`] if a panic occurs.
pub fn catch_unwind<R>(f: impl FnOnce() -> R) -> Result<R, Error> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).map_err(|_e| Error::EUNKNOWN)
}
