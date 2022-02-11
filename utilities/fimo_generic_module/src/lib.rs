//! Implementation of a generic rust module.
use fimo_ffi::error::InnerError;
use fimo_ffi::marker::SendSyncMarker;
use fimo_ffi::vtable::{IBase, VTable};
use fimo_ffi::{ObjArc, ObjWeak, SpanInner};
use fimo_module::rust_loader::{IRustModuleInner, IRustModuleInnerVTable, IRustModuleParent};
use fimo_module::{
    impl_vtable, is_object, Error, ErrorKind, IModule, IModuleInstance, IModuleInstanceVTable,
    IModuleInterface, IModuleLoader, IModuleVTable, ModuleInfo, ModuleInterfaceDescriptor,
    PathChar,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt::Debug;
use std::mem::MaybeUninit;

/// Error resulting from an unknown interface.
#[derive(Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct UnknownInterfaceError {
    interface: ModuleInterfaceDescriptor,
}

struct Interface {
    builder: InterfaceBuilder,
    ptr: Option<ObjWeak<IModuleInterface>>,
}

/// Builds a module instance.
pub type InstanceBuilder =
    fn(ObjArc<IRustModuleParent>) -> Result<ObjArc<GenericModuleInstance>, Error>;

type InterfaceBuilder = fn(
    ObjArc<GenericModuleInstance>,
    &HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<IModuleInterface>>>,
) -> Result<ObjArc<IModuleInterface>, Error>;

/// A generic rust module.
#[derive(Debug)]
pub struct GenericModule {
    module_info: ModuleInfo,
    parent: MaybeUninit<ObjWeak<IRustModuleParent>>,
    instance_builder: InstanceBuilder,
}

impl GenericModule {
    /// Constructs a new `GenericModule`.
    pub fn new_inner(
        module_info: ModuleInfo,
        instance_builder: InstanceBuilder,
    ) -> ObjArc<IRustModuleInner> {
        let module = ObjArc::new(Self {
            module_info,
            parent: MaybeUninit::uninit(),
            instance_builder,
        });

        ObjArc::coerce_object(module)
    }
}

is_object! { #![uuid(0x73a946f0, 0x4fdf, 0x43b6, 0x8c0d, 0xfcc8bd039cea)] GenericModule }

impl_vtable! {
    impl IModuleVTable => GenericModule {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBase<SendSyncMarker> {
            let i: &IRustModuleInnerVTable = GenericModule::get_vtable();
            i.as_super()
        }

        unsafe extern "C" fn module_path(ptr: *const ()) -> SpanInner<PathChar, false> {
            let this = &*(ptr as *const GenericModule);
            let parent = this.parent.assume_init_ref().upgrade().unwrap();
            parent.module_path()
        }

        unsafe extern "C" fn module_info(ptr: *const ()) -> *const ModuleInfo {
            let this = &*(ptr as *const GenericModule);
            &this.module_info
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn module_loader(ptr: *const ()) -> &'static IModuleLoader {
            let this = &*(ptr as *const GenericModule);
            let parent = this.parent.assume_init_ref().upgrade().unwrap();
            parent.module_loader()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn new_instance(
            ptr: *const (),
        ) -> fimo_ffi::Result<ObjArc<IModuleInstance>, Error> {
            let this = &*(ptr as *const GenericModule);
            let parent = this.parent.assume_init_ref().upgrade().unwrap();
            (this.instance_builder)(parent)
                .map(ObjArc::<IModuleInstance>::coerce_object)
                .into()
        }
    }
}

impl_vtable! {
    impl inline IRustModuleInnerVTable => GenericModule {
        |_ptr| GenericModule::get_vtable(),
        |ptr, parent| unsafe {
            let this = &mut *(ptr as *mut GenericModule);
            this.parent = MaybeUninit::new(parent);
        },
    }
}

impl Drop for GenericModule {
    fn drop(&mut self) {
        unsafe { self.parent.assume_init_drop() }
    }
}

/// A generic rust module instance.
pub struct GenericModuleInstance {
    public_interfaces: Vec<ModuleInterfaceDescriptor>,
    interfaces: Mutex<HashMap<ModuleInterfaceDescriptor, Interface>>,
    interface_dependencies: HashMap<ModuleInterfaceDescriptor, Vec<ModuleInterfaceDescriptor>>,
    dependency_map: Mutex<HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<IModuleInterface>>>>,
    // Drop the parent for last
    parent: ObjArc<IRustModuleParent>,
}

impl GenericModuleInstance {
    /// Constructs a new `GenericModuleInstance`.
    pub fn new(
        parent: ObjArc<IRustModuleParent>,
        interfaces: HashMap<
            ModuleInterfaceDescriptor,
            (InterfaceBuilder, Vec<ModuleInterfaceDescriptor>),
        >,
    ) -> ObjArc<Self> {
        let public_interfaces: Vec<_> = interfaces.keys().cloned().collect();
        let interface: HashMap<_, _> = interfaces
            .iter()
            .map(|(descriptor, &(builder, _))| {
                let interface = Interface { builder, ptr: None };
                (descriptor.clone(), interface)
            })
            .collect();

        let interface_dependencies: HashMap<_, _> = interfaces
            .into_iter()
            .map(|(descriptor, (_, dependencies))| (descriptor, dependencies))
            .collect();

        let mut dependency_map: HashMap<_, _> = HashMap::new();
        for (_, dependencies) in interface_dependencies.iter() {
            for dependency in dependencies {
                dependency_map.insert(dependency.clone(), None);
            }
        }

        ObjArc::new(Self {
            public_interfaces,
            interfaces: Mutex::new(interface),
            interface_dependencies,
            dependency_map: Mutex::new(dependency_map),
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
                interface: interface.clone(),
            })
        }
    }

    /// Extracts an [`ObjArc<IModuleInterface>`] to an interface,
    /// constructing it if it isn't alive.
    pub fn get_interface(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<ObjArc<IModuleInterface>, GetInterfaceError> {
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
                ObjArc::increment_strong_count(self);
                ObjArc::from_raw(self)
            };

            (int.builder)(self_arc, &*dep_map).map_or_else(
                |e| {
                    Err(GetInterfaceError::ConstructionError {
                        interface: interface.clone(),
                        error: e,
                    })
                },
                |arc| {
                    int.ptr = Some(ObjArc::downgrade(&arc));
                    Ok(arc)
                },
            )
        } else {
            Err(GetInterfaceError::UnknownInterface(UnknownInterfaceError {
                interface: interface.clone(),
            }))
        }
    }

    /// Provides an interface to the module instance.
    pub fn set_dependency(
        &self,
        interface_desc: &ModuleInterfaceDescriptor,
        interface: ObjArc<IModuleInterface>,
    ) -> Result<(), UnknownInterfaceError> {
        let mut guard = self.dependency_map.lock();
        if let Some(dep) = guard.get_mut(interface_desc) {
            *dep = Some(ObjArc::downgrade(&interface));
            Ok(())
        } else {
            Err(UnknownInterfaceError {
                interface: interface_desc.clone(),
            })
        }
    }
}

