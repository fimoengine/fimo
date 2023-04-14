//! Implementation of the `fimo-logging` interface.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(c_unwind)]

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    ops::RangeFrom,
    rc::Rc,
};

use fimo_ffi::{type_id::StableTypeId, DynObj, ObjBox, Object};
use fimo_logging_int::{
    BackendId, Channel, ILogger, ILoggerBackend, Level, LevelFilter, Metadata, Record, SpanId,
    SpanMetadata, SpanMetadataBuilder, SpanStackId,
};
use fimo_module::{Error, ErrorKind};
use parking_lot::RwLock;

#[cfg(feature = "module")]
pub mod module;

thread_local! {
    static DATA: RefCell<LocalLoggerData> = RefCell::new(new_local_logger_data());
}

/// Logger implementation.
#[derive(Object, StableTypeId)]
#[name("Logger")]
#[uuid("1ea067c1-988a-4deb-aadb-764ea3737912")]
#[interfaces(ILogger)]
pub struct Logger {
    inner: RwLock<LoggerInner>,
}

struct LoggerInner {
    backend_range: RangeFrom<usize>,
    channel_range: RangeFrom<usize>,
    channels: BTreeMap<String, Channel>,
    channel_infos: HashMap<Channel, ChannelInfo>,
    backends: HashMap<BackendId, ObjBox<DynObj<dyn ILoggerBackend>>>,
}

struct ChannelInfo {
    desc: &'static str,
    level: LevelFilter,
    parent: Option<Channel>,
}

struct LocalLoggerData {
    enabled: bool,
    active_stack: Rc<RefCell<SpanStack>>,
    reusable_spans: Vec<SpanId>,
    span_range: RangeFrom<usize>,
    reusable_stacks: Vec<SpanStackId>,
    stack_range: RangeFrom<usize>,
    spans: HashMap<SpanId, SpanInfo>,
    stacks: HashMap<SpanStackId, Rc<RefCell<SpanStack>>>,
}

struct SpanStack {
    id: SpanStackId,
    stack: Vec<SpanNode>,
    parent: Option<SpanStackId>,
    children: HashSet<SpanStackId>,
    entered_spans: HashSet<SpanId>,
}

#[derive(Clone, Copy)]
struct SpanNode {
    id: SpanId,
}

struct SpanInfo {
    entered_count: usize,
    args: Cow<'static, str>,
    metadata: SpanMetadata<'static>,
}

impl Logger {
    /// Creates a new logger instance.
    pub fn new() -> Logger {
        let level = match std::env::var("FIMO_LOG") {
            Ok(val) if val == "error" => LevelFilter::ERROR,
            Ok(val) if val == "warn" => LevelFilter::WARN,
            Ok(val) if val == "info" => LevelFilter::INFO,
            Ok(val) if val == "debug" => LevelFilter::DEBUG,
            Ok(val) if val == "trace" => LevelFilter::TRACE,
            _ => LevelFilter::OFF,
        };

        let global_info = ChannelInfo {
            desc: "Global",
            level,
            parent: None,
        };
        let channels = BTreeMap::from([(Channel::GLOBAL_KEY.into(), Channel::GLOBAL)]);
        let channel_infos = HashMap::from([(Channel::GLOBAL, global_info)]);

        Logger {
            inner: RwLock::new(LoggerInner {
                backend_range: 1..,
                channel_range: 1..,
                channels,
                channel_infos,
                backends: Default::default(),
            }),
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logger").finish_non_exhaustive()
    }
}

impl ILogger for Logger {
    fn add_backend(
        &self,
        mut logger: ObjBox<DynObj<dyn ILoggerBackend>>,
    ) -> fimo_module::Result<BackendId> {
        let mut lock = self.inner.write();

        let id = match lock.backend_range.next() {
            None => {
                return Err(Error::new(
                    ErrorKind::ResourceExhausted,
                    "exhausted all possible id's",
                ));
            }
            Some(id) => BackendId(id),
        };

        for (key, channel) in &lock.channels {
            logger.create_channel(*channel, key)
        }

        for (channel, info) in &lock.channel_infos {
            if let Some(parent) = info.parent {
                logger.set_channel_parent(*channel, parent)
            }
        }

        lock.backends.insert(id, logger);
        Ok(id)
    }

