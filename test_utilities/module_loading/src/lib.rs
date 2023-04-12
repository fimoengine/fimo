#![feature(unsize)]

use fimo_ffi::{DynObj, ObjArc};
use fimo_module::loader::{RustLoader, MODULE_LOADER_TYPE};
use fimo_module::{Error, IModule, IModuleLoader, InterfaceDescriptor};
use fimo_module::{FimoInterface, IModuleInstance, IModuleInterface};
use std::any::TypeId;
use std::collections::BTreeMap;
use std::marker::Unsize;
use std::path::{Path, PathBuf};

use fimo_core_int::modules::{IModuleRegistry, IModuleRegistryExt, InterfaceHandle};
use fimo_core_int::IFimoCore;
use fimo_ffi::ptr::{CastInto, IBase};

#[cfg(feature = "fimo_actix_int")]
use fimo_actix_int::IFimoActix;

#[cfg(feature = "fimo_tasks_int")]
use fimo_tasks_int::IFimoTasks;

#[cfg(feature = "fimo_logging_int")]
use fimo_logging_int::IFimoLogging;

#[cfg(target_os = "windows")]
macro_rules! lib_path {
    ($lib:literal) => {
        std::concat!($lib, ".dll")
    };
}

#[cfg(target_os = "linux")]
macro_rules! lib_path {
    ($lib:literal) => {
        std::concat!("lib", $lib, ".so")
    };
}

#[cfg(target_os = "macos")]
macro_rules! lib_path {
    ($lib:literal) => {
        std::concat!("lib", $lib, ".dylib")
    };
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
macro_rules! lib_path {
    ($lib:literal) => {
        std::compile_error("Not supported");
    };
}

pub struct ModuleDatabase {
    core: ObjArc<DynObj<dyn IFimoCore>>,
    core_instance: ObjArc<DynObj<dyn IModuleInstance>>,
    paths: BTreeMap<InterfaceDescriptor, PathBuf>,
}

impl ModuleDatabase {
    pub fn new() -> Result<Self, Error> {
        let mut paths = BTreeMap::new();
        paths.insert(
            <dyn IFimoCore>::new_descriptor(),
            module_path("fimo_core", lib_path!("fimo_core")),
        );
        #[cfg(feature = "fimo_actix_int")]
        paths.insert(
            <dyn IFimoActix>::new_descriptor(),
            module_path("fimo_actix", lib_path!("fimo_actix")),
        );
        #[cfg(feature = "fimo_tasks_int")]
        paths.insert(
            <dyn IFimoTasks>::new_descriptor(),
            module_path("fimo_tasks", lib_path!("fimo_tasks")),
        );
        #[cfg(feature = "fimo_logging_int")]
        paths.insert(
            <dyn IFimoLogging>::new_descriptor(),
            module_path("fimo_logging", lib_path!("fimo_logging")),
        );

        let module_loader = RustLoader::new();
        let core_module = unsafe {
            module_loader.load_module_raw(
                paths
                    .get(&<dyn IFimoCore>::new_descriptor())
                    .unwrap()
                    .as_path(),
            )?
        };
        let core_instance = core_module.new_instance()?;
        let core_descriptor = core_instance
            .available_interfaces()
            .iter()
            .find(|interface| interface.name == <dyn IFimoCore>::NAME)
            .unwrap();

        let interface = core_instance.interface(core_descriptor)?;
        let core = fimo_module::try_downcast_arc(interface)?;

        Ok(Self {
            core,
            core_instance,
            paths,
        })
    }

    pub fn interface_path<'a, I: FimoInterface<'a> + ?Sized>(&self) -> Option<&Path> {
        self.paths.get(&I::new_descriptor()).map(|p| p.as_path())
    }

    pub fn core_interface(&self) -> ObjArc<DynObj<dyn IFimoCore>> {
        ObjArc::cast_super(self.core.clone())
    }

    pub fn new_interface<I>(
        &self,
    ) -> Result<InterfaceHandle<'_, DynObj<I>, DynObj<dyn IModuleRegistry + '_>>, Error>
    where
        I: CastInto<'static, dyn IModuleInterface>
            + Unsize<dyn IBase>
            + FimoInterface<'static>
            + ?Sized
            + 'static,
    {
        if TypeId::of::<I>() == TypeId::of::<dyn IFimoCore>() {
            panic!("Can not create core")
        }

        let module_registry = self.core.modules();
        let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
        let module = unsafe { loader.load_module_raw(self.interface_path::<I>().unwrap())? };

        let instance = module.new_instance()?;
        let interface_descriptor = instance
            .available_interfaces()
            .iter()
            .find(|interface| interface.name == I::NAME)
            .unwrap();
        let core_descriptor = self
            .core_instance
            .available_interfaces()
            .iter()
            .find(|interface| interface.name == <dyn IFimoCore>::NAME)
            .unwrap();

        let i = self.core_instance.interface(core_descriptor)?;

        instance.bind_interface(interface_descriptor, i)?;
        let interface = instance.interface(interface_descriptor)?;
        let interface: ObjArc<DynObj<I>> = fimo_module::try_downcast_arc(interface)?;
        let handle = module_registry.register_interface(true, interface)?;
        Ok(handle)
    }

    pub fn new_service<S>(&self) -> Result<&'static DynObj<S>, Error>
    where
        S: CastInto<'static, dyn IModuleInterface>
            + Unsize<dyn IBase>
            + FimoInterface<'static>
            + ?Sized
            + 'static,
    {
        if TypeId::of::<S>() == TypeId::of::<dyn IFimoCore>() {
            panic!("Can not create core")
        }

        let module_registry = self.core.modules();
        let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
        let module = unsafe { loader.load_module_raw(self.interface_path::<S>().unwrap())? };

        let instance = module.new_instance()?;
        let interface_descriptor = instance
            .available_interfaces()
            .iter()
            .find(|interface| {
                interface.name == S::NAME && S::VERSION.is_compatible(&interface.version)
            })
            .unwrap();

        let service = instance.interface(interface_descriptor)?;
        let service: ObjArc<DynObj<S>> = fimo_module::try_downcast_arc(service)?;
        let service = unsafe { &*ObjArc::into_raw(service) };
        module_registry.register_service(service)?;

        Ok(service)
    }
}

pub fn core_path() -> PathBuf {
    module_path("fimo_core", lib_path!("fimo_core"))
}

fn module_path(name: &str, lib_name: &str) -> PathBuf {
    let out_dir = std::env::var_os("FIMO_OUT_DIR").expect("FIMO_OUT_DIR env variable not set");
    let mut path = PathBuf::from(out_dir);
    path.push(name);
    path.push(lib_name);
    path
}