is_object! { #![uuid(0xda5d5297, 0x2b5a, 0x4fe5, 0x8af1, 0xc8420d4d1edc)] GenericModuleInstance }

impl_vtable! {
    impl IModuleInstanceVTable => GenericModuleInstance {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBase<SendSyncMarker> {
            let i: &IModuleInstanceVTable = GenericModuleInstance::get_vtable();
            i.as_super()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn module(ptr: *const ()) -> ObjArc<IModule> {
            let this = &*(ptr as *const GenericModuleInstance);
            let parent = this.parent.clone();
            let (parent, alloc) = ObjArc::into_raw_parts(parent);
            let module = (*parent).as_module();
            ObjArc::from_raw_parts(module, alloc)
        }

        unsafe extern "C" fn available_interfaces(
            ptr: *const (),
        ) -> SpanInner<ModuleInterfaceDescriptor, false> {
            let this = &*(ptr as *const GenericModuleInstance);
            this.get_available_interfaces().into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn interface(
            ptr: *const (),
            desc: *const ModuleInterfaceDescriptor,
        ) -> fimo_module::Result<ObjArc<IModuleInterface>> {
            let this = &*(ptr as *const GenericModuleInstance);
            this.get_interface(&*desc)
                .map_err(|e| Error::new(ErrorKind::Internal, e))
                .into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn dependencies(
            ptr: *const (),
            desc: *const ModuleInterfaceDescriptor,
        ) -> fimo_module::Result<SpanInner<ModuleInterfaceDescriptor, false>> {
            let this = &*(ptr as *const GenericModuleInstance);
            this.get_interface_dependencies(&*desc).map_or_else(
                |e| fimo_module::Result::Err(Error::new(ErrorKind::NotFound, e)),
                |d| fimo_module::Result::Ok(d.into()),
            )
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn set_core(
            ptr: *const (),
            desc: *const ModuleInterfaceDescriptor,
            core: ObjArc<IModuleInterface>,
        ) -> fimo_module::Result<()> {
            let this = &*(ptr as *const GenericModuleInstance);
            this.set_dependency(&*desc, core)
                .map_err(|e| Error::new(ErrorKind::NotFound, e))
                .into()
        }
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
        error: Error,
    },
}

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

impl InnerError for GetInterfaceError {
    fn source(&self) -> Option<&fimo_ffi::IError> {
        match self {
            GetInterfaceError::UnknownInterface(_) => None,
            GetInterfaceError::ConstructionError { error, .. } => error.source(),
        }
    }
}
