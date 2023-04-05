//! Definition of the `fimo-logger` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(negative_impls)]
#![feature(unsize)]

use fimo_ffi::{interface, DynObj, ObjBox, Object, ReleaseType, Version};
use fimo_module::{FimoInterface, IModuleInterface};
use std::{
    fmt::{Arguments, Display},
    io::Write,
    sync::atomic::AtomicUsize,
};

/// Global reference to a logger. Is guaranteed to be `None` until
/// `LOGGER_STATUS` is set to `INITIALIZED`.
static mut LOGGER: Option<&'static DynObj<dyn ILogger>> = None;
static LOGGER_STATUS: AtomicUsize = AtomicUsize::new(UNINITIALIZED);

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;
const INITIALIZED: usize = 2;

interface! {
    #![interface_cfg(uuid = "6a146db2-e2b0-4a03-878a-242475cb6650")]

    /// The logging interface.
    pub frozen interface IFimoLogging: IModuleInterface @ frozen version("0.0") {
        /// Fetches a reference to the logger.
        fn logger(&self) -> &DynObj<dyn ILogger>;
    }
}

impl<'a> FimoInterface for dyn IFimoLogging + 'a {
    const NAME: &'static str = "fimo::interfaces::core::fimo_logging";

    const VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

    const EXTENSIONS: &'static [&'static str] = &[];
}

interface! {
    #![interface_cfg(uuid = "db526e53-2bec-4405-b5c1-d373c38a2ea9")]

