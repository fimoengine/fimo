//! Tracing subsystem.
use crate::{
    context::{ConfigId, Handle},
    handle,
    modules::symbols::{AssertSharable, Share, SliceRef, StrRef},
    utils::{ConstNonNull, OpaqueHandle, Unsafe},
};
use std::{
    ffi::CStr,
    fmt::{Arguments, Debug, Write},
    marker::PhantomData,
    mem::ManuallyDrop,
    num::NonZeroUsize,
    ptr::NonNull,
};

/// Virtual function table of the tracing subsystem.
#[repr(C)]
#[derive(Debug)]
pub struct VTableV0 {
    pub is_enabled: unsafe extern "C" fn() -> bool,
    pub register_thread: unsafe extern "C" fn(),
    pub unregister_thread: unsafe extern "C" fn(),
    pub create_call_stack: unsafe extern "C" fn() -> CallStackHandle,
    pub destroy_call_stack: unsafe extern "C" fn(stack: CallStackHandle, abort: bool),
    pub swap_call_stack: unsafe extern "C" fn(stack: CallStackHandle) -> CallStackHandle,
    pub unblock_call_stack: unsafe extern "C" fn(stack: CallStackHandle),
    pub suspend_current_call_stack: unsafe extern "C" fn(mark_blocked: bool),
    pub resume_current_call_stack: unsafe extern "C" fn(),
    pub enter_span: unsafe extern "C" fn(
        id: &'static EventInfo,
        formatter: unsafe extern "C" fn(
            buffer: NonNull<u8>,
            len: usize,
            data: Option<ConstNonNull<()>>,
        ) -> usize,
        formatter_data: Option<ConstNonNull<()>>,
    ),
    pub exit_span: unsafe extern "C" fn(id: &'static EventInfo),
    pub log_message: unsafe extern "C" fn(
        info: &'static EventInfo,
        formatter: unsafe extern "C" fn(
            buffer: NonNull<u8>,
            len: usize,
            data: Option<ConstNonNull<()>>,
        ) -> usize,
        formatter_data: Option<ConstNonNull<()>>,
    ),
}

/// Checks whether the tracing subsystem is enabled.
///
/// This function can be used to check whether to call into the subsystem at all. Calling this
/// function is not necessary, as the remaining functions of the subsystem are guaranteed to return
/// default values, in case the subsystem is disabled.
#[inline(always)]
pub fn is_enabled() -> bool {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.tracing_v0.is_enabled;
    unsafe { f() }
}

/// Emits a new event.
///
/// The message may be cut of, if the length exceeds the internal formatting buffer size.
#[inline(always)]
pub fn log_message(info: &'static EventInfo, arguments: Arguments<'_>) {
    let handle = unsafe { Handle::get_handle() };
    let f = handle.tracing_v0.log_message;
    unsafe {
        f(
            info,
            Formatter::format_into_buffer,
            Some(ConstNonNull::new_unchecked(&raw const arguments).cast()),
        );
    }
}

/// Constructs a new [`Span`].
#[macro_export]
macro_rules! span {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        {
            const INFO: &'static $crate::tracing::EventInfo = $crate::tracing_event_info!(
                name: $name,
                target: $target,
                scope: $scope,
                lvl: $lvl
            );
            const SPAN: &'static $crate::tracing::Span = &$crate::tracing::Span::new(INFO);
            SPAN.enter(core::format_args!($($arg)+))
        }
    };
    (name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, scope: "", lvl: $lvl, $($arg)+)
    };
    (target: $target:literal, scope: $scope:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        {
            const INFO: &'static $crate::tracing::EventInfo = $crate::tracing_event_info!(
                target: $target,
                scope: $scope,
                lvl: $lvl
            );
            const SPAN: &'static $crate::tracing::Span = &$crate::tracing::Span::new(INFO);
            SPAN.enter(core::format_args!($($arg)+))
        };
    };
    (target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::span!(target: $target, scope: "", lvl: $lvl, $($arg)+)
    };
    (scope: $scope:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::span!(target: "", scope: $scope, lvl: $lvl, $($arg)+)
    };
    (lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::span!(target: "", scope: "", lvl: $lvl, $($arg)+)
    };
}

