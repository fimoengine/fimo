//! Tracing subsystem.
use crate::{
    bindings,
    context::{private::SealedContext, Context, ContextView},
    error,
    error::{to_result_indirect, to_result_indirect_in_place, Error},
    ffi::{FFISharable, FFITransferable},
    time::Time,
};
use alloc::boxed::Box;
use core::{
    ffi::CStr,
    fmt::{Arguments, Write},
    mem::ManuallyDrop,
};

/// Definition of the tracing subsystem.
pub trait TracingSubsystem: SealedContext {
    /// Emits a new event.
    ///
    /// The message may be cut of, if the length exceeds the internal formatting buffer size.

    fn emit_event(&self, event: &Event, arguments: Arguments<'_>) -> error::Result;

    /// Checks whether the tracing subsystem is enabled.
    ///
    /// This function can be used to check whether to call into the subsystem at all. Calling this
    /// function is not necessary, as the remaining functions of the backend are guaranteed to
    /// return default values, in case the backend is disabled.
    fn is_enabled(&self) -> bool;

    /// Flushes the streams used for tracing.
    ///
    /// If successful, any unwritten data is written out by the individual subscribers.
    fn flush(&self) -> error::Result;
}

impl<T> TracingSubsystem for T
where
    T: SealedContext,
{
    fn emit_event(&self, event: &Event, arguments: Arguments<'_>) -> error::Result {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_event_emit_custom(
                self.share_to_ffi(),
                event.share_to_ffi(),
                Some(Formatter::format_into_buffer as _),
                core::ptr::from_ref(&arguments).cast(),
            );
        })
    }

    fn is_enabled(&self) -> bool {
        // Safety: FFI call is safe.
        unsafe { bindings::fimo_tracing_is_enabled(self.share_to_ffi()) }
    }

    fn flush(&self) -> error::Result {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_flush(self.share_to_ffi());
        })
    }
}

/// Constructs a new [`Span`].
#[macro_export]
macro_rules! tracing_span {
    ($ctx:expr, name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        let _ = {
            const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
                name: $name,
                target: $target,
                lvl: $lvl
            );
            const DESCRIPTOR: &'static $crate::tracing::SpanDescriptor =
                &$crate::tracing::SpanDescriptor::new(METADATA);
            $crate::tracing::Span::new($ctx, DESCRIPTOR, core::format_args!($($arg)+))
                .expect("could not create span")
        };
    };
    ($ctx:expr, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        let _ = {
            const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
                target: $target,
                lvl: $lvl
            );
            const DESCRIPTOR: &'static $crate::tracing::SpanDescriptor =
                &$crate::tracing::SpanDescriptor::new(METADATA);
            $crate::tracing::Span::new($ctx, DESCRIPTOR, core::format_args!($($arg)+))
                .expect("could not create span")
        };
    };
    ($ctx:expr, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, target: "", lvl: $lvl, $($arg)+)
    };
}

