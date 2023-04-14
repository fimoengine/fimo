use crate::{Builder, Runtime};
use fimo_core_int::settings::{ISettingsRegistryExt, SettingsItem, SettingsPath};
use fimo_core_int::IFimoCore;
use fimo_ffi::provider::{request_obj, IProvider};
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, ObjBox, Object, Version};
use fimo_module::context::{IInterface, IInterfaceContext};
use fimo_module::module::{Interface, ModuleBuilderBuilder};
use fimo_module::{Error, ErrorKind, QueryBuilder, VersionQuery};
use fimo_tasks_int::IFimoTasks;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::Arc;

/// Implementation of the `fimo-tasks` interface.
#[derive(Object, StableTypeId)]
#[name("TasksInterface")]
#[uuid("4d7a5ec0-3acd-42c5-9f18-a4694961985a")]
#[interfaces(IInterface, IFimoTasks)]
pub struct TasksInterface<'a> {
    runtime: Arc<Runtime>,
    context: &'a DynObj<dyn IInterfaceContext + 'a>,
    _core: &'a DynObj<dyn IFimoCore + 'a>,
}

impl Debug for TasksInterface<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TasksInterface")
            .field("runtime", &self.runtime)
            .field("context", &(self.context as *const _))
            .field("_core", &(self._core as *const _))
            .finish()
    }
}

impl IProvider for TasksInterface<'_> {
    fn provide<'a>(&'a self, demand: &mut fimo_ffi::provider::Demand<'a>) {
        demand.provide_obj::<dyn IFimoTasks + 'a>(fimo_ffi::ptr::coerce_obj(self));
    }
}

impl IInterface for TasksInterface<'_> {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> Version {
        Self::VERSION
    }

    fn extensions(&self) -> &[fimo_ffi::String] {
        &[]
    }
}

impl IFimoTasks for TasksInterface<'_> {
    fn runtime(&self) -> &DynObj<dyn fimo_tasks_int::runtime::IRuntime> {
        fimo_ffi::ptr::coerce_obj(&*self.runtime)
    }
}

const REQUIRED_CORE_VERSION: VersionQuery = VersionQuery::Minimum(Version::new_short(0, 1, 0));

impl Interface for TasksInterface<'_> {
    type Result<'a> = TasksInterface<'a>;
    const NAME: &'static str = QueryBuilder.name::<dyn IFimoTasks>();
    const VERSION: Version = Version::new_short(0, 1, 0);

    fn extensions(_feature: Option<&str>) -> Vec<String> {
        vec![]
    }

    fn dependencies(feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        if feature.is_none() {
            vec![QueryBuilder.query_version::<dyn IFimoCore>(REQUIRED_CORE_VERSION)]
        } else {
            vec![]
        }
    }

    fn optional_dependencies(_feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        vec![]
    }

    fn construct<'a>(
        _module_root: &Path,
        context: &'a DynObj<dyn IInterfaceContext + 'a>,
    ) -> fimo_module::Result<ObjBox<Self::Result<'a>>> {
        let core = context
            .get_interface(QueryBuilder.query_version::<dyn IFimoCore>(REQUIRED_CORE_VERSION))?;
        let core = request_obj::<dyn IFimoCore + 'a>(core)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "The core interface was not found"))?;

        let settings_path = SettingsPath::new("fimo-tasks").unwrap();
        let settings = core
            .settings()
            .read_or(settings_path, ModuleSettings::default())
            .map_err(|_| Error::from(ErrorKind::FailedPrecondition))?;

        let runtime = Builder::new()
            .stack_size(settings.stack_size)
            .allocated_tasks(settings.pre_allocated_stacks)
            .free_threshold(settings.task_free_threshold)
            .workers(settings.workers)
            .build()?;

        Ok(ObjBox::new(TasksInterface {
            runtime,
            context,
            _core: core,
        }))
    }
}

fimo_module::module!(|path, features| {
    Ok(
        ModuleBuilderBuilder::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .with_interface::<TasksInterface<'_>>()
            .build(path, features),
    )
});

#[derive(Copy, Clone)]
struct ModuleSettings {
    stack_size: usize,
    pre_allocated_stacks: usize,
    task_free_threshold: usize,
    workers: Option<usize>,
}

impl ModuleSettings {
    fn new() -> Self {
        Self {
            stack_size: Builder::DEFAULT_STACK_SIZE,
            pre_allocated_stacks: Builder::DEFAULT_PRE_ALLOCATED_TASKS,
            task_free_threshold: Builder::DEFAULT_TASK_FREE_THRESHOLD,
            workers: Builder::DEFAULT_NUM_WORKERS,
        }
    }
}

impl Default for ModuleSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ModuleSettings> for SettingsItem {
    fn from(settings: ModuleSettings) -> Self {
        let mut item = SettingsItem::new_object();
        let map = item.as_map_mut().unwrap();
        map.insert("stack_size".into(), settings.stack_size.into());
        map.insert(
            "pre_allocated_stacks".into(),
            settings.pre_allocated_stacks.into(),
        );
        map.insert(
            "task_free_threshold".into(),
            settings.task_free_threshold.into(),
        );
        map.insert("workers".into(), settings.workers.unwrap_or(0).into());

        item
    }
}

impl TryFrom<SettingsItem> for ModuleSettings {
    type Error = Error;

    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        let mut map = value
            .into_map()
            .ok_or_else(|| Error::new(ErrorKind::InvalidArgument, "Expected map"))?;

        let path_err = || Error::from(ErrorKind::NotFound);
        let err_f = |_e| Error::from(ErrorKind::InvalidArgument);

        let stack_size: usize = map
            .remove("stack_size")
            .ok_or_else(path_err)?
            .try_into()
            .map_err(err_f)?;
        let pre_allocated_stacks: usize = map
            .remove("pre_allocated_stacks")
            .ok_or_else(path_err)?
            .try_into()
            .map_err(err_f)?;
        let task_free_threshold: usize = map
            .remove("task_free_threshold")
            .ok_or_else(path_err)?
            .try_into()
            .map_err(err_f)?;
        let workers: usize = map
            .remove("workers")
            .ok_or_else(path_err)?
            .try_into()
            .map_err(err_f)?;

        let workers = if workers == 0 { None } else { Some(workers) };

        Ok(Self {
            stack_size,
            pre_allocated_stacks,
            task_free_threshold,
            workers,
        })
    }
}
