//! Tracing subsystem.
use crate::{
    context::{Context, ContextHandle, ContextView, TypeId},
    ffi::{ConstCStr, ConstNonNull, OpaqueHandle, VTablePtr, Viewable},
    handle,
    time::Time,
};
use std::{
    ffi::CStr,
    fmt::{Arguments, Debug, Write},
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ptr::NonNull,
    sync::Arc,
};

/// Virtual function table of the tracing subsystem.
#[repr(C)]
#[derive(Debug)]
pub struct VTableV0 {
    pub create_call_stack: unsafe extern "C" fn(handle: ContextHandle) -> CallStack,
    pub suspend_current_call_stack: unsafe extern "C" fn(handle: ContextHandle, block: bool),
    pub resume_current_call_stack: unsafe extern "C" fn(handle: ContextHandle),
    pub create_span: unsafe extern "C" fn(
        handle: ContextHandle,
        desc: &SpanDescriptor,
        formatter: unsafe extern "C" fn(
            buffer: NonNull<u8>,
            len: usize,
            data: Option<ConstNonNull<()>>,
            written: &mut MaybeUninit<usize>,
        ),
        data: Option<ConstNonNull<()>>,
    ) -> Span,
    pub emit_event: unsafe extern "C" fn(
        handle: ContextHandle,
        event: &Event,
        formatter: unsafe extern "C" fn(
            buffer: NonNull<u8>,
            len: usize,
            data: Option<ConstNonNull<()>>,
            written: &mut MaybeUninit<usize>,
        ),
        data: Option<ConstNonNull<()>>,
    ),
    pub is_enabled: unsafe extern "C" fn(handle: ContextHandle) -> bool,
    pub register_thread: unsafe extern "C" fn(handle: ContextHandle),
    pub unregister_thread: unsafe extern "C" fn(handle: ContextHandle),
    pub flush: unsafe extern "C" fn(handle: ContextHandle),
}

/// Definition of the tracing subsystem.
pub trait TracingSubsystem: Copy {
    /// Emits a new event.
    ///
    /// The message may be cut of, if the length exceeds the internal formatting buffer size.
    fn emit_event(self, event: &Event, arguments: Arguments<'_>);

    /// Checks whether the tracing subsystem is enabled.
    ///
    /// This function can be used to check whether to call into the subsystem at all. Calling this
    /// function is not necessary, as the remaining functions of the backend are guaranteed to
    /// return default values, in case the backend is disabled.
    fn is_enabled(self) -> bool;

    /// Flushes the streams used for tracing.
    ///
    /// If successful, any unwritten data is written out by the individual subscribers.
    fn flush(self);
}

impl<'a, T> TracingSubsystem for T
where
    T: Viewable<ContextView<'a>>,
{
    #[inline(always)]
    fn emit_event(self, event: &Event, arguments: Arguments<'_>) {
        let ctx = self.view();
        let f = ctx.vtable.tracing_v0.emit_event;
        unsafe {
            f(
                ctx.handle,
                event,
                Formatter::format_into_buffer,
                Some(ConstNonNull::new_unchecked(&raw const arguments).cast()),
            );
        }
    }

    #[inline(always)]
    fn is_enabled(self) -> bool {
        let ctx = self.view();
        let f = ctx.vtable.tracing_v0.is_enabled;
        unsafe { f(ctx.handle) }
    }

    #[inline(always)]
    fn flush(self) {
        let ctx = self.view();
        let f = ctx.vtable.tracing_v0.flush;
        unsafe { f(ctx.handle) }
    }
}

/// Constructs a new [`Span`].
#[macro_export]
macro_rules! tracing_span {
    ($ctx:expr, name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        {
            const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
                name: $name,
                target: $target,
                lvl: $lvl
            );
            const DESCRIPTOR: &'static $crate::tracing::SpanDescriptor =
                &$crate::tracing::SpanDescriptor::new(METADATA);
            $crate::tracing::Span::new($ctx, DESCRIPTOR, core::format_args!($($arg)+))
        }
    };
    ($ctx:expr, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        {
            const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
                target: $target,
                lvl: $lvl
            );
            const DESCRIPTOR: &'static $crate::tracing::SpanDescriptor =
                &$crate::tracing::SpanDescriptor::new(METADATA);
            $crate::tracing::Span::new($ctx, DESCRIPTOR, core::format_args!($($arg)+))
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
        $ctx.emit_event(EVENT, core::format_args!($($arg)+));
    }};
    ($ctx:expr, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        use $crate::tracing::TracingSubsystem;
        const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
            target: $target,
            lvl: $lvl
        );
        const EVENT: &'static $crate::tracing::Event = &$crate::tracing::Event::new(METADATA);
        $ctx.emit_event(EVENT, core::format_args!($($arg)+));
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
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(NAME.as_bytes()) };
        const TARGET_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(TARGET.as_bytes()) };
        const FILE_CSTR: &'static core::ffi::CStr =
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
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(NAME.as_bytes()) };
        const TARGET_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(TARGET.as_bytes()) };
        const FILE_CSTR: &'static core::ffi::CStr =
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

