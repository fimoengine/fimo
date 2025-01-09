//! Panic utilities.

use crate::error::AnyError;
use std::{cell::Cell, panic::AssertUnwindSafe};

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
pub fn catch_unwind<R>(f: impl FnOnce() -> R) -> Result<R, AnyError<dyn Send>> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).map_err(|e| match e.downcast::<&'static str>() {
        Ok(e) => AnyError::new_send(e),
        Err(e) => match e.downcast::<String>() {
            Ok(e) => AnyError::new_send(e),
            Err(_e) => AnyError::from_string(c"unknown error"),
        },
    })
}

#[thread_local]
static CURRENT_CONTEXT: Cell<Option<crate::context::ContextView<'static>>> = Cell::new(None);

/// Replaces the current panic hook with a function that forwards the error to the tracing
/// subsystem.
///
/// The new panic hook will forward the panic info to the tracing subsystem, by emitting an error
/// event. If the tracing subsytem is disabled, or a panic occurs without a panic context set
/// (see [`with_panic_context`]), the implementation will forward the panic info to the previous
/// panic hook.
pub fn set_panic_hook() {
    std::panic::update_hook(|prev, info| {
        use crate::tracing::TracingSubsystem;

        // If no context is defined, we are forced to use the fallback.
        let current = CURRENT_CONTEXT.get();
        if current.is_none() {
            prev(info);
            return;
        }

        let context = current.unwrap();

        // We also utilize the fallback hook in case the tracing subsystem is disabled, as we would
        // not emit any event otherwise.
        if !context.is_enabled() {
            prev(info);
            return;
        }

        let backtrace = std::backtrace::Backtrace::capture();

        // The current implementation always returns `Some`.
        let location = info.location().unwrap();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");

        if backtrace.status() == std::backtrace::BacktraceStatus::Disabled {
            crate::emit_error!(
                context,
                "thread '{name}' panicked at {location}:\n{msg}\n\
                note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace"
            );
        } else {
            crate::emit_error!(context, "thread '{name}' panicked at {location}:\n{msg}");
        }
    });
}

/// Sets the panic context of the current thread.
///
/// The closure `f` is called with `context` set as the panic context.
/// The panic context is set per thread and will be unset once `f` finishes executing.
pub fn with_panic_context<'ctx, R>(
    context: crate::context::ContextView<'ctx>,
    f: impl FnOnce(&crate::context::ContextView<'ctx>) -> R,
) -> R {
    // Shortly extend the lifetime of the context, such that we can store it globally.
    let context = unsafe {
        std::mem::transmute::<crate::context::ContextView<'ctx>, crate::context::ContextView<'static>>(
            context,
        )
    };

    // Call the function in the new context.
    let old = CURRENT_CONTEXT.replace(Some(context));
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| f(&context)));
    CURRENT_CONTEXT.set(old);

    // Propagate any possible panic.
    match result {
        Ok(val) => val,
        Err(e) => std::panic::resume_unwind(e),
    }
}
