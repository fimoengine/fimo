//! Implementation of generic modules.

use crate::{
    IModule, IModuleInstance, IModuleInterface, IModuleLoader, ModuleInfo,
    ModuleInterfaceDescriptor, PathChar,
};
use fimo_ffi::error::{Error, ErrorKind, IError};
use fimo_ffi::fmt::{IDebug, IDisplay};
use fimo_ffi::ptr::{coerce_obj, IBase, IBaseExt};
use fimo_ffi::{DynObj, ObjArc, ObjWeak, ObjectId};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::path::Path;

/// A generic module.
#[derive(ObjectId)]
#[fetch_vtable(uuid = "54ccece6-c648-42e5-807b-553049080256", interfaces(IModule))]
pub struct Module {
    info: ModuleInfo,
    root: Box<[PathChar]>,
    parent: &'static DynObj<dyn IModuleLoader>,
    build: Box<dyn Fn(ObjArc<Module>) -> crate::Result<ObjArc<Instance>> + Send + Sync>,
}

impl Module {
    /// Builds a new `Module`.
    pub fn new<F>(
        info: ModuleInfo,
        root: &Path,
        parent: &'static DynObj<dyn IModuleLoader>,
        f: F,
    ) -> ObjArc<Module>
    where
        F: Fn(ObjArc<Module>) -> crate::Result<ObjArc<Instance>> + Send + Sync + 'static,
    {
        let path: Box<[PathChar]>;
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let os_str = root.as_os_str();
            let buf: Vec<PathChar> = OsStrExt::encode_wide(os_str).collect();
            path = buf.into();
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            let os_str = root.as_os_str();
            let bytes = OsStrExt::as_bytes(os_str);
            path = bytes.into();
        }

        let build = Box::new(f);

        ObjArc::new(Self {
            info,
            root: path,
            parent,
            build,
        })
    }
}

impl Debug for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Module").field(&self.info).finish()
    }
}

impl IModule for Module {
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        coerce_obj::<_, dyn IModule + Send + Sync>(self).cast_super()
    }

    fn module_path_slice(&self) -> &[PathChar] {
        &self.root
    }

    fn module_info(&self) -> &ModuleInfo {
        &self.info
    }

    fn module_loader(&self) -> &'static DynObj<dyn IModuleLoader> {
        self.parent
    }

    fn new_instance(&self) -> crate::Result<ObjArc<DynObj<dyn IModuleInstance>>> {
        // A `Module` always lives inside an `ObjArc` so we try to manually clone it.
        let cloned = unsafe {
            let this = ObjArc::from_raw(self);
            let tmp = this.clone();
            std::mem::forget(this);
            tmp
        };

        let instance = (self.build)(cloned)?;
        Ok(ObjArc::coerce_obj(instance))
    }
}

/// Builder for an [`Instance`].
pub struct InstanceBuilder {
    parent: ObjArc<Module>,
    interfaces:
        HashMap<ModuleInterfaceDescriptor, (Vec<ModuleInterfaceDescriptor>, Box<InterfaceBuildFn>)>,
}

impl InstanceBuilder {
    /// Constructs a new `InstanceBuilder`.
    pub fn new(module: ObjArc<Module>) -> Self {
        Self {
            parent: module,
            interfaces: Default::default(),
        }
    }

    /// Adds a new interface without dependencies to the `InstanceBuilder`.
    pub fn empty<F: Send + 'static>(self, desc: ModuleInterfaceDescriptor, mut f: F) -> Self
    where
        F: FnMut(ObjArc<Instance>) -> crate::Result<ObjArc<DynObj<dyn IModuleInterface>>>,
    {
        let f = move |i, _| f(i);
        self.interface(desc, Default::default(), f)
    }

    /// Adds a new interface to the `InstanceBuilder`.
    pub fn interface<F: Send + 'static>(
        mut self,
        desc: ModuleInterfaceDescriptor,
        deps: &[ModuleInterfaceDescriptor],
        f: F,
    ) -> Self
    where
        F: FnMut(
            ObjArc<Instance>,
            Vec<ObjArc<DynObj<dyn IModuleInterface>>>,
        ) -> crate::Result<ObjArc<DynObj<dyn IModuleInterface>>>,
    {
        // remove duplicate dependencies.
        let mut set: HashSet<_> = deps.iter().cloned().collect();
        let deps: Vec<_> = set.drain().collect();
        let f = Box::new(f);
        self.interfaces.insert(desc, (deps, f));
        self
    }

    /// Builds an [`Instance`].
    pub fn build(self) -> ObjArc<Instance> {
        let InstanceBuilder { parent, interfaces } = self;
        let inter = interfaces;

        let available_interfaces = inter.keys().cloned().collect();
        let mut interfaces = HashMap::new();
        let mut interface_deps = HashMap::new();

        for (i, (deps, build)) in inter {
            let dependencies = deps.iter().map(|dep| (dep.clone(), None)).collect();

            let interface = Interface {
                build,
                dependencies,
                interface: None,
            };

            interfaces.insert(i.clone(), Mutex::new(interface));
            interface_deps.insert(i, deps);
        }

        ObjArc::new(Instance {
            available_interfaces,
            interfaces,
            interface_deps,
            parent,
        })
    }
}

impl Debug for InstanceBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(InstanceBuilder)")
    }
}

/// Implementation of a generic module instance.
#[derive(Debug, ObjectId)]
#[fetch_vtable(
    uuid = "8323b969-b6f8-475e-906b-bb29d8af7978",
    interfaces(IModuleInstance)
)]
pub struct Instance {
    available_interfaces: Vec<ModuleInterfaceDescriptor>,
    interfaces: HashMap<ModuleInterfaceDescriptor, Mutex<Interface>>,
    interface_deps: HashMap<ModuleInterfaceDescriptor, Vec<ModuleInterfaceDescriptor>>,
    // Drop the module for last
    parent: ObjArc<Module>,
}

