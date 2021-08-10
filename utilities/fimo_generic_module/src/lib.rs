//! Implementation of a generic rust module.
#![feature(maybe_uninit_extra)]
use fimo_module_core::rust_loader::{RustModule, RustModuleExt};
use fimo_module_core::{
    Module, ModuleInfo, ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader,
    ModulePtr,
};
use parking_lot::Mutex;
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::{Arc, Weak};

/// A generic rust module.
#[derive(Debug)]
pub struct GenericModule {
    module_info: ModuleInfo,
    parent: MaybeUninit<Weak<RustModule>>,
    instance_builder: InstanceBuilder,
}

/// A generic rust module instance.
pub struct GenericModuleInstance {
    public_interfaces: Vec<ModuleInterfaceDescriptor>,
    interfaces: Mutex<HashMap<ModuleInterfaceDescriptor, Interface>>,
    interface_dependencies: HashMap<ModuleInterfaceDescriptor, Vec<ModuleInterfaceDescriptor>>,
    dependency_map: Mutex<HashMap<ModuleInterfaceDescriptor, Option<Weak<dyn ModuleInterface>>>>,
    // Drop the parent for last
    parent: Arc<RustModule>,
}

/// Error resulting from an unknown interface.
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct UnknownInterfaceError {
    interface: ModuleInterfaceDescriptor,
}

/// Error from the [GenericModuleInstance::get_interface] function.
#[derive(Debug)]
pub enum GetInterfaceError {
    /// Error resulting from an unknown interface.
    UnknownInterface(UnknownInterfaceError),
    /// Error resulting from the unsuccessful construction of an interface.
    ConstructionError {
        /// Interface tried to construct.
        interface: ModuleInterfaceDescriptor,
        /// Resulting error.
        error: Box<dyn Error>,
    },
}

struct Interface {
    builder: InterfaceBuilder,
    ptr: Option<Weak<dyn ModuleInterface>>,
}

/// Builds a module instance.
pub type InstanceBuilder =
    fn(Arc<RustModule>) -> Result<Arc<GenericModuleInstance>, Box<dyn Error>>;

type InterfaceBuilder = fn(
    Arc<dyn ModuleInstance>,
    &HashMap<ModuleInterfaceDescriptor, Option<Weak<dyn ModuleInterface>>>,
) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>;

impl GenericModule {
    /// Constructs a new `GenericModule`.
    pub fn new(module_info: ModuleInfo, instance_builder: InstanceBuilder) -> Box<Self> {
        Box::new(Self {
            module_info,
            parent: MaybeUninit::uninit(),
            instance_builder,
        })
    }
}

impl GenericModuleInstance {
    /// Constructs a new `GenericModuleInstance`.
    pub fn new(
        parent: Arc<RustModule>,
        interfaces: HashMap<
            ModuleInterfaceDescriptor,
            (InterfaceBuilder, Vec<ModuleInterfaceDescriptor>),
        >,
    ) -> Arc<Self> {
        let pub_inter = interfaces.keys().copied().collect();
        let inter = interfaces
            .iter()
            .map(|(m, i)| {
                (
                    *m,
                    Interface {
                        builder: i.0,
                        ptr: None,
                    },
                )
            })
            .collect();
        let dep_map = interfaces.keys().map(|i| (*i, None)).collect();
        let inter_dep = interfaces.into_iter().map(|(d, i)| (d, i.1)).collect();

        Arc::new(Self {
            public_interfaces: pub_inter,
            interfaces: Mutex::new(inter),
            interface_dependencies: inter_dep,
            dependency_map: Mutex::new(dep_map),
            parent,
        })
    }