/// Emits a new [`Event`].
#[macro_export]
macro_rules! tracing_emit {
    ($ctx:expr, name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        use $crate::tracing::TracingSubsystem;
        const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
            name: $name,
            target: $target,
            lvl: $lvl
        );
        const EVENT: &'static $crate::tracing::Event = &$crate::tracing::Event::new(METADATA);
        $ctx.emit_event(EVENT, core::format_args!($($arg)+)).expect("could not emit event");
    }};
    ($ctx:expr, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        use $crate::tracing::TracingSubsystem;
        const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
            target: $target,
            lvl: $lvl
        );
        const EVENT: &'static $crate::tracing::Event = &$crate::tracing::Event::new(METADATA);
        $ctx.emit_event(EVENT, core::format_args!($($arg)+)).expect("could not emit event");
    }};
    ($ctx:expr, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, target: "", lvl: $lvl, $($arg)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! tracing_metadata {
    (name: $name:literal, target: $target:literal, lvl: $lvl:expr) => {{
        const NAME: &'static str = core::concat!($name, '\0');
        const TARGET: &'static str = core::concat!($target, '\0');
        const FILE: &'static str = core::concat!(core::file!(), '\0');
        const LINE: u32 = core::line!() as u32;

        const NAME_CSTR: &'static core::ffi::CStr =
            // Safety:
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(NAME.as_bytes()) };
        const TARGET_CSTR: &'static core::ffi::CStr =
            // Safety:
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(TARGET.as_bytes()) };
        const FILE_CSTR: &'static core::ffi::CStr =
            // Safety:
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(FILE.as_bytes()) };

        const METADATA: &'static $crate::tracing::Metadata = &$crate::tracing::Metadata::new(
            NAME_CSTR,
            TARGET_CSTR,
            $lvl,
            Some(FILE_CSTR),
            Some(LINE),
        );
        METADATA
    }};
    (target: $target:literal, lvl: $lvl:expr) => {{
        const NAME: &'static str = core::concat!(core::module_path!(), '\0');
        const TARGET: &'static str = core::concat!($target, '\0');
        const FILE: &'static str = core::concat!(core::file!(), '\0');
        const LINE: u32 = core::line!() as u32;

        const NAME_CSTR: &'static core::ffi::CStr =
            // Safety:
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(NAME.as_bytes()) };
        const TARGET_CSTR: &'static core::ffi::CStr =
            // Safety:
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(TARGET.as_bytes()) };
        const FILE_CSTR: &'static core::ffi::CStr =
            // Safety:
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(FILE.as_bytes()) };

        const METADATA: &'static $crate::tracing::Metadata = &$crate::tracing::Metadata::new(
            NAME_CSTR,
            TARGET_CSTR,
            $lvl,
            Some(FILE_CSTR),
            Some(LINE),
        );
        METADATA
    }};
}

/// Emits a new [`Level::Error`] event.
#[macro_export]
macro_rules! emit_error {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
}

/// Emits a new [`Level::Warn`] event.
#[macro_export]
macro_rules! emit_warn {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
}

/// Emits a new [`Level::Info`] event.
#[macro_export]
macro_rules! emit_info {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
}

/// Emits a new [`Level::Debug`] event.
#[macro_export]
macro_rules! emit_debug {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
}