    /// Interface of a logger.
    pub frozen interface ILogger: marker Send + marker Sync {
        /// Adds a new backend to the logger.
        fn add_backend(
            &self,
            logger: ObjBox<DynObj<dyn ILoggerBackend>>,
        ) -> fimo_module::Result<BackendId>;

        /// Removes a registered backend.
        fn remove_backend(&self, id: BackendId) -> fimo_module::Result<()>;

        /// Creates a new channel where messages can be logged to.
        ///
        /// # Panics
        ///
        /// May panic if `parent` is invalid.
        fn create_channel(
            &self,
            key: &str,
            description: &'static str,
            parent: Channel,
            level: LevelFilter,
        ) -> fimo_module::Result<Channel>;

        /// Returns all channels.
        fn get_channels(&self) -> Vec<Channel>;

        /// Searches for a channel by key.
        fn get_channel(&self, key: &str) -> Option<Channel>;

        /// Returns the description and level registered with a channel.
        fn channel_info(&self, channel: Channel) -> Option<(&'static str, LevelFilter)>;

        /// Changes the level of a channel.
        ///
        /// # Panics
        ///
        /// May panic if `channel` is invalid.
        fn set_channel_level(&self, channel: Channel, level: LevelFilter) -> fimo_module::Result<()>;

        /// Creates a new span.
        fn create_span(
            &self,
            metadata: SpanMetadata<'static>,
            args: Arguments<'_>,
        ) -> fimo_module::Result<SpanId>;

        /// Removes span from the logger.
        ///
        /// The span must not be entered.
        ///
        /// # Panics
        ///
        /// May panic if `span` is invalid.
        fn delete_span(&self, span: SpanId) -> fimo_module::Result<()>;

        /// Enters a new span.
        ///
        /// # Panics
        ///
        /// May panic if `span` is invalid.
        fn enter_span(&self, span: SpanId) -> fimo_module::Result<()>;

        /// Exits the current span.
        ///
        /// # Panics
        ///
        /// May panic if `current` is not the current span or is invalid.
        fn exit_span(&self, current: SpanId) -> fimo_module::Result<()>;

        /// Branches the active stack span and switches to the new one.
        ///
        /// The original branch remains immutable, until all branches are
        /// joined or truncated.
        ///
        /// # Panics
        ///
        /// May panic if `current` is not the current stack or is invalid.
        fn branch_span_stack(&self, current: SpanStackId) -> fimo_module::Result<SpanStackId>;

        /// Removes all branches originating from the current stack.
        ///
        /// Does not attempt to cleanup the spans created by the branches.
        ///
        /// # Panics
        ///
        /// May panic if `current` is not the current stack or is invalid.
        fn truncate_branched_stacks(&self, current: SpanStackId) -> fimo_module::Result<()>;

        /// Joins a branched stack back to its parent.
        ///
        /// Can only be called when the current stack is empty.
        /// Is a noop if called with `SpanStackId::Thread` and the stack is joinable.
        ///
        /// # Panics
        ///
        /// May panic if `current` is not the current stack or is invalid.
        fn join_stack(&self, current: SpanStackId) -> fimo_module::Result<()>;

        /// Sets the active span stack.
        ///
        /// # Panics
        ///
        /// May panic if either id is invalid, or if `current` is not the current stack.
        fn switch_span_stack(&self, current: SpanStackId, new: SpanStackId) -> fimo_module::Result<()>;

        /// Determines if a log message with the specified metadata would be logged.
        fn enabled(&self, metadata: &Metadata<'_>) -> bool;

        /// Logs a new record to the logger.
        fn log(&self, record: &Record<'_>);

        /// Flushes the messages written to the logger.
        fn flush(&self);

        /// Marks that the current thread has resumed it's execution.
        ///
        /// A thread is marked as running by default, but can be suspended with
        /// [`suspend`](#method.suspend).
        fn resume(&self) -> fimo_module::Result<()>;

        /// Suspends the current thread.
        ///
        /// Calling into the logger using a suspended thread may result in an error.
        fn suspend(&self) -> fimo_module::Result<()>;
    }
}

interface! {
    #![interface_cfg(uuid = "a44d9a8b-87cb-45e1-9a29-615574886d8a")]

    /// Backend of the logger interface.
    ///
    /// Backends are responsible for logging messages.
    pub frozen interface ILoggerBackend: marker Send + marker Sync {
        /// Registers a channel with the backend.
        fn create_channel(&mut self, channel: Channel, key: &str);

        /// Sets the parent of a channel.
        ///
        /// Channels without a parent default to using [`Channel::GLOBAL`] as their parent.
        fn set_channel_parent(&mut self, channel: Channel, parent: Channel);

        /// Logs a new record to the backend.
        fn log(
            &mut self,
            record: &Record<'_>,
            span_args: &Arguments<'_>,
            span_metadata: &SpanMetadata<'_>,
        );

        /// Flushes the messages from the backend.
        fn flush(&mut self);
    }
}

/// An enum representing the available verbosity levels of the logger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Level(usize);

impl Level {
    /// The "error" level.
    pub const ERROR: Level = Level(1);
    /// The "warn" level.
    pub const WARN: Level = Level(2);
    /// The "info" level.
    pub const INFO: Level = Level(3);
    /// The "debug" level.
    pub const DEBUG: Level = Level(4);
    /// The "trace" level.
    pub const TRACE: Level = Level(5);

    const LOG_LEVEL_NAMES: [&'static str; 6] = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

    /// Returns the most verbose logging level.
    pub const fn max() -> Level {
        Self::TRACE
    }

    /// Converts the `Level` to the equivalent [`LevelFilter`].
    pub const fn to_level_filter(&self) -> LevelFilter {
        LevelFilter(self.0)
    }

    /// Returns the string representation of the `Level`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
    pub const fn as_str(&self) -> &'static str {
        Self::LOG_LEVEL_NAMES[self.0]
    }
}

impl PartialEq<LevelFilter> for Level {
    fn eq(&self, other: &LevelFilter) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd<LevelFilter> for Level {
    fn partial_cmp(&self, other: &LevelFilter) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

/// The available verbosity level filters of the logger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LevelFilter(usize);

impl LevelFilter {
    /// A level lower than all log levels.
    pub const OFF: LevelFilter = LevelFilter(0);
    /// Corresponds to the `Error` log level.
    pub const ERROR: LevelFilter = LevelFilter(1);
    /// Corresponds to the `WARN` log level.
    pub const WARN: LevelFilter = LevelFilter(2);
    /// Corresponds to the `INFO` log level.
    pub const INFO: LevelFilter = LevelFilter(3);
    /// Corresponds to the `DEBUG` log level.
    pub const DEBUG: LevelFilter = LevelFilter(4);
    /// Corresponds to the `TRACE` log level.
    pub const TRACE: LevelFilter = LevelFilter(5);

    /// Returns the most verbose logging level filter.
    pub const fn max() -> LevelFilter {
        Self::TRACE
    }

    /// Converts self to the equivalent Level.
    ///
    /// Returns None if self is [`LevelFilter::OFF`].
    pub const fn to_level(&self) -> Option<Level> {
        match self.0 {
            level if level >= Level::ERROR.0 && level <= Level::max().0 => Some(Level(level)),
            _ => None,
        }
    }

    /// Returns the string representation of the `LevelFilter`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
    pub const fn as_str(&self) -> &'static str {
        Level::LOG_LEVEL_NAMES[self.0]
    }
}

impl PartialEq<Level> for LevelFilter {
    fn eq(&self, other: &Level) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd<Level> for LevelFilter {
    fn partial_cmp(&self, other: &Level) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// The channel that can contain messages.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Channel(pub usize);

impl Channel {
    /// The global channel, parent of every other channel.
    pub const GLOBAL: Channel = Channel(0);

    /// Key of the `Global` channel.
    pub const GLOBAL_KEY: &'static str = "";
}

/// Id of a registered logger backend.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BackendId(pub usize);

impl BackendId {
    /// Id of an invalid backend.
    pub const INVALID: BackendId = BackendId(0);
}

/// RAII guard for a registered logger backend.
#[must_use = "Dropping the guard unregisters the backend"]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BackendGuard(BackendId);

impl BackendGuard {
    /// Registers a new backend.
    pub fn new(logger: ObjBox<DynObj<dyn ILoggerBackend>>) -> BackendGuard {
        let id = crate::logger()
            .add_backend(logger)
            .expect("could not register a new backend");
        Self(id)
    }
}

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = logger().remove_backend(self.0);
    }
}

/// Identifier of a span returned by a logger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpanId(pub usize);

impl SpanId {
    /// The id of a disabled span.
    pub const DISABLED: SpanId = SpanId(0);

    /// The id of the root span of the current thread.
    pub const ROOT: SpanId = SpanId(1);
}

/// Identifier of a span stack returned by a logger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpanStackId(pub usize);

impl SpanStackId {
    /// The id of stack created automatically for the current thread.
    pub const THREAD: SpanStackId = SpanStackId(0);
}

/// Metadata of a span.
#[derive(Clone, Debug)]
pub struct SpanMetadata<'a> {
    level: Level,
    name: &'a str,
    target: &'a str,
    channel: Channel,
    module_path: Option<&'a str>,
    file: Option<&'a str>,
    line: Option<u32>,
}

impl<'a> SpanMetadata<'a> {
    /// Returns a new builder.
    pub fn builder(name: &'a str) -> SpanMetadataBuilder<'a> {
        SpanMetadataBuilder::new(name)
    }

    /// The verbosity level of the span.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The name of the span.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// The name of the target of the directive.
    pub fn target(&self) -> &'a str {
        self.target
    }

    /// The channel where the span writes it's messages.
    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// The module path of the span.
    pub fn module_path(&self) -> Option<&'a str> {
        self.module_path
    }

    /// The source file containing the span.
    pub fn file(&self) -> Option<&'a str> {
        self.file
    }

    /// The line containing the span.
    pub fn line(&self) -> Option<u32> {
        self.line
    }
}

/// Builder for [`SpanMetadata`].
#[derive(Debug)]
pub struct SpanMetadataBuilder<'a> {
    metadata: SpanMetadata<'a>,
}

