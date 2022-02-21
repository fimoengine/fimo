use crate::{Builder, Runtime};
use fimo_core_int::rust::settings_registry::{SettingsItem, SettingsRegistryPath};
use fimo_core_int::rust::IFimoCore;
use fimo_ffi::marker::SendSyncMarker;
use fimo_ffi::vtable::{IBase, VTable};
use fimo_ffi::{ObjArc, ObjWeak, Object, Optional, StrInner};
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module::rust_loader::{IRustModuleInner, IRustModuleParent};
use fimo_module::{
    impl_vtable, is_object, rust_module, Error, ErrorKind, FimoInterface, IModuleInstance,
    IModuleInterface, IModuleInterfaceVTable, ModuleInfo, ModuleInterfaceDescriptor, Version,
};
use fimo_tasks_int::{IFimoTasks, IFimoTasksVTable};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

const MODULE_NAME: &str = "fimo_tasks";

#[derive(Debug)]
struct RuntimeModule {
    runtime: Arc<Runtime>,
    parent: ObjArc<IModuleInstance>,
}

impl RuntimeModule {
    #[inline]
    pub fn new(runtime: Arc<Runtime>, parent: ObjArc<IModuleInstance>) -> Self {
        Self { runtime, parent }
    }

    #[inline]
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}

is_object! { #![uuid(0x4d7a5ec0, 0x3acd, 0x42c5, 0x9f18, 0xa4694961985a)] RuntimeModule }

impl_vtable! {
    impl IModuleInterfaceVTable => RuntimeModule {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBase<SendSyncMarker> {
            let i: &IFimoTasksVTable = RuntimeModule::get_vtable();
            i.as_super()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn version(_ptr: *const ()) -> Version {
            IFimoTasks::VERSION
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn extension(
            _ptr: *const (),
            _ext: StrInner<false>,
        ) -> Optional<*const Object<IBase<SendSyncMarker>>> {
            Optional::None
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn instance(ptr: *const ()) -> ObjArc<IModuleInstance> {
            let this = &*(ptr as *const RuntimeModule);
            this.parent.clone()
        }
    }
}

impl_vtable! {
    impl inline IFimoTasksVTable => RuntimeModule {
        |this| {
            let this = unsafe { &*(this as *const RuntimeModule) };
            (&**this.runtime()).into()
        }
    }
}

rust_module!(construct_module);

#[allow(improper_ctypes_definitions)]
extern "C" fn construct_module() -> Result<ObjArc<IRustModuleInner>, Error> {
    Ok(GenericModule::new_inner(
        construct_module_info(),
        build_instance,
    ))
}

fn build_instance(
    parent: ObjArc<IRustModuleParent>,
) -> Result<ObjArc<GenericModuleInstance>, Error> {
    let desc = IFimoTasks::new_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        desc,
        (build_interface as _, vec![IFimoCore::new_descriptor()]),
    );
    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_interface(
    instance: ObjArc<GenericModuleInstance>,
    dep_map: &HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<IModuleInterface>>>,
) -> Result<ObjArc<IModuleInterface>, Error> {
    let core = dep_map
        .get(&IFimoCore::new_descriptor())
        .and_then(|i| i.as_ref().map(|i| i.upgrade()))
        .flatten();

    if core.is_none() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "fimo-core interface not found",
        ));
    }

    let core = core.unwrap();
    let core: ObjArc<IFimoCore> = IModuleInterface::try_downcast_arc(core)?;

    let settings_path = SettingsRegistryPath::new("fimo_tasks").unwrap();

    let registry = core.get_settings_registry();
    let settings: ModuleSettings = match registry.try_read(settings_path) {
        Ok(Some(Ok(s))) => s,
        _ => {
            let settings = ModuleSettings::new();
            registry.write(settings_path, settings).unwrap();
            settings
        }
    };

    let runtime = Builder::new()
        .stack_size(settings.stack_size)
        .allocated_tasks(settings.pre_allocated_stacks)
        .free_threshold(settings.task_free_threshold)
        .workers(settings.workers)
        .build()?;
    let instance = ObjArc::coerce_object(instance);
    let module = RuntimeModule::new(runtime, instance);

    Ok(ObjArc::coerce_object(ObjArc::new(module)))
}

fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: IFimoTasks::VERSION.into(),
    }
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

impl From<ModuleSettings> for SettingsItem {
    fn from(settings: ModuleSettings) -> Self {
        let mut map: BTreeMap<_, _> = Default::default();
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

        map.into()
    }
}

impl TryFrom<SettingsItem> for ModuleSettings {
    type Error = Error;

    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        let mut map = value
            .into_map()
            .ok_or_else(|| Error::new(ErrorKind::InvalidArgument, "Expected map"))?;

        let stack_size: usize = map
            .remove("stack_size")
            .ok_or_else(|| Error::new(ErrorKind::Internal, "Path not found"))?
            .try_into()
            .map_err(|e| Error::new(ErrorKind::Internal, e))?;
        let pre_allocated_stacks: usize = map
            .remove("pre_allocated_stacks")
            .ok_or_else(|| Error::new(ErrorKind::Internal, "Path not found"))?
            .try_into()
            .map_err(|e| Error::new(ErrorKind::Internal, e))?;
        let task_free_threshold: usize = map
            .remove("task_free_threshold")
            .ok_or_else(|| Error::new(ErrorKind::Internal, "Path not found"))?
            .try_into()
            .map_err(|e| Error::new(ErrorKind::Internal, e))?;
        let workers: usize = map
            .remove("workers")
            .ok_or_else(|| Error::new(ErrorKind::Internal, "Path not found"))?
            .try_into()
            .map_err(|e| Error::new(ErrorKind::Internal, e))?;

        let workers = if workers == 0 { None } else { Some(workers) };

        Ok(Self {
            stack_size,
            pre_allocated_stacks,
            task_free_threshold,
            workers,
        })
    }
}
