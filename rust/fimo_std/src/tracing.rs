//! Tracing subsystem.
use crate::{
    context::{ConfigId, Handle},
    handle,
    module::symbols::{AssertSharable, Share, SliceRef, StrRef},
    time::Time,
    utils::{ConstNonNull, OpaqueHandle, Unsafe},
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
    pub create_call_stack: unsafe extern "C" fn() -> CallStack,
    pub suspend_current_call_stack: unsafe extern "C" fn(block: bool),
    pub resume_current_call_stack: unsafe extern "C" fn(),
    pub create_span: unsafe extern "C" fn(
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
        event: &Event,
        formatter: unsafe extern "C" fn(
            buffer: NonNull<u8>,
            len: usize,
            data: Option<ConstNonNull<()>>,
            written: &mut MaybeUninit<usize>,
        ),
        data: Option<ConstNonNull<()>>,
    ),
    pub is_enabled: unsafe extern "C" fn() -> bool,
    pub register_thread: unsafe extern "C" fn(),
    pub unregister_thread: unsafe extern "C" fn(),
    pub flush: unsafe extern "C" fn(),
}

/// Emits a new event.
///
/// The message may be cut of, if the length exceeds the internal formatting buffer size.
#[inline(always)]
pub fn emit_event(event: &Event, arguments: Arguments<'_>) {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.tracing_v0.emit_event;
    unsafe {
        f(
            event,
            Formatter::format_into_buffer,
            Some(ConstNonNull::new_unchecked(&raw const arguments).cast()),
        );
    }
}

/// Checks whether the tracing subsystem is enabled.
///
/// This function can be used to check whether to call into the subsystem at all. Calling this
/// function is not necessary, as the remaining functions of the backend are guaranteed to
/// return default values, in case the backend is disabled.
#[inline(always)]
pub fn is_enabled() -> bool {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.tracing_v0.is_enabled;
    unsafe { f() }
}

/// Flushes the streams used for tracing.
///
/// If successful, any unwritten data is written out by the individual subscribers.
#[inline(always)]
pub fn flush() {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.tracing_v0.flush;
    unsafe { f() }
}

/// Constructs a new [`Span`].
#[macro_export]
macro_rules! tracing_span {
    (name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        {
            const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
                name: $name,
                target: $target,
                lvl: $lvl
            );
            const DESCRIPTOR: &'static $crate::tracing::SpanDescriptor =
                &$crate::tracing::SpanDescriptor::new(METADATA);
            $crate::tracing::Span::new(DESCRIPTOR, core::format_args!($($arg)+))
        }
    };
    (target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        {
            const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
                target: $target,
                lvl: $lvl
            );
            const DESCRIPTOR: &'static $crate::tracing::SpanDescriptor =
                &$crate::tracing::SpanDescriptor::new(METADATA);
            $crate::tracing::Span::new(DESCRIPTOR, core::format_args!($($arg)+))
        };
    };
    (lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::tracing_span!(target: "", lvl: $lvl, $($arg)+)
    };
}

/// Emits a new [`Event`].
#[macro_export]
macro_rules! tracing_emit {
    (name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        use $crate::tracing::TracingSubsystem;
        const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
            name: $name,
            target: $target,
            lvl: $lvl
        );
        const EVENT: &'static $crate::tracing::Event = &$crate::tracing::Event::new(METADATA);
        $crate::tracing::emit_event(EVENT, core::format_args!($($arg)+));
    }};
    (target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        const METADATA: &'static $crate::tracing::Metadata = $crate::tracing_metadata!(
            target: $target,
            lvl: $lvl
        );
        const EVENT: &'static $crate::tracing::Event = &$crate::tracing::Event::new(METADATA);
        $crate::tracing::emit_event(EVENT, core::format_args!($($arg)+));
    }};
    (lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::tracing_emit!(target: "", lvl: $lvl, $($arg)+)
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
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(name: $name, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_emit!(lvl: $crate::tracing::Level::Error, $($arg)+);
    };
}