impl<'a> SpanMetadataBuilder<'a> {
    /// Constructs a new `RecordBuilder`.
    ///
    /// The default options are:
    ///
    /// - `level`: [`Level::INFO`]
    /// - `target`: `""`
    /// - `channel`: [`Channel::GLOBAL`]
    /// - `module_path`: `None`
    /// - `file`: `None`
    /// - `line`: `None`
    pub fn new(name: &'a str) -> SpanMetadataBuilder<'a> {
        SpanMetadataBuilder {
            metadata: SpanMetadata {
                name,
                level: Level::INFO,
                target: "",
                channel: Channel::GLOBAL,
                module_path: None,
                file: None,
                line: None,
            },
        }
    }

    /// Setter for [`level`](SpanMetadata::level).
    pub fn level(&mut self, level: Level) -> &mut SpanMetadataBuilder<'a> {
        self.metadata.level = level;
        self
    }

    /// Setter for [`target`](SpanMetadata::target).
    pub fn target(&mut self, target: &'a str) -> &mut SpanMetadataBuilder<'a> {
        self.metadata.target = target;
        self
    }

    /// Setter for [`channel`](SpanMetadata::channel).
    pub fn channel(&mut self, channel: Channel) -> &mut SpanMetadataBuilder<'a> {
        self.metadata.channel = channel;
        self
    }

    /// Setter for [`module_path`](SpanMetadata::module_path).
    pub fn module_path(&mut self, path: Option<&'a str>) -> &mut SpanMetadataBuilder<'a> {
        self.metadata.module_path = path;
        self
    }

    /// Setter for [`file`](SpanMetadata::file).
    pub fn file(&mut self, file: Option<&'a str>) -> &mut SpanMetadataBuilder<'a> {
        self.metadata.file = file;
        self
    }

    /// Setter for [`line`](SpanMetadata::line).
    pub fn line(&mut self, line: Option<u32>) -> &mut SpanMetadataBuilder<'a> {
        self.metadata.line = line;
        self
    }

    /// Returns a [`Metadata`] object.
    pub fn build(&self) -> SpanMetadata<'a> {
        self.metadata.clone()
    }
}

