//! Implementation of a generic rust module.
#![feature(maybe_uninit_extra)]
use fimo_module_core::rust::module_loader::RustModuleInnerCaster;
use fimo_module_core::rust::{
    module_loader::{RustModule, RustModuleInnerArc, RustModuleInnerVTable},
    Module, ModuleArc, ModuleCaster, ModuleInstance, ModuleInstanceArc, ModuleInstanceCaster,
    ModuleInstanceVTable, ModuleInterfaceArc, ModuleInterfaceWeak, ModuleVTable,
};
use fimo_module_core::{ModuleInfo, ModuleInterfaceDescriptor, ModulePtr};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::sync::{Arc, Weak};

const MODULE_VTABLE: ModuleVTable = ModuleVTable::new(
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };

        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            module
                .parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_raw_ptr()
        }
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };

        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            module
                .parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_raw_type_id()
        }
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };

        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            &*(module
                .parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_module_path() as *const _)
        }
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };
        &module.module_info
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };

        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            module
                .parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_module_loader()
        }
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };
        let parent = unsafe { module.parent.assume_init_ref().upgrade().unwrap() };
        (module.instance_builder)(parent).map(|i| {
            let (_, vtable) = (&**i).into_raw_parts();
            let caster = ModuleInstanceCaster::new(vtable);
            unsafe { ModuleInstanceArc::from_inner((i, caster)) }
        })
    },
);

const MODULE_INNER_VTABLE: RustModuleInnerVTable = RustModuleInnerVTable::new(
    |ptr, parent| {
        let module = unsafe { &mut *(ptr as *mut GenericModule) };
        module.parent = MaybeUninit::new(parent);
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const GenericModule) };
        &**module
    },
);

const MODULE_INSTANCE_VTABLE: ModuleInstanceVTable = ModuleInstanceVTable::new(
    |_ptr| ModulePtr::Slim(&MODULE_INSTANCE_VTABLE as *const _ as *const u8),
    |_ptr| "fimo::module_instance::generic",
    |ptr| {
        let inst = unsafe { &*(ptr as *const GenericModuleInstance) };
        let parent = inst.parent.clone();
        let (_, vtable) = (&**parent).into_raw_parts();
        let caster = ModuleCaster::new(vtable);

        unsafe { ModuleArc::from_inner((parent, caster)) }
    },
    |ptr| {
        let inst = unsafe { &*(ptr as *const GenericModuleInstance) };
        inst.get_available_interfaces()
    },
    |ptr, desc| {
        let inst = unsafe { &*(ptr as *const GenericModuleInstance) };
        let desc = unsafe { &*desc };

        inst.get_interface(desc).map_err(|e| Box::new(e) as _)
    },
    |ptr, desc| {
        let inst = unsafe { &*(ptr as *const GenericModuleInstance) };
        let desc = unsafe { &*desc };

        inst.get_interface_dependencies(desc)
            .map_or_else(|e| Err(Box::new(e) as _), |dep| Ok(dep as _))
    },
    |ptr, desc, interface| {
        let inst = unsafe { &*(ptr as *const GenericModuleInstance) };
        let desc = unsafe { &*desc };

        inst.set_dependency(desc, interface)
            .map_err(|e| Box::new(e) as _)
    },
);

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
    dependency_map: Mutex<HashMap<ModuleInterfaceDescriptor, Option<ModuleInterfaceWeak>>>,
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
    ptr: Option<ModuleInterfaceWeak>,
}

/// Builds a module instance.
pub type InstanceBuilder =
    fn(Arc<RustModule>) -> Result<Arc<GenericModuleInstance>, Box<dyn Error>>;

type InterfaceBuilder = fn(
    Arc<GenericModuleInstance>,
    &HashMap<ModuleInterfaceDescriptor, Option<ModuleInterfaceWeak>>,
) -> Result<ModuleInterfaceArc, Box<dyn Error>>;

impl GenericModule {
    /// Constructs a new `GenericModule`.
    pub fn new_inner(
        module_info: ModuleInfo,
        instance_builder: InstanceBuilder,
    ) -> RustModuleInnerArc {
        let module = Arc::new(Self {
            module_info,
            parent: MaybeUninit::uninit(),
            instance_builder,
        });

        let caster = RustModuleInnerCaster::new(&MODULE_INNER_VTABLE);
        unsafe { RustModuleInnerArc::from_inner((module, caster)) }
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
        let public_interfaces: Vec<_> = interfaces.keys().copied().collect();
        let interface: HashMap<_, _> = interfaces
            .iter()
            .map(|(descriptor, &(builder, _))| {
                let interface = Interface { builder, ptr: None };
                (*descriptor, interface)
            })
            .collect();

        let interface_dependencies: HashMap<_, _> = interfaces
            .into_iter()
            .map(|(descriptor, (_, dependencies))| (descriptor, dependencies))
            .collect();

        let mut dependency_map: HashMap<_, _> = HashMap::new();
        for (_, dependencies) in interface_dependencies.iter() {
            for dependency in dependencies {
                dependency_map.insert(*dependency, None);
            }
        }

        Arc::new(Self {
            public_interfaces,
            interfaces: Mutex::new(interface),
            interface_dependencies,
            dependency_map: Mutex::new(dependency_map),
            parent,
        })
    }

    /// Coerces the generic instance to a type-erased instance.
    pub fn as_module_instance_arc(this: Arc<Self>) -> ModuleInstanceArc {
        let caster = ModuleInstanceCaster::new(&MODULE_INSTANCE_VTABLE);
        unsafe { ModuleInstanceArc::from_inner((this, caster)) }
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

    /// Extracts an [`ModuleInterfaceArc`] to an interface,
    /// constructing it if it isn't alive.
    pub fn get_interface(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<ModuleInterfaceArc, GetInterfaceError> {
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
                    int.ptr = Some(ModuleInterfaceArc::downgrade(&arc));
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
        interface: ModuleInterfaceArc,
    ) -> Result<(), UnknownInterfaceError> {
        let mut guard = self.dependency_map.lock();
        if let Some(dep) = guard.get_mut(interface_desc) {
            *dep = Some(ModuleInterfaceArc::downgrade(&interface));
            Ok(())
        } else {
            Err(UnknownInterfaceError {
                interface: *interface_desc,
            })
        }
    }
}

impl Deref for GenericModule {
    type Target = Module;

    fn deref(&self) -> &Self::Target {
        let self_ptr = self as *const _ as *const ();
        let vtable = &MODULE_VTABLE;

        unsafe { &*Module::from_raw_parts(self_ptr, vtable) }
    }
}

impl Drop for GenericModule {
    fn drop(&mut self) {
        unsafe { self.parent.assume_init_drop() }
    }
}

impl Deref for GenericModuleInstance {
    type Target = ModuleInstance;

    fn deref(&self) -> &Self::Target {
        let self_ptr = self as *const _ as *const ();
        let vtable = &MODULE_INSTANCE_VTABLE;

        unsafe { &*ModuleInstance::from_raw_parts(self_ptr, vtable) }
    }
}

impl Debug for GenericModuleInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(GenericModuleInstance)")
    }
}

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