    fn remove_backend(&self, id: BackendId) -> fimo_module::Result<()> {
        let mut lock = self.inner.write();

        if lock.backends.remove(&id).is_some() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::NotFound, "id not found"))
        }
    }

    fn create_channel(
        &self,
        key: &str,
        description: &'static str,
        parent: Channel,
        level: LevelFilter,
    ) -> fimo_module::Result<Channel> {
        let mut lock = self.inner.write();
        if !lock.channel_infos.contains_key(&parent) {
            return Err(Error::new(ErrorKind::NotFound, "parent channel not found"));
        }

        let channel = match lock.channel_range.next() {
            None => {
                return Err(Error::new(
                    ErrorKind::ResourceExhausted,
                    "exhausted all possible channels",
                ));
            }
            Some(channel) => Channel(channel),
        };

        let entry = lock.channels.entry(key.into());
        if matches!(entry, std::collections::btree_map::Entry::Occupied(_)) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "channel key already exists",
            ));
        }
        entry.or_insert(channel);
        lock.channel_infos.insert(
            channel,
            ChannelInfo {
                desc: description,
                level,
                parent: Some(parent),
            },
        );

        lock.backends.iter_mut().for_each(|(_, backend)| {
            backend.create_channel(channel, key);
            backend.set_channel_parent(channel, parent);
        });

        Ok(channel)
    }

    fn get_channels(&self) -> Vec<Channel> {
        let lock = self.inner.read();
        lock.channels.values().cloned().collect()
    }

    fn get_channel(&self, key: &str) -> Option<Channel> {
        let lock = self.inner.read();
        lock.channels.get(key).cloned()
    }

    fn channel_info(&self, channel: Channel) -> Option<(&'static str, LevelFilter)> {
        let lock = self.inner.read();
        let info = lock.channel_infos.get(&channel)?;
        Some((info.desc, info.level))
    }

    fn set_channel_level(&self, channel: Channel, level: LevelFilter) -> fimo_module::Result<()> {
        let mut lock = self.inner.write();
        let info = match lock.channel_infos.get_mut(&channel) {
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "channel does not exist",
                ))
            }
            Some(info) => info,
        };

        info.level = level;

        Ok(())
    }

    fn create_span(
        &self,
        metadata: SpanMetadata<'static>,
        args: std::fmt::Arguments<'_>,
    ) -> fimo_module::Result<SpanId> {
        DATA.with(|local| {
            if self.channel_info(metadata.channel()).is_none() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "the required channel does not exist",
                ));
            }

            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            let id = if let Some(id) = local.reusable_spans.pop() {
                id
            } else if let Some(id) = local.span_range.next() {
                SpanId(id)
            } else {
                return Err(Error::new(
                    ErrorKind::ResourceExhausted,
                    "ran out of span id's",
                ));
            };

            let args = match args.as_str() {
                Some(x) => Cow::Borrowed(x),
                None => Cow::Owned(std::fmt::format(args)),
            };

            let span = SpanInfo {
                entered_count: 0,
                metadata,
                args,
            };
            assert!(local.spans.insert(id, span,).is_none());

            Ok(id)
        })
    }

    fn delete_span(&self, span: SpanId) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            if !local.spans.contains_key(&span) {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "the provided span was not found",
                ));
            }

            match local.spans.get_mut(&span) {
                Some(info) => {
                    if info.entered_count != 0 {
                        return Err(Error::new(
                            ErrorKind::FailedPrecondition,
                            "the provided span is in use",
                        ));
                    }
                }
                None => {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        "the provided span was not found",
                    ));
                }
            }
            assert!(local.spans.remove(&span).is_some());
            local.reusable_spans.push(span);

            Ok(())
        })
    }

    fn enter_span(&self, span: SpanId) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            if !local.spans.contains_key(&span) {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "the provided span was not found",
                ));
            }

            let mut stack = local.active_stack.borrow_mut();
            if !stack.children.is_empty() {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "can not modify immutable stack",
                ));
            }

            if stack.entered_spans.contains(&span) {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "can not reenter span multiple times",
                ));
            }

            let span_node = SpanNode { id: span };
            stack.stack.push(span_node);
            assert!(stack.entered_spans.insert(span));
            drop(stack);

            let span_info = local
                .spans
                .get_mut(&span)
                .expect("each span must have an associated info struct");

            span_info.entered_count += 1;

            Ok(())
        })
    }

    fn exit_span(&self, current: SpanId) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            let mut stack = local.active_stack.borrow_mut();
            if !stack.children.is_empty() {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "can not modify immutable stack",
                ));
            }

            if stack.stack.len() == 1 {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "can not exit root span",
                ));
            }

            if stack.stack.last().unwrap().id != current {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "the provided span is not at the top of the stack",
                ));
            }

            let span = stack.stack.pop().unwrap();
            assert!(stack.entered_spans.remove(&span.id));
            drop(stack);

            let span_info = local
                .spans
                .get_mut(&span.id)
                .expect("each span must have an associated info struct");

            span_info.entered_count -= 1;

            Ok(())
        })
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        DATA.with(|local| {
            let local = local.borrow();

            if !local.enabled {
                return false;
            }

            let current_stack = local.active_stack.borrow();
            let span = current_stack
                .stack
                .last()
                .expect("stack must always contain at least one element");

            let span_info = local
                .spans
                .get(&span.id)
                .expect("current span must have info associated with it");

            let channel = span_info.metadata.channel();

            let lock = self.inner.read();
            let mut channel_info = lock
                .channel_infos
                .get(&channel)
                .expect("channel must exist");
            let mut channel_level = channel_info.level;

            // Return without logging, if the level was set to off.
            if channel_level == LevelFilter::OFF {
                return false;
            }

            // Find the logging level of the parent channel.
            while let Some(parent) = channel_info.parent {
                let parent_info = lock
                    .channel_infos
                    .get(&parent)
                    .expect("parent channel must exist");

                channel_level = channel_level.min(parent_info.level);
                channel_info = parent_info;

                // Return without logging, if the level was set to off.
                if channel_level == LevelFilter::OFF {
                    return false;
                }
            }

            let span_level = span_info.metadata.level().to_level_filter();
            let min_level = channel_level.min(span_level);
            min_level >= metadata.level()
        })
    }

    fn log(&self, record: &Record<'_>) {
        DATA.with(|local| {
            let local = local.borrow();

            if !local.enabled {
                return;
            }

            let current_stack = local.active_stack.borrow();
            let span = current_stack
                .stack
                .last()
                .expect("stack must always contain at least one element");

            let span_info = local
                .spans
                .get(&span.id)
                .expect("current span must have info associated with it");

            let channel = span_info.metadata.channel();

            let mut lock = self.inner.write();
            let mut channel_info = lock
                .channel_infos
                .get(&channel)
                .expect("channel must exist");
            let mut channel_level = channel_info.level;

            // Return without logging, if the level was set to off.
            if channel_level == LevelFilter::OFF {
                return;
            }

            // Find the logging level of the parent channel.
            while let Some(parent) = channel_info.parent {
                let parent_info = lock
                    .channel_infos
                    .get(&parent)
                    .expect("parent channel must exist");

                channel_level = channel_level.min(parent_info.level);
                channel_info = parent_info;

                // Return without logging, if the level was set to off.
                if channel_level == LevelFilter::OFF {
                    return;
                }
            }

            let span_level = span_info.metadata.level().to_level_filter();
            let min_level = channel_level.min(span_level);

            // Log the message only if the cumulative logging verbosity exceeds or equals
            // the requested verbosity.
            if min_level >= record.level() {
                lock.backends.iter_mut().for_each(|(_, backend)| {
                    backend.log(
                        record,
                        &format_args!("{}", span_info.args),
                        &span_info.metadata,
                    )
                });
            }
        });
    }

    fn branch_span_stack(
        &self,
        current: fimo_logging_int::SpanStackId,
    ) -> fimo_module::Result<fimo_logging_int::SpanStackId> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();
            let LocalLoggerData {
                enabled,
                active_stack,
                reusable_stacks,
                stack_range,
                stacks,
                ..
            } = &mut *local;

            if !*enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            let mut current_stack = active_stack.borrow_mut();
            if current_stack.id != current {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "the provided stack is not currently active",
                ));
            }

            let id = if let Some(id) = reusable_stacks.pop() {
                id
            } else if let Some(id) = stack_range.next() {
                SpanStackId(id)
            } else {
                return Err(Error::new(
                    ErrorKind::ResourceExhausted,
                    "ran out of span stack id's",
                ));
            };

            assert!(current_stack.children.insert(id));
            let spans = current_stack.entered_spans.clone();
            let root = current_stack.stack.last().copied().unwrap();
            drop(current_stack);

            let stack = Rc::new(RefCell::new(SpanStack {
                id,
                stack: vec![root],
                parent: Some(current),
                children: HashSet::default(),
                entered_spans: spans,
            }));
            assert!(stacks.insert(id, stack.clone()).is_none());
            *active_stack = stack;

            Ok(id)
        })
    }

    fn truncate_branched_stacks(
        &self,
        current: fimo_logging_int::SpanStackId,
    ) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            let mut current_stack = local.active_stack.borrow_mut();
            if current_stack.id != current {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "the provided stack is not currently active",
                ));
            }

            let mut to_cleanup = HashSet::default();
            std::mem::swap(&mut current_stack.children, &mut to_cleanup);
            drop(current_stack);

            while !to_cleanup.is_empty() {
                let mut tmp = HashSet::default();
                for id in to_cleanup.drain() {
                    let stack = local.stacks.get(&id).expect("stack branch must exist");
                    let stack = stack.borrow_mut();

                    tmp.extend(stack.children.iter());
                    drop(stack);

                    assert!(local.stacks.remove(&id).is_some());
                }
                to_cleanup = tmp;
            }

            Ok(())
        })
    }

    fn join_stack(&self, current: SpanStackId) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            let current_stack = local.active_stack.borrow();
            if current_stack.id != current {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "the provided stack is not currently active",
                ));
            }

            if current_stack.stack.len() != 1 {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the provided stack is not empty",
                ));
            }

            if !current_stack.children.is_empty() {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the provided stack is immutable",
                ));
            }

            let parent = current_stack.parent;
            drop(current_stack);

            if current != SpanStackId::THREAD {
                assert!(local.stacks.remove(&current).is_some());
                let parent =
                    parent.expect("every stack except the THREAD stack must have a parent");
                let parent_stack = local
                    .stacks
                    .get(&parent)
                    .expect("parent stack must exist")
                    .clone();

                let mut lock = parent_stack.borrow_mut();
                assert!(lock.children.remove(&current));
                drop(lock);

                local.active_stack = parent_stack;
            }

            Ok(())
        })
    }

    fn switch_span_stack(
        &self,
        current: fimo_logging_int::SpanStackId,
        new: fimo_logging_int::SpanStackId,
    ) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is suspended",
                ));
            }

            let current_stack = local.active_stack.borrow();
            if current_stack.id != current {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    "the provided stack is not currently active",
                ));
            }
            drop(current_stack);

            let new_stack = match local.stacks.get(&new) {
                Some(x) => x.clone(),
                None => {
                    return Err(Error::new(ErrorKind::NotFound, "the stack does not exist"));
                }
            };

            local.active_stack = new_stack;

            Ok(())
        })
    }

    fn flush(&self) {
        let mut lock = self.inner.write();
        lock.backends
            .iter_mut()
            .for_each(|(_, backend)| backend.flush());
    }

    fn resume(&self) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is already running",
                ));
            }

            local.enabled = true;
            Ok(())
        })
    }

    fn suspend(&self) -> fimo_module::Result<()> {
        DATA.with(|local| {
            let mut local = local.borrow_mut();

            if !local.enabled {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    "the current thread is already suspended",
                ));
            }

            local.enabled = false;
            Ok(())
        })
    }
}

fn new_local_logger_data() -> LocalLoggerData {
    let root_span_metadata = SpanMetadataBuilder::new("THREAD")
        .level(Level::TRACE)
        .build();
    let root_span = SpanInfo {
        entered_count: 1,
        metadata: root_span_metadata,
        args: Cow::Borrowed(""),
    };
    let spans = HashMap::from([(SpanId::ROOT, root_span)]);

    let root_node = SpanNode { id: SpanId::ROOT };

    let root_stack = Rc::new(RefCell::new(SpanStack {
        id: SpanStackId::THREAD,
        stack: vec![root_node],
        parent: None,
        children: HashSet::default(),
        entered_spans: HashSet::default(),
    }));

    let stacks = HashMap::from([(SpanStackId::THREAD, root_stack.clone())]);

    LocalLoggerData {
        enabled: true,
        span_range: 2..,
        reusable_spans: vec![],
        stack_range: 1..,
        reusable_stacks: vec![],
        active_stack: root_stack,
        spans,
        stacks,
    }
}