/// An owned span for logging messages.
#[derive(Debug)]
pub struct Span {
    id: SpanId,
}

impl !Send for Span {}
impl !Sync for Span {}

impl Span {
    /// Creates a new `Span` with the provided metadata and arguments.
    pub fn new(metadata: SpanMetadata<'static>, args: Arguments<'_>) -> Span {
        let id = logger()
            .create_span(metadata, args)
            .expect("could not create span");
        Span { id }
    }

    /// Enters the context.
    pub fn enter(&self) -> SpanGuard<'_> {
        if self.id != SpanId::DISABLED {
            logger().enter_span(self.id).expect("could not enter span");
        }
        SpanGuard { span: self }
    }

    /// Enters the context for the duration of the scope.
    pub fn scoped<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.scoped_with_guard(move |_| f())
    }

    /// Enters the context for the duration of the scope and additionally
    /// provides a reference to the created guard.
    pub fn scoped_with_guard<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&SpanGuard<'_>) -> R,
    {
        let guard = self.enter();
        f(&guard)
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.id != SpanId::DISABLED {
            logger()
                .delete_span(self.id)
                .expect("could not delete span");
        }
    }
}

/// RAII guard for spans.
#[derive(Debug)]
pub struct SpanGuard<'a> {
    span: &'a Span,
}

impl Drop for SpanGuard<'_> {
    fn drop(&mut self) {
        if self.span.id != SpanId::DISABLED {
            logger()
                .exit_span(self.span.id)
                .expect("could not exit span");
        }
    }
}

/// The “payload” of a log message.
#[derive(Clone, Debug)]
pub struct Record<'a> {
    metadata: Metadata<'a>,
    args: Arguments<'a>,
    module_path: Option<&'a str>,
    file: Option<&'a str>,
    line: Option<u32>,
}

impl<'a> Record<'a> {
    /// Returns a new builder.
    pub fn builder() -> RecordBuilder<'a> {
        RecordBuilder::new()
    }

    /// The message body.
    pub fn args(&self) -> &Arguments<'a> {
        &self.args
    }

    /// Metadata about the log directive.
    pub fn metadata(&self) -> &Metadata<'a> {
        &self.metadata
    }

    /// The verbosity level of the message.
    pub fn level(&self) -> Level {
        self.metadata.level()
    }

    /// The name of the target of the directive.
    pub fn target(&self) -> &'a str {
        self.metadata.target()
    }

    /// The module path of the message.
    pub fn module_path(&self) -> Option<&'a str> {
        self.module_path
    }

    /// The source file containing the message.
    pub fn file(&self) -> Option<&'a str> {
        self.file
    }

    /// The line containing the message.
    pub fn line(&self) -> Option<u32> {
        self.line
    }
}

/// Builder for [`Record`].
#[derive(Debug)]
pub struct RecordBuilder<'a> {
    record: Record<'a>,
}

impl<'a> RecordBuilder<'a> {
    /// Constructs a new `RecordBuilder`.
    ///
    /// The default options are:
    ///
    /// - `args`: [`format_args!("")`]
    /// - `metadata`: [`Metadata::builder().build()`]
    /// - `module_path`: `None`
    /// - `file`: `None`
    /// - `line`: `None`
    ///
    /// [`format_args!("")`]: https://doc.rust-lang.org/std/macro.format_args.html
    pub fn new() -> RecordBuilder<'a> {
        RecordBuilder {
            record: Record {
                metadata: Metadata::builder().build(),
                args: format_args!(""),
                module_path: None,
                file: None,
                line: None,
            },
        }
    }

    /// Setter for [`args`](Record::args).
    pub fn args(&mut self, args: Arguments<'a>) -> &mut RecordBuilder<'a> {
        self.record.args = args;
        self
    }

    /// Setter for [`metadata`](Record::metadata).
    pub fn metadata(&mut self, metadata: Metadata<'a>) -> &mut RecordBuilder<'a> {
        self.record.metadata = metadata;
        self
    }

    /// Setter for [`level`](Record::level).
    pub fn level(&mut self, level: Level) -> &mut RecordBuilder<'a> {
        self.record.metadata.level = level;
        self
    }

    /// Setter for [`target`](Record::target).
    pub fn target(&mut self, target: &'a str) -> &mut RecordBuilder<'a> {
        self.record.metadata.target = target;
        self
    }

    /// Setter for [`module_path`](Record::module_path).
    pub fn module_path(&mut self, path: Option<&'a str>) -> &mut RecordBuilder<'a> {
        self.record.module_path = path;
        self
    }

    /// Setter for [`file`](Record::file).
    pub fn file(&mut self, file: Option<&'a str>) -> &mut RecordBuilder<'a> {
        self.record.file = file;
        self
    }

    /// Setter for [`line`](Record::line).
    pub fn line(&mut self, line: Option<u32>) -> &mut RecordBuilder<'a> {
        self.record.line = line;
        self
    }

    /// Invoke the builder and return a [`Record`].
    pub fn build(&self) -> Record<'a> {
        self.record.clone()
    }
}

impl Default for RecordBuilder<'_> {
    fn default() -> Self {
        RecordBuilder::new()
    }
}