/// Emits a new [`Level::Trace`] event.
#[macro_export]
macro_rules! emit_trace {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_emit!($ctx, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
}

/// Constructs a new [`Level::Error`] span.
#[macro_export]
macro_rules! span_error {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
}

/// Constructs a new [`Level::Warn`] span.
#[macro_export]
macro_rules! span_warn {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
}

/// Constructs a new [`Level::Info`] span.
#[macro_export]
macro_rules! span_info {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
}

/// Constructs a new [`Level::Debug`] span.
#[macro_export]
macro_rules! span_debug {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
}

/// Constructs a new [`Level::Trace`] span.
#[macro_export]
macro_rules! span_trace {
    ($ctx:expr, name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, name: $name, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($ctx:expr, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($ctx:expr, $($arg:tt)+) => {
        $crate::tracing_span!($ctx, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
}

/// Available levels in the tracing subsystem.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Level {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    const fn to_ffi(self) -> bindings::FimoTracingLevel {
        match self {
            Level::Off => bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_OFF,
            Level::Error => bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_ERROR,
            Level::Warn => bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_WARN,
            Level::Info => bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_INFO,
            Level::Debug => bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_DEBUG,
            Level::Trace => bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_TRACE,
        }
    }
}

impl From<Level> for bindings::FimoTracingLevel {
    fn from(value: Level) -> Self {
        value.to_ffi()
    }
}

impl TryFrom<bindings::FimoTracingLevel> for Level {
    type Error = Error;

    fn try_from(
        value: bindings::FimoTracingLevel,
    ) -> Result<Self, <Level as TryFrom<bindings::FimoTracingLevel>>::Error> {
        match value {
            bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_OFF => Ok(Level::Off),
            bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_ERROR => Ok(Level::Error),
            bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_WARN => Ok(Level::Warn),
            bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_INFO => Ok(Level::Info),
            bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_DEBUG => Ok(Level::Debug),
            bindings::FimoTracingLevel::FIMO_TRACING_LEVEL_TRACE => Ok(Level::Trace),
            bindings::FimoTracingLevel(_) => Err(Error::EINVAL),
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Metadata(bindings::FimoTracingMetadata);

impl Metadata {
    pub const fn new(
        name: &'static CStr,
        target: &'static CStr,
        level: Level,
        file_name: Option<&'static CStr>,
        line_number: Option<u32>,
    ) -> Metadata {
        Self(bindings::FimoTracingMetadata {
            type_: bindings::FimoStructType::FIMO_STRUCT_TYPE_TRACING_METADATA,
            next: core::ptr::null(),
            name: name.as_ptr().cast(),
            target: target.as_ptr().cast(),
            level: level.to_ffi(),
            file_name: match file_name {
                None => core::ptr::null(),
                Some(x) => x.as_ptr().cast(),
            },
            line_number: match line_number {
                None => -1,
                Some(x) => x as i32,
            },
        })
    }

    /// Returns the name contained in the `Metadata`.
    pub fn name(&self) -> &CStr {
        // Safety: Must contain a valid string.
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Returns the target contained in the `Metadata`.
    pub fn target(&self) -> &CStr {
        // Safety: Must contain a valid string.
        unsafe { CStr::from_ptr(self.0.target) }
    }

    /// Returns the level contained in the `Metadata`.
    pub fn level(&self) -> Level {
        self.0.level.try_into().expect("must contain a valid level")
    }

    /// Returns the file name contained in the `Metadata`.
    pub fn file_name(&self) -> Option<&CStr> {
        // Safety: Must contain a valid string or null.
        unsafe { self.0.file_name.as_ref().map(|x| CStr::from_ptr(x)) }
    }

    /// Returns the file number contained in the `Metadata`.
    pub fn line_number(&self) -> Option<u32> {
        if self.0.line_number < 0 {
            None
        } else {
            Some(self.0.line_number as u32)
        }
    }
}

// Safety: The metadata is `Send` and `Sync`.
unsafe impl Send for Metadata {}

// Safety: The metadata is `Send` and `Sync`.
unsafe impl Sync for Metadata {}

impl FFISharable<*const bindings::FimoTracingMetadata> for Metadata {
    type BorrowedView<'a> = &'a Metadata;

    fn share_to_ffi(&self) -> *const bindings::FimoTracingMetadata {
        &self.0
    }

    unsafe fn borrow_from_ffi<'a>(
        ffi: *const bindings::FimoTracingMetadata,
    ) -> Self::BorrowedView<'a> {
        // Safety: `Metadata` is transparent.
        unsafe { &*ffi.cast() }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Event(bindings::FimoTracingEvent);

impl Event {
    /// Constructs a new event.
    pub const fn new(metadata: &'static Metadata) -> Self {
        Self(bindings::FimoTracingEvent {
            type_: bindings::FimoStructType::FIMO_STRUCT_TYPE_TRACING_EVENT,
            next: core::ptr::null(),
            metadata: &metadata.0,
        })
    }
}

impl FFISharable<*const bindings::FimoTracingEvent> for Event {
    type BorrowedView<'a> = &'a Event;

    fn share_to_ffi(&self) -> *const bindings::FimoTracingEvent {
        &self.0
    }

    unsafe fn borrow_from_ffi<'a>(
        ffi: *const bindings::FimoTracingEvent,
    ) -> Self::BorrowedView<'a> {
        // Safety: `Event` is transparent.
        unsafe { &*ffi.cast() }
    }
}

/// Descriptor of a new span.
#[derive(Debug)]
#[repr(transparent)]
pub struct SpanDescriptor(bindings::FimoTracingSpanDesc);

impl SpanDescriptor {
    /// Constructs a new `SpanDescriptor`.
    pub const fn new(metadata: &'static Metadata) -> Self {
        Self(bindings::FimoTracingSpanDesc {
            type_: bindings::FimoStructType::FIMO_STRUCT_TYPE_TRACING_SPAN_DESC,
            next: core::ptr::null(),
            metadata: &metadata.0,
        })
    }

    /// Returns a reference to the contained [`Metadata`].
    pub fn metadata(&self) -> &Metadata {
        // Safety: The pointer must be valid.
        unsafe { Metadata::borrow_from_ffi(self.0.metadata) }
    }
}

impl FFISharable<*const bindings::FimoTracingSpanDesc> for SpanDescriptor {
    type BorrowedView<'a> = &'a SpanDescriptor;

    fn share_to_ffi(&self) -> *const bindings::FimoTracingSpanDesc {
        &self.0
    }

    unsafe fn borrow_from_ffi<'a>(
        ffi: *const bindings::FimoTracingSpanDesc,
    ) -> Self::BorrowedView<'a> {
        // Safety: `SpanDescriptor` is transparent.
        unsafe { &*ffi.cast() }
    }
}

/// A tracing span.
#[derive(Debug)]
pub struct Span(Context, bindings::FimoTracingSpan);

impl Span {
    /// Creates a new span and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message may be cut of, if the length exceeds the internal formatting buffer size.
    pub fn new(
        ctx: &ContextView<'_>,
        span_descriptor: &'static SpanDescriptor,
        arguments: Arguments<'_>,
    ) -> Result<Self, Error> {
        // Safety: FFI call is safe.
        let span = unsafe {
            to_result_indirect_in_place(|error, span| {
                *error = bindings::fimo_tracing_span_create_custom(
                    ctx.share_to_ffi(),
                    span_descriptor.share_to_ffi(),
                    span.as_mut_ptr(),
                    Some(Formatter::format_into_buffer),
                    core::ptr::from_ref(&arguments).cast(),
                );
            })?
        };

        Ok(Self(ctx.to_context(), span))
    }
}

// Safety: `Span` is `Send` and `Sync`.
unsafe impl Send for Span {}

// Safety: `Span` is `Send` and `Sync`.
unsafe impl Sync for Span {}

impl Drop for Span {
    fn drop(&mut self) {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_span_destroy(self.0.share_to_ffi(), &mut self.1);
        })
        .expect("the span should be destroyable");
    }
}

/// A call stack.
pub struct CallStack(Context, bindings::FimoTracingCallStack);

impl CallStack {
    /// Creates a new empty call stack.
    ///
    /// If successful, the new call stack is marked as suspended. The new call stack is not set to
    /// be the active call stack.
    pub fn new(ctx: &ContextView<'_>) -> Result<Self, Error> {
        // Safety: FFI call is safe.
        let stack = unsafe {
            to_result_indirect_in_place(|error, stack| {
                *error = bindings::fimo_tracing_call_stack_create(
                    ctx.share_to_ffi(),
                    stack.as_mut_ptr(),
                );
            })?
        };

        Ok(Self(ctx.to_context(), stack))
    }

    /// Switches the call stack of the current thread.
    ///
    /// If successful, this call stack will be used as the active call stack of the calling thread.
    /// The old call stack is then returned, enabling the caller to switch back to it afterward.
    /// `self` must be in a suspended, but unblocked, state and not be active. The active call stack
    /// must also be in a suspended state, but may also be blocked. On error, this function returns
    /// `self`, along with an error.
    pub fn switch(self) -> Result<Self, (Self, Error)> {
        let this = ManuallyDrop::new(self);

        // Safety: FFI call is safe.
        let stack = unsafe {
            to_result_indirect_in_place(|error, stack| {
                *error = bindings::fimo_tracing_call_stack_create(
                    this.0.share_to_ffi(),
                    stack.as_mut_ptr(),
                );
            })
        };
        let stack = match stack {
            Ok(x) => x,
            Err(e) => return Err((ManuallyDrop::into_inner(this), e)),
        };

        let mut new = ManuallyDrop::into_inner(this);
        new.1 = stack;
        Ok(new)
    }

    /// Unblocks the blocked call stack.
    ///
    /// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
    /// marked as blocked.
    pub fn unblock(&mut self) -> error::Result {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_call_stack_unblock(self.0.share_to_ffi(), self.1);
        })
    }

    /// Marks the current call stack as being suspended.
    ///
    /// While suspended, the call stack can not be utilized for tracing messages. The call stack
    /// can optionally also be marked as blocked. In that case, the call stack must be unblocked
    /// prior to resumption.
    pub fn suspend_current(ctx: &ContextView<'_>, block: bool) -> error::Result {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_call_stack_suspend_current(ctx.share_to_ffi(), block);
        })
    }

    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the call stack can be used to trace messages. To be successful, the current
    /// call stack must be suspended and unblocked.
    pub fn resume_current(ctx: &ContextView<'_>) -> error::Result {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_call_stack_resume_current(ctx.share_to_ffi());
        })
    }
}

