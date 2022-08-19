use crate::{Builder, Runtime};
use fimo_core_int::settings::{ISettingsRegistryExt, SettingsItem, SettingsPath};
use fimo_core_int::IFimoCore;
use fimo_ffi::ptr::IBaseExt;
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, ObjArc, Object};
use fimo_module::{
    Error, ErrorKind, FimoInterface, IModule, IModuleInstance, IModuleInterface, IModuleLoader,
    ModuleInfo,
};
use fimo_tasks_int::IFimoTasks;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::Arc;

const MODULE_NAME: &str = "fimo_tasks";

/// Implementation of the `fimo-tasks` interface.
#[derive(Object, StableTypeId)]
#[name("TasksInterface")]
#[uuid("4d7a5ec0-3acd-42c5-9f18-a4694961985a")]
#[interfaces(IModuleInterface, IFimoTasks)]
pub struct TasksInterface {
    runtime: Arc<Runtime>,
    parent: ObjArc<DynObj<dyn IModuleInstance>>,
}

impl Debug for TasksInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FimoActixInterface)")
    }
}

impl IModuleInterface for TasksInterface {
    fn as_inner(&self) -> &DynObj<dyn fimo_ffi::ptr::IBase + Send + Sync> {
        let inner = fimo_ffi::ptr::coerce_obj::<_, dyn IFimoTasks + Send + Sync>(self);
        inner.cast_super()
    }

    fn name(&self) -> &str {
        <dyn IFimoTasks>::NAME
    }

    fn version(&self) -> fimo_ffi::Version {
        <dyn IFimoTasks>::VERSION
    }

    fn extensions(&self) -> fimo_ffi::Vec<fimo_ffi::String> {
        <dyn IFimoTasks>::EXTENSIONS
            .iter()
            .map(|&s| s.into())
            .collect()
    }

    fn extension(&self, _name: &str) -> Option<&DynObj<dyn fimo_ffi::ptr::IBase + Send + Sync>> {
        None
    }

    fn instance(&self) -> ObjArc<DynObj<dyn IModuleInstance>> {
        self.parent.clone()
    }
}

impl IFimoTasks for TasksInterface {
    fn runtime(&self) -> &DynObj<dyn fimo_tasks_int::runtime::IRuntime> {
        fimo_ffi::ptr::coerce_obj(&*self.runtime)
    }
}

fn module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: <dyn IFimoTasks>::VERSION.into(),
    }
}

fimo_module::rust_module!(load_module);

fn load_module(
    loader: &'static DynObj<dyn IModuleLoader>,
    path: &Path,
) -> fimo_module::Result<ObjArc<DynObj<dyn IModule>>> {
    let module = fimo_module::module::Module::new(module_info(), path, loader, |module| {
        let builder = fimo_module::module::InstanceBuilder::new(module);

        let desc = <dyn IFimoTasks>::new_descriptor();
        let deps = &[<dyn IFimoCore>::new_descriptor()];
        let f = |instance, mut deps: Vec<_>| {
            // we only have one dependency so it must reside as the first element of the vec.
            let core = fimo_module::try_downcast_arc::<dyn IFimoCore, _>(deps.remove(0))?;

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

            let interface = ObjArc::new(TasksInterface {
                runtime,
                parent: ObjArc::coerce_obj(instance),
            });

            Ok(ObjArc::coerce_obj(interface))
        };

        let instance = builder.interface(desc, deps, f).build();
        Ok(instance)
    });
    Ok(ObjArc::coerce_obj(module))
}

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