impl IModuleInstance for Instance {
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        coerce_obj::<_, dyn IModuleInstance + Send + Sync>(self).cast_super()
    }

    fn module(&self) -> ObjArc<DynObj<dyn IModule>> {
        ObjArc::coerce_obj(self.parent.clone())
    }

    fn available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        &self.available_interfaces
    }

    fn interface(
        &self,
        i: &ModuleInterfaceDescriptor,
    ) -> crate::Result<ObjArc<DynObj<dyn IModuleInterface>>> {
        if let Some(interface) = self.interfaces.get(i) {
            // A `Instance` always lives inside an `ObjArc` so we try to manually clone it.
            let cloned = unsafe {
                let this = ObjArc::from_raw(self);
                let tmp = this.clone();
                std::mem::forget(this);
                tmp
            };

            let mut interface = interface.lock();
            interface.build_interface(cloned).map_err(|e| {
                let e = GetInterfaceError::ConstructionError {
                    interface: i.clone(),
                    error: e,
                };
                Error::new(ErrorKind::Unknown, e)
            })
        } else {
            let err = GetInterfaceError::UnknownInterface(UnknownInterfaceError {
                interface: i.clone(),
            });
            Err(Error::new(ErrorKind::NotFound, err))
        }
    }

    fn dependencies(
        &self,
        i: &ModuleInterfaceDescriptor,
    ) -> crate::Result<&[ModuleInterfaceDescriptor]> {
        if let Some(deps) = self.interface_deps.get(i) {
            Ok(deps.as_slice())
        } else {
            let err = UnknownInterfaceError {
                interface: i.clone(),
            };
            Err(Error::new(ErrorKind::NotFound, err))
        }
    }

    fn bind_interface(
        &self,
        desc: &ModuleInterfaceDescriptor,
        interface: ObjArc<DynObj<dyn IModuleInterface>>,
    ) -> crate::Result<()> {
        if let Some(inter) = self.interfaces.get(desc) {
            let mut inter = inter.lock();
            inter.set_dependency(interface)
        } else {
            let err = UnknownInterfaceError {
                interface: desc.clone(),
            };
            Err(Error::new(ErrorKind::NotFound, err))
        }
    }
}

type InterfaceBuildFn = dyn FnMut(
        ObjArc<Instance>,
        Vec<ObjArc<DynObj<dyn IModuleInterface>>>,
    ) -> crate::Result<ObjArc<DynObj<dyn IModuleInterface>>>
    + Send;
type DependencyMap =
    HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<DynObj<dyn IModuleInterface>>>>;

struct Interface {
    build: Box<InterfaceBuildFn>,
    dependencies: DependencyMap,
    interface: Option<ObjWeak<DynObj<dyn IModuleInterface>>>,
}

impl Interface {
    fn set_dependency(&mut self, i: ObjArc<DynObj<dyn IModuleInterface>>) -> crate::Result<()> {
        let desc = i.descriptor();
        if let Some(dep) = self.dependencies.get_mut(&desc) {
            *dep = Some(ObjArc::downgrade(&i));
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("unknown dependency: {:?}", desc),
            ))
        }
    }

    fn build_interface(
        &mut self,
        module: ObjArc<Instance>,
    ) -> crate::Result<ObjArc<DynObj<dyn IModuleInterface>>> {
        let i = self.interface.clone().map(|i| i.upgrade());
        if let Some(i) = i.flatten() {
            Ok(i)
        } else {
            let mut dependencies = Vec::with_capacity(self.dependencies.len());
            for (desc, dep) in &self.dependencies {
                if let Some(dep) = dep.as_ref().and_then(|d| d.upgrade()) {
                    dependencies.push(dep);
                } else {
                    return Err(Error::new(
                        ErrorKind::FailedPrecondition,
                        format!("missing dependency: {:?}", desc),
                    ));
                }
            }

            let i = (self.build)(module, dependencies)?;
            self.interface = Some(ObjArc::downgrade(&i));
            Ok(i)
        }
    }
}

impl Debug for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interface")
            .field("dependencies", &self.dependencies.keys())
            .finish()
    }
}

/// Error resulting from an unknown interface.
#[derive(Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct UnknownInterfaceError {
    interface: ModuleInterfaceDescriptor,
}

impl std::fmt::Display for UnknownInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown interface: {}", self.interface)
    }
}

impl IDebug for UnknownInterfaceError {
    fn fmt(&self, f: &mut fimo_ffi::fmt::Formatter<'_>) -> Result<(), fimo_ffi::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl IDisplay for UnknownInterfaceError {
    fn fmt(&self, f: &mut fimo_ffi::fmt::Formatter<'_>) -> Result<(), fimo_ffi::fmt::Error> {
        write!(f, "{}", self)
    }
}

impl IError for UnknownInterfaceError {}

/// Error that can occur when retrieving an interface from an [Instance].
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

impl IDebug for GetInterfaceError {
    fn fmt(&self, f: &mut fimo_ffi::fmt::Formatter<'_>) -> Result<(), fimo_ffi::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl IDisplay for GetInterfaceError {
    fn fmt(&self, f: &mut fimo_ffi::fmt::Formatter<'_>) -> Result<(), fimo_ffi::fmt::Error> {
        write!(f, "{}", self)
    }
}

impl IError for GetInterfaceError {
    fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
        match self {
            GetInterfaceError::UnknownInterface(_) => None,
            GetInterfaceError::ConstructionError { error, .. } => {
                error.get_ref().map(DynObj::cast_super)
            }
        }
    }
}