// Safety: A `CallStack` is `Send` and `Sync`.
unsafe impl Send for CallStack {}

// Safety: A `CallStack` is `Send` and `Sync`.
unsafe impl Sync for CallStack {}

impl Drop for CallStack {
    fn drop(&mut self) {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_call_stack_destroy(self.0.share_to_ffi(), self.1);
        })
        .expect("the call stack should be destroyable");
    }
}

/// RAII access provider to the [`TracingSubsystem`] for a thread.
#[derive(Debug)]
pub struct ThreadAccess(Context);

impl ThreadAccess {
    /// Registers the calling thread with the tracing subsystem.
    ///
    /// The tracing of the subsystem is opt-in on a per-thread basis, where unregistered threads
    /// will behave as if the backend was disabled. Once registered, the calling thread gains access
    /// to the tracing subsystem and is assigned a new empty call stack.
    pub fn new(ctx: &ContextView<'_>) -> Result<Self, Error> {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_register_thread(ctx.share_to_ffi());
        })?;

        Ok(Self(ctx.to_context()))
    }

    /// Unregisters the calling thread from the tracing subsystem.
    ///
    /// Once unregistered, the calling thread looses access to the tracing subsystem until it is
    /// registered again. The thread can not be unregistered until the call stack is empty.
    pub fn unregister(self) -> Result<(), (Self, Error)> {
        let this = ManuallyDrop::new(self);
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_unregister_thread(this.0.share_to_ffi());
        })
        .map_err(move |e| (ManuallyDrop::into_inner(this), e))
    }
}