/// Emits a new [`Level::Warn`] event.
#[macro_export]
macro_rules! emit_warn {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(name: $name, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_emit!(lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
}

/// Emits a new [`Level::Info`] event.
#[macro_export]
macro_rules! emit_info {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(name: $name, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_emit!(lvl: $crate::tracing::Level::Info, $($arg)+);
    };
}

/// Emits a new [`Level::Debug`] event.
#[macro_export]
macro_rules! emit_debug {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(name: $name, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_emit!(lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
}

/// Emits a new [`Level::Trace`] event.
#[macro_export]
macro_rules! emit_trace {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(name: $name, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_emit!(target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_emit!(lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
}

/// Constructs a new [`Level::Error`] span.
#[macro_export]
macro_rules! span_error {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(name: $name, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_span!(lvl: $crate::tracing::Level::Error, $($arg)+);
    };
}

/// Constructs a new [`Level::Warn`] span.
#[macro_export]
macro_rules! span_warn {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(name: $name, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_span!(lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
}

/// Constructs a new [`Level::Info`] span.
#[macro_export]
macro_rules! span_info {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(name: $name, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_span!(lvl: $crate::tracing::Level::Info, $($arg)+);
    };
}

/// Constructs a new [`Level::Debug`] span.
#[macro_export]
macro_rules! span_debug {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(name: $name, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_span!(lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
}

/// Constructs a new [`Level::Trace`] span.
#[macro_export]
macro_rules! span_trace {
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(name: $name, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::tracing_span!(target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::tracing_span!(lvl: $crate::tracing::Level::Trace, $($arg)+);
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
    pub next: Option<OpaqueHandle<dyn Send + Sync + Share>>,
    pub name: StrRef<'static>,
    pub target: StrRef<'static>,
    pub level: Level,
    pub file_name: Option<StrRef<'static>>,
    pub line_number: i32,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Metadata: Send, Sync, Share);

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
            name: StrRef::new(name),
            target: StrRef::new(target),
            level,
            file_name: match file_name {
                None => None,
                Some(x) => Some(StrRef::new(x)),
            },
            line_number: match line_number {
                None => -1,
                Some(x) => {
                    assert!(x <= i32::MAX as u32);
                    x as i32
                }
            },
            _private: PhantomData,
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
    pub next: Option<OpaqueHandle<dyn Send + Sync + Share>>,
    pub metadata: &'static Metadata,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Event: Send, Sync, Share);

impl Event {
    /// Constructs a new event.
    pub const fn new(metadata: &'static Metadata) -> Self {
        Self {
            next: None,
            metadata,
            _private: PhantomData,
        }
    }
}

/// Descriptor of a new span.
#[repr(C)]
#[derive(Debug)]
pub struct SpanDescriptor {
    pub next: Option<OpaqueHandle<dyn Send + Sync + Share>>,
    pub metadata: &'static Metadata,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(SpanDescriptor: Send, Sync, Share);

impl SpanDescriptor {
    /// Constructs a new `SpanDescriptor`.
    pub const fn new(metadata: &'static Metadata) -> Self {
        Self {
            next: None,
            metadata,
            _private: PhantomData,
        }
    }
}

handle!(pub handle SpanHandle: Send + Sync + Share);

/// Virtual function table of a [`Span`].
#[repr(C)]
#[derive(Debug)]
pub struct SpanVTable {
    pub drop: unsafe extern "C" fn(handle: SpanHandle),
    pub drop_unwind: unsafe extern "C" fn(handle: SpanHandle),
    _private: PhantomData<()>,
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
        pub const fn new(
            drop: unsafe extern "C" fn(handle: SpanHandle),
            drop_unwind: unsafe extern "C" fn(handle: SpanHandle),
        ) -> Self {
            Self {
                drop,
                drop_unwind,
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
    pub vtable: &'static AssertSharable<SpanVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Span: Send, Sync, Share);

impl Span {
    /// Creates a new span and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message may be cut of, if the length exceeds the internal formatting buffer size.
    pub fn new(span_descriptor: &'static SpanDescriptor, arguments: Arguments<'_>) -> Self {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.create_span;
        unsafe {
            f(
                span_descriptor,
                Formatter::format_into_buffer,
                Some(ConstNonNull::new_unchecked(&raw const arguments).cast()),
            )
        }
    }

    /// Unwinds and destroys a span.
    ///
    /// The events won't occur inside the context of the exited span anymore. The span must be the
    /// span at the top of the current call stack. The span may not be in use prior to a call to
    /// this function, and may not be used afterwards.
    ///
    /// This function must be called while the owning call stack is bound by the current thread.
    #[inline(always)]
    pub fn drop_unwind(self) {
        let this = ManuallyDrop::new(self);
        let f = this.vtable.drop_unwind;
        unsafe { f(this.handle) }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let f = self.vtable.drop;
        unsafe { f(self.handle) }
    }
}

handle!(pub handle CallStackHandle: Send + Sync + Share);

/// Virtual function table of a [`CallStack`].
#[repr(C)]
#[derive(Debug)]
pub struct CallStackVTable {
    pub drop: unsafe extern "C" fn(handle: CallStackHandle),
    pub drop_unwind: unsafe extern "C" fn(handle: CallStackHandle),
    pub replace_active: unsafe extern "C" fn(handle: CallStackHandle) -> CallStack,
    pub unblock: unsafe extern "C" fn(handle: CallStackHandle),
    _private: PhantomData<()>,
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
            drop_unwind: unsafe extern "C" fn(handle: CallStackHandle),
            replace_active: unsafe extern "C" fn(handle: CallStackHandle) -> CallStack,
            unblock: unsafe extern "C" fn(handle: CallStackHandle),
        ) -> Self {
            Self {
                drop,
                drop_unwind,
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
    pub vtable: &'static AssertSharable<CallStackVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(CallStack: Send, Sync, Share);

impl CallStack {
    /// Creates a new empty call stack.
    ///
    /// If successful, the new call stack is marked as suspended. The new call stack is not set to
    /// be the active call stack.
    #[inline(always)]
    pub fn new() -> Self {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.create_call_stack;
        unsafe { f() }
    }

    /// Unwinds and destroys a call stack.
    ///
    /// Marks that the task was aborted. Before calling this function, the call stack  must not be
    /// active. If successful, the call stack may not be used afterwards. The caller must own the
    /// call stack uniquely.
    #[inline(always)]
    pub fn drop_unwind(self) {
        let this = ManuallyDrop::new(self);
        let f = this.vtable.drop_unwind;
        unsafe { f(this.handle) }
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
    pub fn suspend_current(block: bool) {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.suspend_current_call_stack;
        unsafe { f(block) }
    }

    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the call stack can be used to trace messages. To be successful, the current
    /// call stack must be suspended and unblocked.
    #[inline(always)]
    pub fn resume_current() {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.resume_current_call_stack;
        unsafe { f() }
    }
}

impl Default for CallStack {
    fn default() -> Self {
        Self::new()
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
pub struct ThreadAccess(PhantomData<()>);

sa::assert_impl_all!(ThreadAccess: Send, Sync, Share);

impl ThreadAccess {
    /// Registers the calling thread with the tracing subsystem.
    ///
    /// The tracing of the subsystem is opt-in on a per-thread basis, where unregistered threads
    /// will behave as if the backend was disabled. Once registered, the calling thread gains access
    /// to the tracing subsystem and is assigned a new empty call stack.
    #[inline(always)]
    pub fn new() -> Self {
        unsafe {
            let handle = Handle::get_handle();
            let f = handle.tracing_v0.register_thread;
            f();
            Self(PhantomData)
        }
    }
}

impl Default for ThreadAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ThreadAccess {
    fn drop(&mut self) {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.unregister_thread;
        unsafe { f() }
    }
}

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore, it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
pub trait Subscriber: Send + Sync + Share {
    /// Type of the internal call stack.
    type CallStack: Send + Sync + Share;

    /// Creates a new call stack.
    fn create_call_stack(&self, time: Time) -> Box<Self::CallStack>;

    /// Drops the call stack without tracing anything.
    fn drop_call_stack(&self, call_stack: Box<Self::CallStack>);

    /// Destroys the call stack.
    fn destroy_call_stack(&self, time: Time, call_stack: Box<Self::CallStack>, is_unwind: bool);

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
    fn destroy_span(&self, time: Time, call_stack: &mut Self::CallStack, is_unwind: bool);

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

handle!(pub handle SubscriberHandle: Send + Sync + Share);
handle!(pub handle SubscriberCallStackHandle: Send + Sync + Share);

/// Virtual function table of a [`OpaqueSubscriber`].
#[repr(C)]
#[derive(Debug)]
pub struct SubscriberVTable {
    pub next: Option<OpaqueHandle<dyn Send + Sync + Share>>,
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
        is_unwind: bool,
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
        is_unwind: bool,
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
    pub vtable: &'static AssertSharable<SubscriberVTable>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(OpaqueSubscriber: Send, Sync, Share);

impl OpaqueSubscriber {
    /// Constructs a new `OpaqueSubscriber` from a reference to a [`Subscriber`].
    pub const fn from_ref<T: Subscriber>(subscriber: &'static T) -> Self {
        trait VTableProvider {
            const TABLE: AssertSharable<SubscriberVTable>;
        }
        impl<T: Subscriber> VTableProvider for T {
            const TABLE: AssertSharable<SubscriberVTable> =
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
            vtable: &<T as VTableProvider>::TABLE,
            _private: PhantomData,
        }
    }

    /// Constructs a new `OpaqueSubscriber` from a [`Subscriber`] in an [`Arc`].
    pub fn from_arc<T: Subscriber>(subscriber: Arc<T>) -> Self {
        trait VTableProvider {
            const TABLE: AssertSharable<SubscriberVTable>;
        }
        impl<T: Subscriber> VTableProvider for T {
            const TABLE: AssertSharable<SubscriberVTable> =
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
            vtable: &<T as VTableProvider>::TABLE,
            _private: PhantomData,
        }
    }

    const fn build_vtable<T: Subscriber>(
        acquire_fn: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
        release_fn: unsafe extern "C" fn(handle: Option<SubscriberHandle>),
    ) -> AssertSharable<SubscriberVTable> {
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
            is_unwind: bool,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = Box::from_raw(call_stack.as_ptr());
                subscriber.destroy_call_stack(*time, call_stack, is_unwind);
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
            is_unwind: bool,
        ) {
            unsafe {
                let subscriber: &T =
                    &*handle.map_or(std::ptr::null(), |x| x.as_ptr::<T>().cast_const());
                let call_stack = &mut *call_stack.as_ptr();
                subscriber.destroy_span(*time, call_stack, is_unwind);
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

        unsafe {
            AssertSharable::new(SubscriberVTable {
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
            })
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
            _private: PhantomData,
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
#[derive(Debug, Copy, Clone)]
pub struct Config<'a> {
    /// # Safety
    ///
    /// Must be [`ConfigId::TracingConfig`].
    pub id: Unsafe<ConfigId>,
    pub format_buffer_length: Option<NonZeroUsize>,
    pub max_level: Level,
    pub subscribers: SliceRef<'a, OpaqueSubscriber>,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Config<'_>: Send, Sync);

impl<'a> Config<'a> {
    /// Creates the default config.
    pub const fn new() -> Self {
        unsafe {
            Self {
                id: Unsafe::new(ConfigId::TracingConfig),
                format_buffer_length: None,
                max_level: if cfg!(debug_assertions) {
                    Level::Debug
                } else {
                    Level::Error
                },
                subscribers: SliceRef::new(&[]),
                _private: PhantomData,
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

    /// Sets a custom list of subscribers.
    pub const fn with_subscribers(mut self, subscribers: &'a [OpaqueSubscriber]) -> Self {
        self.subscribers = SliceRef::new(subscribers);
        self
    }

    /// Returns a slice of all subscribers.
    pub const fn subscribers(&self) -> &[OpaqueSubscriber] {
        self.subscribers.as_slice()
    }
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self::new()
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