/// Metadata about a log message.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata<'a> {
    level: Level,
    target: &'a str,
}

impl<'a> Metadata<'a> {
    /// Returns a new builder.
    pub fn builder() -> MetadataBuilder<'a> {
        MetadataBuilder::new()
    }

    /// The verbosity level of the message.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The name of the target of the directive.
    pub fn target(&self) -> &'a str {
        self.target
    }
}

/// Builder for a [`Metadata`].
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MetadataBuilder<'a> {
    metadata: Metadata<'a>,
}

impl<'a> MetadataBuilder<'a> {
    /// Constructs a new `MetadataBuilder`.
    ///
    /// The default options are:
    ///
    /// - `level`: [`Level::INFO`]
    /// - `target`: `""`
    pub fn new() -> MetadataBuilder<'a> {
        MetadataBuilder {
            metadata: Metadata {
                level: Level::INFO,
                target: "",
            },
        }
    }

    /// Setter for [`level`](Metadata::level).
    pub fn level(&mut self, level: Level) -> &mut MetadataBuilder<'a> {
        self.metadata.level = level;
        self
    }

    /// Setter for [`target`](Metadata::target).
    pub fn target(&mut self, target: &'a str) -> &mut MetadataBuilder<'a> {
        self.metadata.target = target;
        self
    }

    /// Returns a [`Metadata`] object.
    pub fn build(&self) -> Metadata<'a> {
        self.metadata.clone()
    }
}

impl Default for MetadataBuilder<'_> {
    fn default() -> Self {
        MetadataBuilder::new()
    }
}

/// The type returned by [`set_logger`] if [`set_logger`] has already been called.
#[derive(Debug)]
pub struct SetLoggerError;

impl Display for SetLoggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("attempted to set a logger after the logging system was already initialized")
    }
}

impl std::error::Error for SetLoggerError {}

/// Sets the global logger. Can only be called once.
pub fn set_logger(logger: &'static DynObj<dyn ILogger>) -> Result<(), SetLoggerError> {
    let status = match LOGGER_STATUS.compare_exchange(
        UNINITIALIZED,
        INITIALIZING,
        std::sync::atomic::Ordering::SeqCst,
        std::sync::atomic::Ordering::SeqCst,
    ) {
        Ok(s) | Err(s) => s,
    };

    match status {
        UNINITIALIZED => {
            unsafe { LOGGER = Some(logger) };
            LOGGER_STATUS.store(INITIALIZED, std::sync::atomic::Ordering::Release);
            Ok(())
        }
        INITIALIZING => {
            while LOGGER_STATUS.load(std::sync::atomic::Ordering::Acquire) == INITIALIZING {
                std::hint::spin_loop()
            }
            Err(SetLoggerError)
        }
        _ => Err(SetLoggerError),
    }
}

/// Retrieves a reference to the registered logger.
pub fn logger() -> &'static DynObj<dyn ILogger> {
    if LOGGER_STATUS.load(std::sync::atomic::Ordering::Acquire) != INITIALIZED {
        static LOGGER: &NoopLogger = &NoopLogger;
        fimo_ffi::ptr::coerce_obj(LOGGER)
    } else {
        unsafe { LOGGER.unwrap_unchecked() }
    }
}

/// Returns the current maximum log level.
pub fn max_level() -> LevelFilter {
    logger().channel_info(Channel::GLOBAL).unwrap().1
}

/// Sets the global maximum log level.
pub fn set_max_level(level: LevelFilter) {
    logger()
        .set_channel_level(Channel::GLOBAL, level)
        .expect("could not set maximum level")
}

/// The standard logging macro.
#[macro_export]
macro_rules! log {
    // log!(target: "my_target", Level::Info, "a {} event", "log");
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => {{
        let lvl = $lvl;
        if lvl <= $crate::max_level() {
            $crate::__private_api_log(
                format_args!($($arg)+),
                lvl,
                &($target, module_path!(), file!(), line!())
            );
        }
    }};

    // log!(Level::Info, "a log event")
    ($lvl:expr, $($arg:tt)+) => ($crate::log!(target: module_path!(), $lvl, $($arg)+));
}