/// Tracing levels.
///
/// The levels are ordered such that given two levels `lvl1` and `lvl2`, where `lvl1 >= lvl2`, then
/// an event with level `lvl2` will be traced in a context where the maximum tracing level is
/// `lvl1`.
#[repr(i32)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Level {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Metadata for a span and event.
#[repr(C)]
#[derive(Debug)]
pub struct Metadata {
    pub next: Option<OpaqueHandle<dyn Send + Sync>>,
    pub name: ConstCStr,
    pub target: ConstCStr,
    pub level: Level,
    pub file_name: Option<ConstCStr>,
    pub line_number: i32,
}

impl Metadata {
    pub const fn new(
        name: &'static CStr,
        target: &'static CStr,
        level: Level,
        file_name: Option<&'static CStr>,
        line_number: Option<u32>,
    ) -> Metadata {
        Self {
            next: None,
            name: ConstCStr::new(name),
            target: ConstCStr::new(target),
            level,
            file_name: match file_name {
                None => None,
                Some(x) => Some(ConstCStr::new(x)),
            },
            line_number: match line_number {
                None => -1,
                Some(x) => {
                    assert!(x <= i32::MAX as u32);
                    x as i32
                }
            },
        }
    }

    /// Returns the name contained in the `Metadata`.
    pub fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }

    /// Returns the target contained in the `Metadata`.
    pub fn target(&self) -> &CStr {
        unsafe { self.target.as_ref() }
    }

    /// Returns the file name contained in the `Metadata`.
    pub fn file_name(&self) -> Option<&CStr> {
        unsafe { self.file_name.map(|x| x.as_ref()) }
    }

    /// Returns the file number contained in the `Metadata`.
    pub fn line_number(&self) -> Option<u32> {
        if self.line_number < 0 {
            None
        } else {
            Some(self.line_number as u32)
        }
    }
}

/// An event to be traced.
#[repr(C)]
#[derive(Debug)]
pub struct Event {
    pub next: Option<OpaqueHandle<dyn Send + Sync>>,
    pub metadata: &'static Metadata,
}

impl Event {
    /// Constructs a new event.
    pub const fn new(metadata: &'static Metadata) -> Self {
        Self {
            next: None,
            metadata,
        }
    }
}

/// Descriptor of a new span.
#[repr(C)]
#[derive(Debug)]
pub struct SpanDescriptor {
    pub next: Option<OpaqueHandle<dyn Send + Sync>>,
    pub metadata: &'static Metadata,
}

impl SpanDescriptor {
    /// Constructs a new `SpanDescriptor`.
    pub const fn new(metadata: &'static Metadata) -> Self {
        Self {
            next: None,
            metadata,
        }
    }
}

handle!(pub handle SpanHandle: Send + Sync);

/// Virtual function table of a [`Span`].
#[repr(C)]
#[derive(Debug)]
pub struct SpanVTable {
    pub drop: unsafe extern "C" fn(handle: SpanHandle),
    pub(crate) _private: PhantomData<()>,
}

impl SpanVTable {
    cfg_internal! {
        /// Constructs a new `SpanVTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(drop: unsafe extern "C" fn(handle: SpanHandle)) -> Self {
            Self {
                drop,
                _private: PhantomData,
            }
        }
    }
}

/// A period of time, during which events can occur.
#[repr(C)]
#[derive(Debug)]
pub struct Span {
    pub handle: SpanHandle,
    pub vtable: VTablePtr<SpanVTable>,
}