impl Drop for ThreadAccess {
    fn drop(&mut self) {
        // Safety: FFI call is safe.
        to_result_indirect(|error| unsafe {
            *error = bindings::fimo_tracing_unregister_thread(self.0.share_to_ffi());
        })
        .expect("should be able to unregister a thread");
    }
}

/// Interface of a tracing subscriber.
pub trait Subscriber: Send + Sync {
    /// Type of the internal call stack.
    type CallStack;

    /// Type of the internal span.
    type Span;

    /// Creates a new call stack.
    fn create_call_stack(&self, time: Time) -> Result<Box<Self::CallStack>, Error>;

    /// Destroys the call stack.
    fn destroy_call_stack(&self, time: Time, call_stack: Box<Self::CallStack>);

    /// Marks the call stack as being unblocked.
    fn unblock_call_stack(&self, time: Time, call_stack: &mut Self::CallStack);

    /// Marks the stack as being suspended/blocked.
    fn suspend_call_stack(&self, time: Time, call_stack: &mut Self::CallStack, block: bool);

    /// Marks the stack as being resumed.
    fn resume_call_stack(&self, time: Time, call_stack: &mut Self::CallStack);

    /// Creates a new span.
    fn create_span(
        &self,
        time: Time,
        span_descriptor: &SpanDescriptor,
        message: &[u8],
        call_stack: &mut Self::CallStack,
    ) -> Result<Box<Self::Span>, Error>;

    /// Exits and destroys a span.
    fn destroy_span(&self, time: Time, call_stack: &mut Self::CallStack, span: Box<Self::Span>);

    /// Emits an event.
    fn emit_event(
        &self,
        time: Time,
        call_stack: &mut Self::CallStack,
        event: &Event,
        message: &[u8],
    );

    /// Flushes the messages of the `Subscriber`.
    fn flush(&self);
}

