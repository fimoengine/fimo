use fimo_ffi::ObjArc;
use fimo_module_core::rust_loader::{RustLoader, MODULE_LOADER_TYPE};
use fimo_module_core::{Error, ErrorKind, ModuleInterfaceDescriptor};
use fimo_module_core::{FimoInterface, IModuleInstance, IModuleInterface};
use std::any::TypeId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fimo_core_int::rust::module_registry::InterfaceHandle;
use fimo_core_int::rust::IFimoCore;

use fimo_actix_int::IFimoActix;
use fimo_tasks_int::rust::IFimoTasks;

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
    core: ObjArc<IFimoCore>,
    core_instance: ObjArc<IModuleInstance>,
    paths: BTreeMap<ModuleInterfaceDescriptor, PathBuf>,
}

impl ModuleDatabase {
    pub fn new() -> Result<Self, Error> {
        let mut paths = BTreeMap::new();
        paths.insert(
            IFimoCore::new_descriptor(),
            new_path(lib_path!("fimo_core"))?,
        );
        paths.insert(
            IFimoTasks::new_descriptor(),
            new_path(lib_path!("fimo_tasks"))?,
        );
        paths.insert(
            IFimoActix::new_descriptor(),
            new_path(lib_path!("fimo_actix"))?,
        );

        let module_loader = RustLoader::new();
        let core_module = unsafe {
            module_loader
                .load_module_raw(paths.get(&IFimoCore::new_descriptor()).unwrap().as_path())?
        };
        let core_instance = core_module.new_instance()?;
        let core_descriptor = core_instance
            .available_interfaces()
            .iter()
            .find(|interface| interface.name == IFimoCore::NAME)
            .unwrap();

        let interface = core_instance.interface(core_descriptor).into_rust()?;
        let core = IModuleInterface::try_downcast_arc(interface)?;

        Ok(Self {
            core,
            core_instance,
            paths,
        })
    }

    pub fn interface_path<I: FimoInterface + ?Sized>(&self) -> Option<&Path> {
        self.paths.get(&I::new_descriptor()).map(|p| p.as_path())
    }

    pub fn core_interface(&self) -> ObjArc<IFimoCore> {
        self.core.clone()
    }

    pub fn new_interface<I: 'static + FimoInterface + ?Sized>(
        &self,
    ) -> Result<(ObjArc<I>, InterfaceHandle<IModuleInterface>), Error> {
        if TypeId::of::<I>() == TypeId::of::<IFimoCore>() {
            panic!("Can not create core")
        }

        let module_registry = self.core.get_module_registry();
        let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
        let module = unsafe {
            loader
                .load_module_raw(self.interface_path::<I>().unwrap())
                .into_rust()?
        };

        let instance = module.new_instance().into_rust()?;
        let interface_descriptor = instance
            .available_interfaces()
            .iter()
            .find(|interface| interface.name == I::NAME)
            .unwrap();
        let core_descriptor = self
            .core_instance
            .available_interfaces()
            .iter()
            .find(|interface| interface.name == IFimoCore::NAME)
            .unwrap();

        let i = self.core_instance.interface(core_descriptor).into_rust()?;

        instance.set_core(core_descriptor, i).into_rust()?;
        let interface = instance.interface(interface_descriptor).into_rust()?;
        let handle = module_registry.register_interface(&I::new_descriptor(), interface.clone())?;
        let interface = IModuleInterface::try_downcast_arc(interface)?;
        Ok((interface, handle))
    }
}

pub fn core_path() -> &'static Path {
    const CORE_PATH: &str = lib_path!("fimo_core");
    Path::new(CORE_PATH)
}

fn new_path(path: &str) -> Result<PathBuf, Error> {
    let current = std::env::current_exe().map_err(|e| Error::new(ErrorKind::Internal, e))?;
    let artifact_dir = PathBuf::from(current.parent().unwrap().parent().unwrap());
    artifact_dir
        .join(path)
        .canonicalize()
        .map_err(|e| Error::new(ErrorKind::Internal, e))
}