/// Logs a message at the error level.
#[macro_export]
macro_rules! error {
    // error!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ($crate::log!(target: $target, $crate::Level::ERROR, $($arg)+));

    // error!("a {} event", "log")
    ($($arg:tt)+) => ($crate::log!($crate::Level::ERROR, $($arg)+))
}

/// Logs a message at the warn level.
#[macro_export]
macro_rules! warn {
    // warn!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ($crate::log!(target: $target, $crate::Level::WARN, $($arg)+));

    // warn!("a {} event", "log")
    ($($arg:tt)+) => ($crate::log!($crate::Level::WARN, $($arg)+))
}

/// Logs a message at the info level.
#[macro_export]
macro_rules! info {
    // info!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ($crate::log!(target: $target, $crate::Level::INFO, $($arg)+));

    // info!("a {} event", "log")
    ($($arg:tt)+) => ($crate::log!($crate::Level::INFO, $($arg)+))
}

/// Logs a message at the debug level.
#[macro_export]
macro_rules! debug {
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ($crate::log!(target: $target, $crate::Level::DEBUG, $($arg)+));

    // debug!("a {} event", "log")
    ($($arg:tt)+) => ($crate::log!($crate::Level::DEBUG, $($arg)+))
}

/// Logs a message at the trace level.
#[macro_export]
macro_rules! trace {
    // trace!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ($crate::log!(target: $target, $crate::Level::TRACE, $($arg)+));

    // trace!("a {} event", "log")
    ($($arg:tt)+) => ($crate::log!($crate::Level::TRACE, $($arg)+))
}

/// Determines if a message logged at the specified level in that module will
/// be logged.
#[macro_export]
macro_rules! log_enabled {
    (target: $target:expr, $lvl:expr) => {{
        let lvl = $lvl;
        if lvl <= $crate::max_level()
                && $crate::__private_api_enabled(lvl, $target)
    }};
    ($lvl:expr) => {
        log_enabled!(target: module_path!(), $lvl)
    }
}

/// Constructs a new span.
#[macro_export]
macro_rules! span {
    // span!(target: "my_target", channel: Channel::GLOBAL, Level::TRACE, "my span", "a {} event", "log")
    (target: $target:expr, channel: $channel:expr, $lvl:expr, $name:expr, $($arg:tt)+) => {{
        let lvl = $lvl;
        let name = $name;
        let target = $target;
        let channel = $channel;
        $crate::__private_api_span(
            lvl,
            channel,
            name,
            target,
            format_args!($($arg)+)
        )
    }};

    // span!(channel: Channel::GLOBAL, Level::TRACE, "my span", "a {} event", "log")
    (channel: $channel:expr, $lvl:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            target: module_path!(),
            channel: $channel,
            $crate::Level::INFO,
            $name,
            $($arg)+
        )
    };

    // span!(Level::TRACE, "my span", "a {} event", "log")
    ($lvl:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            channel: $crate::Channel::GLOBAL,
            $crate::Level::INFO,
            $name,
            $($arg)+
        )
    };

    // span!("my span", "a {} event", "log")
    ($name:expr, $($arg:tt)+) => {
        $crate::span!(
            $crate::Level::INFO,
            $name,
            $($arg)+
        )
    }
}

/// Constructs a new span at the error level.
#[macro_export]
macro_rules! error_span {
    // error_span!(target: "my_target", channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (target: $target:expr, channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            target: $target,
            channel: $channel,
            $crate::Level::ERROR,
            $name,
            $($arg)+
        )
    };

    // error_span!(channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::error_span!(
            target: module_path!(),
            channel: $channel,
            $name,
            $($arg)+
        )
    };

    // error_span!("my span", "a {} event", "log")
    ($name:expr, $($arg:tt)+) => {
        $crate::error_span!(
            channel: $crate::Channel::GLOBAL,
            $name,
            $($arg)+
        )
    }
}

/// Constructs a new span at the warn level.
#[macro_export]
macro_rules! warn_span {
    // warn_span!(target: "my_target", channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (target: $target:expr, channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            target: $target,
            channel: $channel,
            $crate::Level::WARN,
            $name,
            $($arg)+
        )
    };

    // warn_span!(channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::warn_span!(
            target: module_path!(),
            channel: $channel,
            $name,
            $($arg)+
        )
    };

    // warn_span!("my span", "a {} event", "log")
    ($name:expr, $($arg:tt)+) => {
        $crate::warn_span!(
            channel: $crate::Channel::GLOBAL,
            $name,
            $($arg)+
        )
    }
}