impl Span {
    /// Creates a new span and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message may be cut of, if the length exceeds the internal formatting buffer size.
    pub fn new(
        ctx: impl Viewable<ContextView<'_>>,
        span_descriptor: &'static SpanDescriptor,
        arguments: Arguments<'_>,
    ) -> Self {
        let ctx = ctx.view();
        let f = ctx.vtable.tracing_v0.create_span;
        unsafe {
            f(
                ctx.handle,
                span_descriptor,
                Formatter::format_into_buffer,
                Some(ConstNonNull::new_unchecked(&raw const arguments).cast()),
            )
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let f = self.vtable.drop;
        unsafe { f(self.handle) }
    }
}

handle!(pub handle CallStackHandle: Send + Sync);

/// Virtual function table of a [`CallStack`].
#[repr(C)]
#[derive(Debug)]
pub struct CallStackVTable {
    pub drop: unsafe extern "C" fn(handle: CallStackHandle),
    pub replace_active: unsafe extern "C" fn(handle: CallStackHandle) -> CallStack,
    pub unblock: unsafe extern "C" fn(handle: CallStackHandle),
    pub(crate) _private: PhantomData<()>,
}

impl CallStackVTable {
    cfg_internal! {
        /// Constructs a new `CallStackVTable`.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        pub const fn new(
            drop: unsafe extern "C" fn(handle: CallStackHandle),
            replace_active: unsafe extern "C" fn(handle: CallStackHandle) -> CallStack,
            unblock: unsafe extern "C" fn(handle: CallStackHandle),
        ) -> Self {
            Self {
                drop,
                replace_active,
                unblock,
                _private: PhantomData,
            }
        }
    }
}

/// A call stack.
///
/// Each call stack represents a unit of computation, like a thread. A call stack is active on only
/// one thread at any given time. The active call stack of a thread can be swapped, which is useful
/// for tracing where a `M:N` threading model is used. In that case, one would create one stack for
/// each task, and activate it when the task is resumed.
#[repr(C)]
#[derive(Debug)]
pub struct CallStack {
    pub handle: CallStackHandle,
    pub vtable: VTablePtr<CallStackVTable>,
}

impl CallStack {
    /// Creates a new empty call stack.
    ///
    /// If successful, the new call stack is marked as suspended. The new call stack is not set to
    /// be the active call stack.
    #[inline(always)]
    pub fn new(ctx: impl Viewable<ContextView<'_>>) -> Self {
        let ctx = ctx.view();
        let f = ctx.vtable.tracing_v0.create_call_stack;
        unsafe { f(ctx.handle) }
    }

    /// Switches the call stack of the current thread.
    ///
    /// If successful, this call stack will be used as the active call stack of the calling thread.
    /// The old call stack is returned, enabling the caller to switch back to it afterward. This
    /// call stack must be in a suspended, but unblocked, state and not be active. The active call
    /// stack must also be in a suspended state, but may also be blocked.
    #[inline(always)]
    pub fn switch(self) -> Self {
        let this = ManuallyDrop::new(self);
        let f = this.vtable.replace_active;
        unsafe { f(this.handle) }
    }

    /// Unblocks the blocked call stack.
    ///
    /// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
    /// marked as blocked.
    #[inline(always)]
    pub fn unblock(&mut self) {
        let f = self.vtable.unblock;
        unsafe { f(self.handle) }
    }

    /// Marks the current call stack as being suspended.
    ///
    /// While suspended, the call stack can not be utilized for tracing messages. The call stack
    /// can optionally also be marked as blocked. In that case, the call stack must be unblocked
    /// prior to resumption.
    #[inline(always)]
    pub fn suspend_current(ctx: impl Viewable<ContextView<'_>>, block: bool) {
        let ctx = ctx.view();
        let f = ctx.vtable.tracing_v0.suspend_current_call_stack;
        unsafe { f(ctx.handle, block) }
    }

    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the call stack can be used to trace messages. To be successful, the current
    /// call stack must be suspended and unblocked.
    #[inline(always)]
    pub fn resume_current(ctx: impl Viewable<ContextView<'_>>) {
        let ctx = ctx.view();
        let f = ctx.vtable.tracing_v0.resume_current_call_stack;
        unsafe { f(ctx.handle) }
    }
}