/// A type-erased [`Subscriber`].
#[derive(Debug)]
#[repr(transparent)]
pub struct OpaqueSubscriber(bindings::FimoTracingSubscriber);

impl OpaqueSubscriber {
    /// Constructs a new `OpaqueSubscriber` from a reference to a [`Subscriber`].
    pub const fn from_ref<T: Subscriber>(subscriber: &'static T) -> Self {
        trait VTableProvider {
            const TABLE: bindings::FimoTracingSubscriberVTable;
        }
        impl<T: Subscriber> VTableProvider for T {
            const TABLE: bindings::FimoTracingSubscriberVTable =
                OpaqueSubscriber::build_vtable::<T>(None);
        }

        let vtable: &'static bindings::FimoTracingSubscriberVTable = &<T as VTableProvider>::TABLE;
        Self(bindings::FimoTracingSubscriber {
            type_: bindings::FimoStructType::FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER,
            next: core::ptr::null(),
            ptr: core::ptr::from_ref(subscriber).cast_mut().cast(),
            vtable: core::ptr::from_ref(vtable),
        })
    }

    /// Constructs a new `OpaqueSubscriber` from a boxed [`Subscriber`].
    pub fn from_box<T: Subscriber>(subscriber: Box<T>) -> Self {
        trait VTableProvider {
            const TABLE: bindings::FimoTracingSubscriberVTable;
        }
        impl<T: Subscriber> VTableProvider for T {
            const TABLE: bindings::FimoTracingSubscriberVTable =
                OpaqueSubscriber::build_vtable::<T>(Some(drop_box::<T>));
        }
        unsafe extern "C" fn drop_box<T>(ptr: *mut core::ffi::c_void) {
            // Safety: We know that the type is right.
            unsafe { drop(Box::from_raw(ptr.cast::<T>())) }
        }

        let vtable: &'static bindings::FimoTracingSubscriberVTable = &<T as VTableProvider>::TABLE;
        Self(bindings::FimoTracingSubscriber {
            type_: bindings::FimoStructType::FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER,
            next: core::ptr::null(),
            ptr: Box::into_raw(subscriber).cast(),
            vtable: core::ptr::from_ref(vtable),
        })
    }