/// Constructs a new span at the info level.
#[macro_export]
macro_rules! info_span {
    // info_span!(target: "my_target", channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (target: $target:expr, channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            target: $target,
            channel: $channel,
            $crate::Level::INFO,
            $name,
            $($arg)+
        )
    };

    // info_span!(channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::info_span!(
            target: module_path!(),
            channel: $channel,
            $name,
            $($arg)+
        )
    };

    // info_span!("my span", "a {} event", "log")
    ($name:expr, $($arg:tt)+) => {
        $crate::info_span!(
            channel: $crate::Channel::GLOBAL,
            $name,
            $($arg)+
        )
    }
}

/// Constructs a new span at the debug level.
#[macro_export]
macro_rules! debug_span {
    // debug_span!(target: "my_target", channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (target: $target:expr, channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            target: $target,
            channel: $channel,
            $crate::Level::DEBUG,
            $name,
            $($arg)+
        )
    };

    // debug_span!(channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::debug_span!(
            target: module_path!(),
            channel: $channel,
            $name,
            $($arg)+
        )
    };

    // debug_span!("my span", "a {} event", "log")
    ($name:expr, $($arg:tt)+) => {
        $crate::debug_span!(
            channel: $crate::Channel::GLOBAL,
            $name,
            $($arg)+
        )
    }
}

/// Constructs a new span at the trace level.
#[macro_export]
macro_rules! trace_span {
    // trace_span!(target: "my_target", channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (target: $target:expr, channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::span!(
            target: $target,
            channel: $channel,
            $crate::Level::TRACE,
            $name,
            $($arg)+
        )
    };

    // trace_span!(channel: Channel::GLOBAL, "my span", "a {} event", "log")
    (channel: $channel:expr, $name:expr, $($arg:tt)+) => {
        $crate::trace_span!(
            target: module_path!(),
            channel: $channel,
            $name,
            $($arg)+
        )
    };

    // trace_span!("my span", "a {} event", "log")
    ($name:expr, $($arg:tt)+) => {
        $crate::trace_span!(
            channel: $crate::Channel::GLOBAL,
            $name,
            $($arg)+
        )
    }
}

#[doc(hidden)]
pub fn __private_api_log(
    args: Arguments<'_>,
    level: Level,
    &(target, module_path, file, line): &(&str, &'static str, &'static str, u32),
) {
    logger().log(
        &Record::builder()
            .args(args)
            .level(level)
            .target(target)
            .module_path(Some(module_path))
            .file(Some(file))
            .line(Some(line))
            .build(),
    )
}

#[doc(hidden)]
pub fn __private_api_enabled(level: Level, target: &str) -> bool {
    logger().enabled(&Metadata::builder().level(level).target(target).build())
}

#[doc(hidden)]
pub fn __private_api_span(
    level: Level,
    channel: Channel,
    name: &'static str,
    target: &'static str,
    args: Arguments<'_>,
) -> Span {
    Span::new(
        SpanMetadata::builder(name)
            .level(level)
            .target(target)
            .channel(channel)
            .build(),
        args,
    )
}

#[derive(Object)]
#[interfaces(ILogger)]
struct NoopLogger;

impl ILogger for NoopLogger {
    fn add_backend(
        &self,
        _logger: ObjBox<DynObj<dyn ILoggerBackend>>,
    ) -> fimo_module::Result<BackendId> {
        Ok(BackendId::INVALID)
    }

    fn remove_backend(&self, _id: BackendId) -> fimo_module::Result<()> {
        Ok(())
    }

    fn create_channel(
        &self,
        _key: &str,
        _description: &'static str,
        parent: Channel,
        _level: LevelFilter,
    ) -> fimo_module::Result<Channel> {
        debug_assert_eq!(parent, Channel::GLOBAL);
        Ok(Channel::GLOBAL)
    }

    fn get_channels(&self) -> Vec<Channel> {
        vec![]
    }

    fn get_channel(&self, key: &str) -> Option<Channel> {
        if key == Channel::GLOBAL_KEY {
            Some(Channel::GLOBAL)
        } else {
            None
        }
    }

    fn channel_info(&self, channel: Channel) -> Option<(&'static str, LevelFilter)> {
        if channel == Channel::GLOBAL {
            Some(("", LevelFilter::OFF))
        } else {
            None
        }
    }

    fn set_channel_level(&self, channel: Channel, _level: LevelFilter) -> fimo_module::Result<()> {
        debug_assert_eq!(channel, Channel::GLOBAL);
        Ok(())
    }