impl Drop for CallStack {
    fn drop(&mut self) {
        let f = self.vtable.drop;
        unsafe { f(self.handle) }
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
    #[inline(always)]
    pub fn new<'a, T: Viewable<ContextView<'a>>>(ctx: T) -> Self {
        unsafe {
            let ctx = ctx.view();
            let f = ctx.vtable().tracing_v0.register_thread.unwrap_unchecked();
            f(ctx.data());
            Self(ctx.to_context())
        }
    }
}

impl Drop for ThreadAccess {
    fn drop(&mut self) {
        let ctx = self.0.view();
        let f = ctx.vtable.tracing_v0.unregister_thread;
        unsafe { f(ctx.handle) }
    }
}

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore, it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
pub trait Subscriber: Send + Sync {
    /// Type of the internal call stack.
    type CallStack: Send + Sync;

    /// Creates a new call stack.
    fn create_call_stack(&self, time: Time) -> Box<Self::CallStack>;

    /// Drops the call stack without tracing anything.
    fn drop_call_stack(&self, call_stack: Box<Self::CallStack>);

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
    );

    /// Drops the span without tracing anything.
    fn drop_span(&self, call_stack: &mut Self::CallStack);

    /// Exits and destroys a span.
    fn destroy_span(&self, time: Time, call_stack: &mut Self::CallStack);

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

handle!(pub handle SubscriberHandle: Send + Sync);
handle!(pub handle SubscriberCallStackHandle: Send + Sync);

/// Virtual function table of a [`OpaqueSubscriber`].
#[repr(C)]
#[derive(Debug)]
pub struct SubscriberVTable {
    pub next: Option<OpaqueHandle<dyn Send + Sync>>,
    pub acquire: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
    pub release: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
    pub create_call_stack: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
    ) -> SubscriberCallStackHandle,
    pub drop_call_stack: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        call_stack: SubscriberCallStackHandle,
    ),
    pub destroy_call_stack: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        call_stack: SubscriberCallStackHandle,
    ),
    pub unblock_call_stack: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        call_stack: SubscriberCallStackHandle,
    ),
    pub suspend_call_stack: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        call_stack: SubscriberCallStackHandle,
        block: bool,
    ),
    pub resume_call_stack: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        call_stack: SubscriberCallStackHandle,
    ),
    pub push_span: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        span_descriptor: &SpanDescriptor,
        message: ConstNonNull<u8>,
        message_length: usize,
        call_stack: SubscriberCallStackHandle,
    ),
    pub drop_span: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        call_stack: SubscriberCallStackHandle,
    ),
    pub pop_span: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        call_stack: SubscriberCallStackHandle,
    ),
    pub emit_event: unsafe extern "C" fn(
        handle: Option<SubscriberHandle>,
        time: &Time,
        call_stack: SubscriberCallStackHandle,
        event: &Event,
        message: ConstNonNull<u8>,
        message_length: usize,
    ),
    pub flush: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
}

/// A type-erased [`Subscriber`].
#[repr(C)]
#[derive(Debug)]
pub struct OpaqueSubscriber {
    pub handle: Option<SubscriberHandle>,
    pub vtable: VTablePtr<SubscriberVTable>,
}

impl OpaqueSubscriber {
    /// Constructs a new `OpaqueSubscriber` from a reference to a [`Subscriber`].
    pub const fn from_ref<T: Subscriber>(subscriber: &'static T) -> Self {
        trait VTableProvider {
            const TABLE: SubscriberVTable;
        }
        impl<T: Subscriber> VTableProvider for T {
            const TABLE: SubscriberVTable =
                OpaqueSubscriber::build_vtable::<T>(acquire_noop::<T>, release_noop::<T>);
        }
        unsafe extern "C" fn acquire_noop<T: Subscriber>(_handle: Option<SubscriberHandle>) {}
        unsafe extern "C" fn release_noop<T: Subscriber>(_handle: Option<SubscriberHandle>) {}

        Self {
            handle: unsafe {
                Some(SubscriberHandle::new_unchecked(
                    (&raw const *subscriber).cast_mut(),
                ))
            },
            vtable: VTablePtr::new(&<T as VTableProvider>::TABLE),
        }
    }