    const fn build_vtable<T: Subscriber>(
        drop_fn: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
    ) -> bindings::FimoTracingSubscriberVTable {
        unsafe extern "C" fn call_stack_create<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut *mut core::ffi::c_void,
        ) -> bindings::FimoError {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                match subscriber.create_call_stack(time) {
                    Ok(x) => {
                        core::ptr::write(stack, Box::into_raw(x).cast());
                        Error::EOK.into_error()
                    }
                    Err(e) => e.into_error(),
                }
            }
        }
        unsafe extern "C" fn call_stack_destroy<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut core::ffi::c_void,
        ) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let stack = Box::from_raw(stack.cast());
                subscriber.destroy_call_stack(time, stack);
            }
        }
        unsafe extern "C" fn call_stack_unblock<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut core::ffi::c_void,
        ) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let stack = &mut *stack.cast();
                subscriber.unblock_call_stack(time, stack);
            }
        }
        unsafe extern "C" fn call_stack_suspend<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut core::ffi::c_void,
            block: bool,
        ) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let stack = &mut *stack.cast();
                subscriber.suspend_call_stack(time, stack, block);
            }
        }
        unsafe extern "C" fn call_stack_resume<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut core::ffi::c_void,
        ) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let stack = &mut *stack.cast();
                subscriber.resume_call_stack(time, stack);
            }
        }
        unsafe extern "C" fn span_create<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            span_descriptor: *const bindings::FimoTracingSpanDesc,
            message: *const core::ffi::c_char,
            message_length: usize,
            stack: *mut core::ffi::c_void,
            span: *mut *mut core::ffi::c_void,
        ) -> bindings::FimoError {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let span_descriptor = SpanDescriptor::borrow_from_ffi(span_descriptor);
                let message = core::slice::from_raw_parts(message.cast(), message_length);
                let stack = &mut *stack.cast();
                match subscriber.create_span(time, span_descriptor, message, stack) {
                    Ok(x) => {
                        core::ptr::write(span, Box::into_raw(x).cast());
                        Error::EOK.into_error()
                    }
                    Err(e) => e.into_error(),
                }
            }
        }
        unsafe extern "C" fn span_destroy<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut core::ffi::c_void,
            span: *mut core::ffi::c_void,
        ) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let stack = &mut *stack.cast();
                let span = Box::from_raw(span.cast());
                subscriber.destroy_span(time, stack, span);
            }
        }
        unsafe extern "C" fn event_emit<T: Subscriber>(
            subscriber: *mut core::ffi::c_void,
            time: *const bindings::FimoTime,
            stack: *mut core::ffi::c_void,
            event: *const bindings::FimoTracingEvent,
            message: *const core::ffi::c_char,
            message_length: usize,
        ) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                let time = Time::from_ffi(*time);
                let stack = &mut *stack.cast();
                let event = Event::borrow_from_ffi(event);
                let message = core::slice::from_raw_parts(message.cast(), message_length);
                subscriber.emit_event(time, stack, event, message);
            }
        }
        unsafe extern "C" fn flush<T: Subscriber>(subscriber: *mut core::ffi::c_void) {
            // Safety:
            unsafe {
                let subscriber: &T = &*subscriber.cast::<T>().cast_const();
                subscriber.flush();
            }
        }

        bindings::FimoTracingSubscriberVTable {
            destroy: drop_fn,
            call_stack_create: Some(call_stack_create::<T>),
            call_stack_destroy: Some(call_stack_destroy::<T>),
            call_stack_unblock: Some(call_stack_unblock::<T>),
            call_stack_suspend: Some(call_stack_suspend::<T>),
            call_stack_resume: Some(call_stack_resume::<T>),
            span_create: Some(span_create::<T>),
            span_destroy: Some(span_destroy::<T>),
            event_emit: Some(event_emit::<T>),
            flush: Some(flush::<T>),
        }
    }
}

// Safety: A `Subscriber` is `Send` and `Sync`.
unsafe impl Send for OpaqueSubscriber {}

// Safety: A `Subscriber` is `Send` and `Sync`.
unsafe impl Sync for OpaqueSubscriber {}

impl Drop for OpaqueSubscriber {
    fn drop(&mut self) {
        // Safety: The pointers must all be valid.
        unsafe {
            let vtable = &*self.0.vtable;
            if let Some(destroy) = vtable.destroy {
                destroy(self.0.ptr);
            }
        }
    }
}

struct Formatter<'a> {
    buffer: &'a mut [u8],
    pos: usize,
}

impl Formatter<'_> {
    unsafe fn new(buffer: *mut core::ffi::c_char, buffer_len: usize) -> Self {
        if buffer.is_null() {
            Self {
                buffer: &mut [],
                pos: 0,
            }
        } else {
            // Safety: The buffer must be valid.
            unsafe {
                Self {
                    buffer: core::slice::from_raw_parts_mut(buffer.cast(), buffer_len),
                    pos: 0,
                }
            }
        }
    }

    unsafe extern "C" fn format_into_buffer(
        buffer: *mut core::ffi::c_char,
        buffer_len: usize,
        data: *const core::ffi::c_void,
        written: *mut usize,
    ) -> bindings::FimoError {
        // Safety: The buffer should be valid.
        unsafe {
            let mut f = Self::new(buffer, buffer_len);
            let _ = f.write_fmt(*data.cast::<core::fmt::Arguments<'_>>());
            core::ptr::write(written, f.pos.min(f.buffer.len()));
            Error::EOK.into_error()
        }
    }
}

impl Write for Formatter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end_pos = self.pos.saturating_add(s.len());

        // Skip if we can't write any more data.
        if end_pos <= self.buffer.len() {
            let slice = &mut self.buffer[self.pos..end_pos];
            slice.copy_from_slice(s.as_bytes());
        }
        self.pos = end_pos;

        Ok(())
    }
}