/// Logs a message.
#[macro_export]
macro_rules! log {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        const INFO: &'static $crate::tracing::EventInfo = $crate::tracing_event_info!(
            name: $name,
            target: $target,
            scope: $scope,
            lvl: $lvl
        );
        $crate::tracing::log_message(INFO, core::format_args!($($arg)+));
    }};
    (name: $name:literal, target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, scope: "", lvl: $lvl, $($arg)+)
    };
    (target: $target:literal, scope: $scope:literal, lvl: $lvl:expr, $($arg:tt)+) => {{
        const INFO: &'static $crate::tracing::EventInfo = $crate::tracing_event_info!(
            target: $target,
            scope: $scope,
            lvl: $lvl
        );
        $crate::tracing::log_message(INFO, core::format_args!($($arg)+));
    }};
    (target: $target:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::log!(target: $target, scope: "", lvl: $lvl, $($arg)+)
    };
    (scope: $scope:literal, lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::log!(target: "", scope: $scope, lvl: $lvl, $($arg)+)
    };
    (lvl: $lvl:expr, $($arg:tt)+) => {
        $crate::log!(scope: "", lvl: $lvl, $($arg)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! tracing_event_info {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, lvl: $lvl:expr) => {{
        const NAME: &'static str = core::concat!($name, '\0');
        const TARGET: &'static str = core::concat!($target, '\0');
        const SCOPE: &'static str = core::concat!($scope, '\0');
        const FILE: &'static str = core::concat!(core::file!(), '\0');
        const LINE: u32 = core::line!() as u32;

        const NAME_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(NAME.as_bytes()) };
        const TARGET_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(TARGET.as_bytes()) };
        const SCOPE_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(SCOPE.as_bytes()) };
        const FILE_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(FILE.as_bytes()) };

        const INFO: &'static $crate::tracing::EventInfo = &$crate::tracing::EventInfo::new(
            NAME_CSTR,
            TARGET_CSTR,
            FILE_CSTR,
            Some(FILE_CSTR),
            Some(LINE),
            $lvl,
        );
        INFO
    }};
    (name: $name:literal, target: $target:literal, lvl: $lvl:expr) => {{
        $crate::tracing_event_info!(name: $name, target: $target, scope: "", lvl: $lvl, $($arg)+)
    }};
    (target: $target:literal, scope: $scope:literal, lvl: $lvl:expr) => {{
        const NAME: &'static str = core::concat!(core::module_path!(), '\0');
        const TARGET: &'static str = core::concat!($target, '\0');
        const SCOPE: &'static str = core::concat!($scope, '\0');
        const FILE: &'static str = core::concat!(core::file!(), '\0');
        const LINE: u32 = core::line!() as u32;

        const NAME_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(NAME.as_bytes()) };
        const TARGET_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(TARGET.as_bytes()) };
        const SCOPE_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(SCOPE.as_bytes()) };
        const FILE_CSTR: &'static core::ffi::CStr =
            unsafe { core::ffi::CStr::from_bytes_with_nul_unchecked(FILE.as_bytes()) };

        const INFO: &'static $crate::tracing::EventInfo = &$crate::tracing::EventInfo::new(
            NAME_CSTR,
            TARGET_CSTR,
            SCOPE_CSTR,
            Some(FILE_CSTR),
            Some(LINE),
            $lvl,
        );
        INFO
    }};
    (target: $target:literal, lvl: $lvl:expr) => {{
        $crate::tracing_event_info!(target: $target, scope: "", lvl: $lvl, $($arg)+)
    }};
}

/// Emits a new [`Level::Error`] message.
#[macro_export]
macro_rules! log_error {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(scope: $scope, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::log!(lvl: $crate::tracing::Level::Error, $($arg)+);
    };
}