    /// Constructs a new `OpaqueSubscriber` from a [`Subscriber`] in an [`Arc`].
    pub fn from_arc<T: Subscriber>(subscriber: Arc<T>) -> Self {
        trait VTableProvider {
            const TABLE: SubscriberVTable;
        }
        impl<T: Subscriber> VTableProvider for T {
            const TABLE: SubscriberVTable =
                OpaqueSubscriber::build_vtable::<T>(acquire_arc::<T>, release_arc::<T>);
        }
        unsafe extern "C" fn acquire_arc<T: Subscriber>(handle: Option<SubscriberHandle>) {
            unsafe { Arc::increment_strong_count(handle.unwrap_unchecked().as_ptr::<T>()) }
        }
        unsafe extern "C" fn release_arc<T: Subscriber>(handle: Option<SubscriberHandle>) {
            unsafe { Arc::decrement_strong_count(handle.unwrap_unchecked().as_ptr::<T>()) }
        }

        Self {
            handle: unsafe {
                Some(SubscriberHandle::new_unchecked(
                    Arc::into_raw(subscriber).cast_mut(),
                ))
            },
            vtable: VTablePtr::new(&<T as VTableProvider>::TABLE),
        }
    }

    const fn build_vtable<T: Subscriber>(
        acquire_fn: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
        release_fn: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
    ) -> SubscriberVTable {
        unsafe extern "C" fn call_stack_create<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
        ) -> SubscriberCallStackHandle {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let cs = subscriber.create_call_stack(*time);
                SubscriberCallStackHandle::new_unchecked(Box::into_raw(cs))
            }
        }
        unsafe extern "C" fn call_stack_drop<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = Box::from_raw(call_stack.as_ptr());
                subscriber.drop_call_stack(call_stack);
            }
        }
        unsafe extern "C" fn call_stack_destroy<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = Box::from_raw(call_stack.as_ptr());
                subscriber.destroy_call_stack(*time, call_stack);
            }
        }
        unsafe extern "C" fn call_stack_unblock<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.unblock_call_stack(*time, call_stack);
            }
        }
        unsafe extern "C" fn call_stack_suspend<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            call_stack: SubscriberCallStackHandle,
            block: bool,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.suspend_call_stack(*time, call_stack, block);
            }
        }
        unsafe extern "C" fn call_stack_resume<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.resume_call_stack(*time, call_stack);
            }
        }
        unsafe extern "C" fn span_push<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            span_descriptor: &SpanDescriptor,
            message: ConstNonNull<u8>,
            message_length: usize,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let message = core::slice::from_raw_parts(message.as_ptr(), message_length);
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.create_span(*time, span_descriptor, message, call_stack);
            }
        }
        unsafe extern "C" fn span_drop<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.drop_span(call_stack);
            }
        }
        unsafe extern "C" fn span_pop<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            call_stack: SubscriberCallStackHandle,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.destroy_span(*time, call_stack);
            }
        }
        unsafe extern "C" fn event_emit<T: Subscriber>(
            handle: Option<SubscriberHandle>,
            time: &Time,
            call_stack: SubscriberCallStackHandle,
            event: &Event,
            message: ConstNonNull<u8>,
            message_length: usize,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                let message = core::slice::from_raw_parts(message.as_ptr(), message_length);
                subscriber.emit_event(*time, call_stack, event, message);
            }
        }
        unsafe extern "C" fn flush<T: Subscriber>(handle: Option<SubscriberHandle>) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                subscriber.flush();
            }
        }

        SubscriberVTable {
            next: None,
            acquire: acquire_fn,
            release: release_fn,
            create_call_stack: call_stack_create::<T>,
            drop_call_stack: call_stack_drop::<T>,
            destroy_call_stack: call_stack_destroy::<T>,
            unblock_call_stack: call_stack_unblock::<T>,
            suspend_call_stack: call_stack_suspend::<T>,
            resume_call_stack: call_stack_resume::<T>,
            push_span: span_push::<T>,
            drop_span: span_drop::<T>,
            pop_span: span_pop::<T>,
            emit_event: event_emit::<T>,
            flush: flush::<T>,
        }
    }
}

impl Clone for OpaqueSubscriber {
    fn clone(&self) -> Self {
        let f = self.vtable.acquire;
        unsafe { f(self.handle) };
        Self {
            handle: self.handle,
            vtable: self.vtable,
        }
    }
}