    fn create_span(
        &self,
        _metadata: SpanMetadata<'static>,
        _args: Arguments<'_>,
    ) -> fimo_module::Result<SpanId> {
        Ok(SpanId::DISABLED)
    }

    fn delete_span(&self, span: SpanId) -> fimo_module::Result<()> {
        debug_assert!(span == SpanId::DISABLED || span == SpanId::ROOT);
        Ok(())
    }

    fn enter_span(&self, span: SpanId) -> fimo_module::Result<()> {
        debug_assert!(span == SpanId::DISABLED || span == SpanId::ROOT);
        Ok(())
    }

    fn exit_span(&self, span: SpanId) -> fimo_module::Result<()> {
        debug_assert!(span == SpanId::DISABLED || span == SpanId::ROOT);
        Ok(())
    }

    fn branch_span_stack(&self, current: SpanStackId) -> fimo_module::Result<SpanStackId> {
        debug_assert_eq!(current, SpanStackId::THREAD);
        Ok(SpanStackId::THREAD)
    }

    fn truncate_branched_stacks(&self, current: SpanStackId) -> fimo_module::Result<()> {
        debug_assert_eq!(current, SpanStackId::THREAD);
        Ok(())
    }

    fn join_stack(&self, current: SpanStackId) -> fimo_module::Result<()> {
        debug_assert_eq!(current, SpanStackId::THREAD);
        Ok(())
    }

    fn switch_span_stack(&self, current: SpanStackId, new: SpanStackId) -> fimo_module::Result<()> {
        debug_assert_eq!(current, SpanStackId::THREAD);
        debug_assert_eq!(new, SpanStackId::THREAD);
        Ok(())
    }

    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        false
    }

    fn log(&self, _record: &Record<'_>) {}

    fn flush(&self) {}

    fn resume(&self) -> fimo_module::Result<()> {
        Ok(())
    }

    fn suspend(&self) -> fimo_module::Result<()> {
        Ok(())
    }
}

/// Backend for logging into the console.
#[derive(Clone, Debug, Object)]
#[interfaces(ILoggerBackend)]
pub struct ConsoleBackend {
    colored: bool,
    max_target_width: usize,
}

impl ConsoleBackend {
    /// Constructs a new `ConsoleBackend` with the default settings.
    pub fn new() -> ConsoleBackend {
        Self {
            colored: true,
            max_target_width: 0,
        }
    }
}

impl Default for ConsoleBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ILoggerBackend for ConsoleBackend {
    fn create_channel(&mut self, _channel: Channel, _key: &str) {}

    fn set_channel_parent(&mut self, _channel: Channel, _parent: Channel) {}

    fn log(
        &mut self,
        record: &Record<'_>,
        span_args: &Arguments<'_>,
        span_metadata: &SpanMetadata<'_>,
    ) {
        let record_level = record.level();
        let record_args = record.args();
        let span_name = span_metadata.name();
        let target = record.target();

        let target_len = target.len().max(self.max_target_width);
        self.max_target_width = target_len;

        let color = if self.colored {
            match record_level {
                Level::ERROR => "\x1b[31m",
                Level::WARN => "\x1b[33m",
                Level::INFO => "\x1b[32m",
                Level::DEBUG => "\x1b[34m",
                Level::TRACE => "\x1b[35m",
                _ => "",
            }
        } else {
            ""
        };

        let mut output = std::io::stderr().lock();
        let _ = writeln!(
            output,
            " {color}{record_level:<5}\x1b[m \x1b[1m{target:target_len$}\x1b[m > [name={span_name:?}, args={{{span_args:?}}}] {record_args}"
        );
    }

    fn flush(&mut self) {
        let _ = std::io::stderr().flush();
    }
}

/// Builder for a [`ConsoleBackend`].
#[derive(Debug)]
pub struct ConsoleBackendBuilder {
    backend: ConsoleBackend,
}

impl ConsoleBackendBuilder {
    /// Constructs a new `ConsoleBackendBuilder`.
    ///
    /// The default options are:
    ///
    /// - `color`: `true`
    pub fn new() -> ConsoleBackendBuilder {
        Self {
            backend: ConsoleBackend::new(),
        }
    }

    /// Sets whether the backend outputs colors.
    pub fn color(mut self, c: bool) -> Self {
        self.backend.colored = c;
        self
    }

    /// Builds a [`ConsoleBackend`].
    pub fn build(&self) -> ConsoleBackend {
        self.backend.clone()
    }
}

impl Default for ConsoleBackendBuilder {
    fn default() -> Self {
        Self::new()
    }
}