    /// Extracts the available interfaces.
    pub fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        self.public_interfaces.as_slice()
    }

    /// Extracts the dependencies of an interface.
    pub fn get_interface_dependencies(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<&[ModuleInterfaceDescriptor], UnknownInterfaceError> {
        if let Some(dependencies) = self.interface_dependencies.get(interface) {
            Ok(dependencies.as_slice())
        } else {
            Err(UnknownInterfaceError {
                interface: *interface,
            })
        }
    }

    /// Extracts an `Arc<dyn ModuleInterface>` to an interface,
    /// constructing it if it isn't alive.
    pub fn get_interface(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface>, GetInterfaceError> {
        let mut guard = self.interfaces.lock();
        let dep_map = self.dependency_map.lock();
        if let Some(int) = guard.get_mut(interface) {
            if let Some(ptr) = &int.ptr {
                if let Some(arc) = ptr.upgrade() {
                    return Ok(arc);
                }
            }

            // SAFETY: A `GenericModuleInstance` is always in an `Arc`.
            let self_arc = unsafe {
                Arc::increment_strong_count(self as *const Self);
                Arc::from_raw(self as *const Self)
            };

            (int.builder)(self_arc, &*dep_map).map_or_else(
                |e| {
                    Err(GetInterfaceError::ConstructionError {
                        interface: *interface,
                        error: e,
                    })
                },
                |arc| {
                    int.ptr = Some(Arc::downgrade(&arc));
                    Ok(arc)
                },
            )
        } else {
            Err(GetInterfaceError::UnknownInterface(UnknownInterfaceError {
                interface: *interface,
            }))
        }
    }

    /// Provides an interface to the module instance.
    pub fn set_dependency(
        &self,
        interface_desc: &ModuleInterfaceDescriptor,
        interface: Arc<dyn ModuleInterface>,
    ) -> Result<(), UnknownInterfaceError> {
        let mut guard = self.dependency_map.lock();
        if let Some(dep) = guard.get_mut(interface_desc) {
            *dep = Some(Arc::downgrade(&interface));
            Ok(())
        } else {
            Err(UnknownInterfaceError {
                interface: *interface_desc,
            })
        }
    }
}

impl Drop for GenericModule {
    fn drop(&mut self) {
        unsafe { self.parent.assume_init_drop() }
    }
}

impl Module for GenericModule {
    fn get_raw_ptr(&self) -> ModulePtr {
        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            self.parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_raw_ptr()
        }
    }

    fn get_module_path(&self) -> &Path {
        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            &*(self
                .parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_module_path() as *const _)
        }
    }

    fn get_module_info(&self) -> &ModuleInfo {
        &self.module_info
    }

    fn get_module_loader(&self) -> &'static (dyn ModuleLoader + 'static) {
        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            self.parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_module_loader()
        }
    }

    fn create_instance(&self) -> Result<Arc<dyn ModuleInstance>, Box<dyn Error>> {
        let parent = unsafe { self.parent.assume_init_ref().upgrade().unwrap() };
        (self.instance_builder)(parent).map(|i| i as _)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl RustModuleExt for GenericModule {
    fn set_weak_parent_handle(&mut self, module: Weak<RustModule>) {
        self.parent = MaybeUninit::new(module);
    }

    fn as_module(&self) -> &(dyn Module + 'static) {
        self
    }

    fn as_module_mut(&mut self) -> &mut (dyn Module + 'static) {
        self
    }
}

impl Debug for GenericModuleInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(GenericModuleInstance)")
    }
}

impl ModuleInstance for GenericModuleInstance {
    fn get_raw_ptr(&self) -> ModulePtr {
        fimo_core_interface::to_fimo_module_instance_raw_ptr!(self)
    }

    fn get_module(&self) -> Arc<dyn Module> {
        self.parent.clone()
    }

    fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        self.get_available_interfaces()
    }

    fn get_interface(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
        self.get_interface(interface).map_err(|e| Box::new(e) as _)
    }

    fn get_interface_dependencies(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<&[ModuleInterfaceDescriptor], Box<dyn Error>> {
        self.get_interface_dependencies(interface)
            .map_err(|e| Box::new(e) as _)
    }

    fn set_dependency(
        &self,
        interface_desc: &ModuleInterfaceDescriptor,
        interface: Arc<dyn ModuleInterface>,
    ) -> Result<(), Box<dyn Error>> {
        self.set_dependency(interface_desc, interface)
            .map_err(|e| Box::new(e) as _)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

fimo_core_interface::impl_fimo_module_instance! {GenericModuleInstance}

impl fimo_core_interface::rust::FimoModuleInstanceExt for GenericModuleInstance {}

impl std::fmt::Display for UnknownInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown interface: {}", self.interface)
    }
}

impl Error for UnknownInterfaceError {}

impl std::fmt::Display for GetInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetInterfaceError::UnknownInterface(err) => std::fmt::Display::fmt(err, f),
            GetInterfaceError::ConstructionError { interface, error } => {
                write!(
                    f,
                    "construction error: interface: {}, error: `{}`",
                    interface, error
                )
            }
        }
    }
}

impl Error for GetInterfaceError {}