impl Drop for OpaqueSubscriber {
    fn drop(&mut self) {
        let f = self.vtable.release;
        unsafe { f(self.handle) }
    }
}

unsafe extern "C" {
    static FIMO_TRACING_DEFAULT_SUBSCRIBER: OpaqueSubscriber;
}

/// Returns the default subscriber.
pub fn default_subscriber() -> OpaqueSubscriber {
    unsafe { FIMO_TRACING_DEFAULT_SUBSCRIBER.clone() }
}

/// Configuration of the tracing subsystem.
#[repr(C)]
pub struct Config<'a> {
    /// # Safety
    ///
    /// Must be [`TypeId::TracingConfig`].
    pub unsafe id: TypeId,
    pub next: Option<OpaqueHandle<dyn Send + Sync + 'a>>,
    pub format_buffer_length: Option<NonZeroUsize>,
    pub max_level: Level,
    /// # Safety
    ///
    /// Represents an [`&[OpaqueSubscriber]`] and must therefore match with the length provided in
    /// `subscriber_count`.
    pub unsafe subscribers: Option<ConstNonNull<OpaqueSubscriber>>,
    /// # Safety
    ///
    /// See `subscribers`.
    pub unsafe subscriber_count: usize,
    pub _phantom: PhantomData<&'a [OpaqueSubscriber]>,
}

impl<'a> Config<'a> {
    /// Creates the default config.
    pub const fn new() -> Self {
        unsafe {
            Self {
                id: TypeId::TracingConfig,
                next: None,
                format_buffer_length: None,
                max_level: if cfg!(debug_assertions) {
                    Level::Debug
                } else {
                    Level::Error
                },
                subscribers: None,
                subscriber_count: 0,
                _phantom: PhantomData,
            }
        }
    }

    /// Sets a custom buffer length.
    pub const fn with_format_buffer_length(mut self, buffer_length: NonZeroUsize) -> Self {
        self.format_buffer_length = Some(buffer_length);
        self
    }

    /// Sets a custom tracing max level.
    pub const fn with_max_level(mut self, max_level: Level) -> Self {
        self.max_level = max_level;
        self
    }

    pub const fn with_subscribers(mut self, subscribers: &'a [OpaqueSubscriber]) -> Self {
        unsafe {
            if subscribers.is_empty() {
                self.subscribers = None;
                self.subscriber_count = 0;
                self
            } else {
                self.subscribers = Some(ConstNonNull::new_unchecked(subscribers.as_ptr()));
                self.subscriber_count = subscribers.len();
                self
            }
        }
    }

    /// Returns a slice of all subscribers.
    pub const fn subscribers(&self) -> &[OpaqueSubscriber] {
        unsafe {
            match self.subscribers {
                None => &[],
                Some(subscribers) => {
                    std::slice::from_raw_parts(subscribers.as_ptr(), self.subscriber_count)
                }
            }
        }
    }
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Copy for Config<'_> {}

#[allow(clippy::expl_impl_clone_on_copy)]
impl Clone for Config<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Debug for Config<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("Config")
                .field("id", &self.id)
                .field("next", &self.next)
                .field("format_buffer_length", &self.format_buffer_length)
                .field("max_level", &self.max_level)
                .field("subscribers", &self.subscribers())
                .finish()
        }
    }
}

struct Formatter<'a> {
    buffer: &'a mut [u8],
    pos: usize,
}

impl Formatter<'_> {
    unsafe fn new(buffer: NonNull<u8>, buffer_len: usize) -> Self {
        unsafe {
            Self {
                buffer: core::slice::from_raw_parts_mut(buffer.as_ptr(), buffer_len),
                pos: 0,
            }
        }
    }

    unsafe extern "C" fn format_into_buffer(
        buffer: NonNull<u8>,
        len: usize,
        data: Option<ConstNonNull<()>>,
        written: &mut MaybeUninit<usize>,
    ) {
        unsafe {
            let mut f = Self::new(buffer, len);
            let _ = f.write_fmt(*data.unwrap_unchecked().as_ptr().cast::<Arguments<'_>>());
            core::ptr::write(written.as_mut_ptr(), f.pos.min(f.buffer.len()));
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