/// Emits a new [`Level::Warn`] message.
#[macro_export]
macro_rules! log_warn {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(scope: $scope, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::log!(lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
}

/// Emits a new [`Level::Info`] message.
#[macro_export]
macro_rules! log_info {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(scope: $scope, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::log!(lvl: $crate::tracing::Level::Info, $($arg)+);
    };
}

/// Emits a new [`Level::Debug`] message.
#[macro_export]
macro_rules! log_debug {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(scope: $scope, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::log!(lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
}

/// Emits a new [`Level::Trace`] message.
#[macro_export]
macro_rules! log_trace {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::log!(name: $name, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::log!(target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::log!(scope: $scope, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::log!(lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
}

/// Constructs a new [`Level::Error`] span.
#[macro_export]
macro_rules! span_error {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(scope: $scope, lvl: $crate::tracing::Level::Error, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::span!(lvl: $crate::tracing::Level::Error, $($arg)+);
    };
}

/// Constructs a new [`Level::Warn`] span.
#[macro_export]
macro_rules! span_warn {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(scope: $scope, lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::span!(lvl: $crate::tracing::Level::Warn, $($arg)+);
    };
}

/// Constructs a new [`Level::Info`] span.
#[macro_export]
macro_rules! span_info {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(scope: $scope, lvl: $crate::tracing::Level::Info, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::span!(lvl: $crate::tracing::Level::Info, $($arg)+);
    };
}

/// Constructs a new [`Level::Debug`] span.
#[macro_export]
macro_rules! span_debug {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(scope: $scope, lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::span!(lvl: $crate::tracing::Level::Debug, $($arg)+);
    };
}

/// Constructs a new [`Level::Trace`] span.
#[macro_export]
macro_rules! span_trace {
    (name: $name:literal, target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, scope: $scope, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (name: $name:literal, target: $target:literal, $($arg:tt)+) => {
        $crate::span!(name: $name, target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (target: $target:literal, scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, scope: $scope, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (target: $target:literal, $($arg:tt)+) => {
        $crate::span!(target: $target, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    (scope: $scope:literal, $($arg:tt)+) => {
        $crate::span!(scope: $scope, lvl: $crate::tracing::Level::Trace, $($arg)+);
    };
    ($($arg:tt)+) => {
        $crate::span!(lvl: $crate::tracing::Level::Trace, $($arg)+);
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

/// Basic information regarding a tracing event.
#[repr(C)]
#[derive(Debug)]
pub struct EventInfo {
    pub name: StrRef<'static>,
    pub target: StrRef<'static>,
    pub scope: StrRef<'static>,
    pub file_name: Option<StrRef<'static>>,
    pub line_number: i32,
    pub level: Level,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(EventInfo: Send, Sync, Share);

impl EventInfo {
    pub const fn new(
        name: &'static CStr,
        target: &'static CStr,
        scope: &'static CStr,
        file_name: Option<&'static CStr>,
        line_number: Option<u32>,
        level: Level,
    ) -> EventInfo {
        Self {
            name: StrRef::new(name),
            target: StrRef::new(target),
            scope: StrRef::new(scope),
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
            level,
            _private: PhantomData,
        }
    }

    /// Returns the name contained in the `EventInfo`.
    pub fn name(&self) -> &CStr {
        unsafe { self.name.as_ref() }
    }

    /// Returns the target contained in the `EventInfo`.
    pub fn target(&self) -> &CStr {
        unsafe { self.target.as_ref() }
    }

    /// Returns the scope contained in the `EventInfo`.
    pub fn scope(&self) -> &CStr {
        unsafe { self.scope.as_ref() }
    }

    /// Returns the file name contained in the `EventInfo`.
    pub fn file_name(&self) -> Option<&CStr> {
        unsafe { self.file_name.map(|x| x.as_ref()) }
    }

    /// Returns the file number contained in the `EventInfo`.
    pub fn line_number(&self) -> Option<u32> {
        if self.line_number < 0 {
            None
        } else {
            Some(self.line_number as u32)
        }
    }
}

/// Subscriber events.
pub mod events {
    use std::mem::offset_of;

    use super::SubscriberCallStackHandle;
    use crate::{
        modules::symbols::SliceRef, time::Instant, tracing::EventInfo, utils::ConstNonNull,
    };

    /// Common header of all events.
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct Event(u32);

    impl Event {
        pub const REGISTER_EVENT: Self = Self(0);
        pub const UNREGISTER_THREAD: Self = Self(1);
        pub const CREATE_CALL_STACK: Self = Self(2);
        pub const DESTROY_CALL_STACK: Self = Self(3);
        pub const UNBLOCK_CALL_STACK: Self = Self(4);
        pub const SUSPEND_CALL_STACK: Self = Self(5);
        pub const RESUME_CALL_STACK: Self = Self(6);
        pub const ENTER_SPAN: Self = Self(7);
        pub const EXIT_SPAN: Self = Self(8);
        pub const LOG_MESSAGE: Self = Self(9);

        pub(crate) unsafe fn as_enum<'a>(event: ConstNonNull<Self>) -> EventEnum<'a> {
            unsafe {
                match event.as_ptr().read() {
                    Self::REGISTER_EVENT => {
                        let ptr: *const RegisterThread = event
                            .as_ptr()
                            .byte_sub(offset_of!(RegisterThread, event))
                            .cast();
                        EventEnum::RegisterThread(&*ptr)
                    }
                    Self::UNREGISTER_THREAD => {
                        let ptr: *const UnregisterThread = event
                            .as_ptr()
                            .byte_sub(offset_of!(UnregisterThread, event))
                            .cast();
                        EventEnum::UnregisterThread(&*ptr)
                    }
                    Self::CREATE_CALL_STACK => {
                        let ptr: *const CreateCallStack = event
                            .as_ptr()
                            .byte_sub(offset_of!(CreateCallStack, event))
                            .cast();
                        EventEnum::CreateCallStack(&*ptr)
                    }
                    Self::DESTROY_CALL_STACK => {
                        let ptr: *const DestroyCallStack = event
                            .as_ptr()
                            .byte_sub(offset_of!(DestroyCallStack, event))
                            .cast();
                        EventEnum::DestroyCallStack(&*ptr)
                    }
                    Self::UNBLOCK_CALL_STACK => {
                        let ptr: *const UnblockCallStack = event
                            .as_ptr()
                            .byte_sub(offset_of!(UnblockCallStack, event))
                            .cast();
                        EventEnum::UnblockCallStack(&*ptr)
                    }
                    Self::SUSPEND_CALL_STACK => {
                        let ptr: *const SuspendCallStack = event
                            .as_ptr()
                            .byte_sub(offset_of!(SuspendCallStack, event))
                            .cast();
                        EventEnum::SuspendCallStack(&*ptr)
                    }
                    Self::RESUME_CALL_STACK => {
                        let ptr: *const ResumeCallStack = event
                            .as_ptr()
                            .byte_sub(offset_of!(ResumeCallStack, event))
                            .cast();
                        EventEnum::ResumeCallStack(&*ptr)
                    }
                    Self::ENTER_SPAN => {
                        let ptr: *const EnterSpan<'_> = event
                            .as_ptr()
                            .byte_sub(offset_of!(EnterSpan<'_>, event))
                            .cast();
                        EventEnum::EnterSpan(&*ptr)
                    }
                    Self::EXIT_SPAN => {
                        let ptr: *const ExitSpan =
                            event.as_ptr().byte_sub(offset_of!(ExitSpan, event)).cast();
                        EventEnum::ExitSpan(&*ptr)
                    }
                    Self::LOG_MESSAGE => {
                        let ptr: *const LogMessage<'_> = event
                            .as_ptr()
                            .byte_sub(offset_of!(LogMessage<'_>, event))
                            .cast();
                        EventEnum::LogMessage(&*ptr)
                    }
                    _ => EventEnum::Unknown,
                }
            }
        }
    }

    /// Enum containing all known events.
    #[non_exhaustive]
    #[derive(Debug, Copy, Clone)]
    pub enum EventEnum<'a> {
        RegisterThread(&'a RegisterThread),
        UnregisterThread(&'a UnregisterThread),
        CreateCallStack(&'a CreateCallStack),
        DestroyCallStack(&'a DestroyCallStack),
        UnblockCallStack(&'a UnblockCallStack),
        SuspendCallStack(&'a SuspendCallStack),
        ResumeCallStack(&'a ResumeCallStack),
        EnterSpan(&'a EnterSpan<'a>),
        ExitSpan(&'a ExitSpan),
        LogMessage(&'a LogMessage<'a>),
        Unknown,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct RegisterThread {
        event: Event,
        time: Instant,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct UnregisterThread {
        event: Event,
        time: Instant,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct CreateCallStack {
        event: Event,
        time: Instant,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct DestroyCallStack {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct UnblockCallStack {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct SuspendCallStack {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
        mark_blocked: bool,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct ResumeCallStack {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct EnterSpan<'a> {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
        span: &'static EventInfo,
        message: SliceRef<'a, u8>,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct ExitSpan {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
        span: &'static EventInfo,
        is_unwinding: bool,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct LogMessage<'a> {
        event: Event,
        stack: SubscriberCallStackHandle,
        time: Instant,
        info: &'static EventInfo,
        message: SliceRef<'a, u8>,
    }
}

/// A period of time, during which events can occur.
#[derive(Debug)]
pub struct Span {
    pub id: &'static EventInfo,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(Span: Send, Sync, Share);

impl Span {
    /// Constructs a new span.
    pub const fn new(id: &'static EventInfo) -> Self {
        Self {
            id,
            _private: PhantomData,
        }
    }

    /// Enters the span.
    ///
    /// Once entered, the span is used as the context for succeeding events. A span may be entered
    /// multiple times.
    pub fn enter(&'static self, arguments: Arguments<'_>) -> SpanGuard {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.enter_span;
        unsafe {
            f(
                self.id,
                Formatter::format_into_buffer,
                Some(ConstNonNull::new_unchecked(&raw const arguments).cast()),
            );
        }
        SpanGuard { span: self }
    }

    /// Exits an entered span.
    ///
    /// The events won't occur inside the context of the exited span anymore. The span must be the
    /// span at the top of the current call stack.
    pub fn exit(&'static self) {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.exit_span;
        unsafe {
            f(self.id);
        }
    }
}

/// RAII-guard of an entered span.
pub struct SpanGuard {
    span: &'static Span,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        self.span.exit();
    }
}

handle!(pub handle CallStackHandle: Send + Sync + Share);

/// A call stack.
///
/// Each call stack represents a unit of computation, like a thread. A call stack is active on only
/// one thread at any given time. The active call stack of a thread can be swapped, which is useful
/// for tracing where a `M:N` threading model is used. In that case, one would create one stack for
/// each task, and activate it when the task is resumed.
#[derive(Debug)]
#[repr(transparent)]
pub struct CallStack {
    pub handle: CallStackHandle,
    _private: PhantomData<()>,
}

sa::assert_impl_all!(CallStack: Send, Sync, Share);

impl CallStack {
    /// Creates a new empty call stack.
    ///
    /// The call stack is marked as suspended.
    #[inline(always)]
    pub fn new() -> Self {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.create_call_stack;
        let stack = unsafe { f() };
        Self {
            handle: stack,
            _private: PhantomData,
        }
    }

    /// Unwinds and destroys the call stack.
    ///
    /// Marks that the task was aborted. Before calling this function, the call stack must not be
    /// active. The call stack may not be used afterwards.
    #[inline(always)]
    pub fn abort(self) {
        let this = ManuallyDrop::new(self);
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.destroy_call_stack;
        unsafe { f(this.handle, true) }
    }

    /// Switches the call stack of the current thread.
    ///
    /// This call stack will be used as the active call stack of the calling thread. The old call
    /// stack is returned, enabling the caller to switch back to it afterwards. This call stack
    /// must be in a suspended, but unblocked, state and not be active. The active call stack must
    /// also be in a suspended state, but may also be blocked.
    #[inline(always)]
    pub fn switch(self) -> Self {
        let this = ManuallyDrop::new(self);
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.swap_call_stack;
        let stack = unsafe { f(this.handle) };
        Self {
            handle: stack,
            _private: PhantomData,
        }
    }

    /// Unblocks the blocked call stack.
    ///
    /// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
    /// marked as blocked.
    #[inline(always)]
    pub fn unblock(&mut self) {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.unblock_call_stack;
        unsafe { f(self.handle) }
    }

    /// Marks the current call stack as being suspended.
    ///
    /// While suspended, the call stack can not be utilized for tracing messages. The call stack
    /// optionally also be marked as being blocked. In that case, the call stack must be unblocked
    /// prior to resumption.
    #[inline(always)]
    pub fn suspend_current(block: bool) {
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.suspend_current_call_stack;
        unsafe { f(block) }
    }

    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the context can be used to trace messages. To be successful, the current call
    /// stack must be suspended and unblocked.
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
        let handle = unsafe { Handle::get_handle() };
        let f = handle.tracing_v0.destroy_call_stack;
        unsafe { f(self.handle, false) }
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
    /// Registers the current thread.
    fn register_thread(&self, _event: &events::RegisterThread) {}

    /// Unregisters the current thread.
    fn unregister_thread(&self, _event: &events::UnregisterThread) {}

    /// Creates a new call stack.
    fn create_call_stack(&self, _event: &events::CreateCallStack) -> CallStackHandle {
        CallStackHandle::new(&mut ()).unwrap()
    }

    /// Destroys the call stack.
    fn destroy_call_stack(&self, _event: &events::DestroyCallStack) {}

    /// Marks the call stack as being unblocked.
    fn unblock_call_stack(&self, _event: &events::UnblockCallStack) {}

    /// Marks the stack as being suspended/blocked.
    fn suspend_call_stack(&self, _event: &events::SuspendCallStack) {}

    /// Marks the stack as being resumed.
    fn resume_call_stack(&self, _event: &events::ResumeCallStack) {}

    /// Enters the span.
    fn enter_span(&self, _event: &events::EnterSpan<'_>) {}

    /// Exits the span.
    fn exit_span(&self, _event: &events::ExitSpan) {}

    /// Emits an event.
    fn log_message(&self, _event: &events::LogMessage<'_>) {}
}

handle!(pub handle SubscriberCallStackHandle: Send + Sync + Share);

/// A type-erased [`Subscriber`].
#[repr(C)]
#[derive(Debug)]
pub struct OpaqueSubscriber<'a> {
    pub handle: AssertSharable<OpaqueHandle<dyn Subscriber>>,
    pub on_event: AssertSharable<
        unsafe extern "C" fn(
            handle: OpaqueHandle<dyn Subscriber>,
            event: ConstNonNull<events::Event>,
        ) -> NonNull<()>,
    >,
    _private: PhantomData<&'a ()>,
}

sa::assert_impl_all!(OpaqueSubscriber<'_>: Send, Sync, Share);

impl<'a> OpaqueSubscriber<'a> {
    /// Constructs a new `OpaqueSubscriber` from a reference to a [`Subscriber`].
    pub const fn from_ref<T: Subscriber>(subscriber: &'a T) -> Self {
        unsafe extern "C" fn on_event<'a, T: Subscriber + 'a>(
            handle: OpaqueHandle<dyn Subscriber>,
            event: ConstNonNull<events::Event>,
        ) -> NonNull<()> {
            unsafe {
                let this = &*handle.as_ptr::<T>().cast_const();
                match events::Event::as_enum(event) {
                    events::EventEnum::RegisterThread(register_thread) => {
                        this.register_thread(register_thread);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::UnregisterThread(unregister_thread) => {
                        this.unregister_thread(unregister_thread);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::CreateCallStack(create_call_stack) => {
                        let stack = this.create_call_stack(create_call_stack);
                        NonNull::new_unchecked(stack.as_ptr::<()>())
                    }
                    events::EventEnum::DestroyCallStack(destroy_call_stack) => {
                        this.destroy_call_stack(destroy_call_stack);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::UnblockCallStack(unblock_call_stack) => {
                        this.unblock_call_stack(unblock_call_stack);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::SuspendCallStack(suspend_call_stack) => {
                        this.suspend_call_stack(suspend_call_stack);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::ResumeCallStack(resume_call_stack) => {
                        this.resume_call_stack(resume_call_stack);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::EnterSpan(enter_span) => {
                        this.enter_span(enter_span);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::ExitSpan(exit_span) => {
                        this.exit_span(exit_span);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::LogMessage(log_message) => {
                        this.log_message(log_message);
                        NonNull::from(&mut ())
                    }
                    events::EventEnum::Unknown => NonNull::from(&mut ()),
                }
            }
        }

        unsafe {
            Self {
                handle: AssertSharable::new(OpaqueHandle::new_unchecked(
                    (&raw const *subscriber).cast_mut(),
                )),
                on_event: AssertSharable::new(on_event::<T>),
                _private: PhantomData,
            }
        }
    }
}

unsafe extern "C" {
    static FIMO_TRACING_DEFAULT_SUBSCRIBER: OpaqueSubscriber<'static>;
}

/// Returns the default subscriber.
pub fn default_subscriber() -> OpaqueSubscriber<'static> {
    unsafe { std::ptr::read(&FIMO_TRACING_DEFAULT_SUBSCRIBER) }
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
    pub subscribers: SliceRef<'a, OpaqueSubscriber<'a>>,
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
    pub const fn with_subscribers(mut self, subscribers: &'a [OpaqueSubscriber<'a>]) -> Self {
        self.subscribers = SliceRef::new(subscribers);
        self
    }

    /// Returns a slice of all subscribers.
    pub const fn subscribers(&self) -> &[OpaqueSubscriber<'a>] {
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
    ) -> usize {
        unsafe {
            let mut f = Self::new(buffer, len);
            let _ = f.write_fmt(*data.unwrap_unchecked().as_ptr().cast::<Arguments<'_>>());
            f.pos.min(f.buffer.len())
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
