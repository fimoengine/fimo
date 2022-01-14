//! Implementation of the `ModuleRegistry` type.
use fimo_core_interface::rust::module_registry::{
    IModuleRegistryInner, InterfaceCallback, InterfaceCallbackId, InterfaceId, LoaderCallback,
    LoaderCallbackId, LoaderId, ModuleRegistryInnerVTable, ModuleRegistryVTable,
};
use fimo_ffi::object::{CoerceObject, CoerceObjectMut, ObjectWrapper};
use fimo_ffi::vtable::ObjectID;
use fimo_ffi::ObjArc;
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    Error, ErrorKind, IModuleInterface, IModuleLoader, ModuleInterfaceDescriptor,
};
use fimo_version_core::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::iter::Step;
use std::ops::RangeFrom;
use std::path::Path;

/// Path from module root to manifest file.
pub const MODULE_MANIFEST_PATH: &str = "module.json";

/// The module registry.
pub struct ModuleRegistry {
    inner: parking_lot::Mutex<ModuleRegistryInner>,
}

sa::assert_impl_all!(ModuleRegistry: Send, Sync);

impl ModuleRegistry {
    /// Constructs a new `ModuleRegistry`.
    #[inline]
    pub fn new() -> Self {
        let registry = Self {
            inner: parking_lot::Mutex::new(ModuleRegistryInner::new()),
        };

        use fimo_core_interface::rust::module_registry::IModuleRegistry;
        let i_registry = IModuleRegistry::from_object(registry.coerce_obj());

        if cfg!(feature = "rust_module_loader") {
            let handle = i_registry
                .register_loader(
                    fimo_module_core::rust_loader::MODULE_LOADER_TYPE,
                    fimo_module_core::rust_loader::RustLoader::new(),
                )
                .unwrap();
            std::mem::forget(handle);
        }

        registry
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectID for ModuleRegistry {
    const OBJECT_ID: &'static str = "fimo::modules::core::module::module_registry";
}

impl CoerceObject<ModuleRegistryVTable> for ModuleRegistry {
    fn get_vtable() -> &'static ModuleRegistryVTable {
        static VTABLE: ModuleRegistryVTable =
            ModuleRegistryVTable::new::<ModuleRegistry>(|ptr, f| {
                let registry = unsafe { &*(ptr as *const ModuleRegistry) };
                let mut inner = registry.inner.lock();
                let inner = inner.coerce_obj_mut();
                let inner = IModuleRegistryInner::from_object_mut(inner);
                unsafe { f.call_once((inner,)) }
            });
        &VTABLE
    }
}

impl std::fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(ModuleRegistry)")
    }
}

struct ModuleRegistryInner {
    loader_id_gen: RangeFrom<IdWrapper<LoaderId>>,
    loader_callback_id_gen: RangeFrom<IdWrapper<LoaderCallbackId>>,
    interface_id_gen: RangeFrom<IdWrapper<InterfaceId>>,
    interface_callback_id_gen: RangeFrom<IdWrapper<InterfaceCallbackId>>,
    loaders: BTreeMap<LoaderId, LoaderCollection>,
    loader_type_map: BTreeMap<String, LoaderId>,
    loader_callback_map: BTreeMap<LoaderCallbackId, LoaderId>,
    interfaces: BTreeMap<InterfaceId, InterfaceCollection>,
    interface_map: BTreeMap<ModuleInterfaceDescriptor, InterfaceId>,
    interface_callback_map: BTreeMap<InterfaceCallbackId, InterfaceId>,
}

struct LoaderCollection {
    loader: &'static IModuleLoader,
    callbacks: BTreeMap<LoaderCallbackId, LoaderCallback>,
}

struct InterfaceCollection {
    interface: ObjArc<IModuleInterface>,
    callbacks: BTreeMap<InterfaceCallbackId, InterfaceCallback>,
}

impl ModuleRegistryInner {
    #[inline]
    fn new() -> Self {
        Self {
            loader_id_gen: RangeFrom {
                start: Default::default(),
            },
            loader_callback_id_gen: RangeFrom {
                start: Default::default(),
            },
            interface_id_gen: RangeFrom {
                start: Default::default(),
            },
            interface_callback_id_gen: RangeFrom {
                start: Default::default(),
            },
            loaders: Default::default(),
            loader_type_map: Default::default(),
            loader_callback_map: Default::default(),
            interfaces: Default::default(),
            interface_map: Default::default(),
            interface_callback_map: Default::default(),
        }
    }

    #[inline]
    fn register_loader(
        &mut self,
        r#type: &str,
        loader: &'static IModuleLoader,
    ) -> Result<LoaderId, ModuleRegistryError> {
        if self.loader_type_map.contains_key(r#type) {
            return Err(ModuleRegistryError::DuplicateLoaderType(String::from(
                r#type,
            )));
        }

        if let Some(id) = self.loader_id_gen.next() {
            self.loaders.insert(
                id.get(),
                LoaderCollection {
                    loader,
                    callbacks: BTreeMap::new(),
                },
            );

            self.loader_type_map.insert(String::from(r#type), id.get());
            Ok(id.get())
        } else {
            Err(ModuleRegistryError::IdExhaustion)
        }
    }

    #[inline]
    fn unregister_loader(
        &mut self,
        id: LoaderId,
    ) -> Result<&'static IModuleLoader, ModuleRegistryError> {
        let loader = self.loaders.remove(&id);

        if loader.is_none() {
            return Err(ModuleRegistryError::UnknownLoaderId(id));
        }

        let LoaderCollection { loader, callbacks } = loader.unwrap();
        self.loader_type_map.retain(|_, l_id| *l_id != id);

        let i_registry = IModuleRegistryInner::from_object_mut(self.coerce_obj_mut());
        for (_, callback) in callbacks {
            callback(i_registry, loader)
        }

        Ok(loader)
    }

    #[inline]
    fn register_loader_callback(
        &mut self,
        r#type: &str,
        callback: LoaderCallback,
    ) -> Result<LoaderCallbackId, ModuleRegistryError> {
        if let Some(id) = self.loader_callback_id_gen.next() {
            let LoaderCollection { callbacks, .. } = self.get_loader_from_type_inner_mut(r#type)?;
            callbacks.insert(id.get(), callback);

            let loader_id = unsafe { std::ptr::read(self.loader_type_map.get(r#type).unwrap()) };
            self.loader_callback_map.insert(id.get(), loader_id);

            Ok(id.get())
        } else {
            Err(ModuleRegistryError::IdExhaustion)
        }
    }

    #[inline]
    fn unregister_loader_callback(
        &mut self,
        id: LoaderCallbackId,
    ) -> Result<(), ModuleRegistryError> {
        if let Some(l_id) = self.loader_callback_map.remove(&id) {
            let collection = self.loaders.get_mut(&l_id);
            if collection.is_none() {
                return Ok(());
            }

            let LoaderCollection { callbacks, .. } = collection.unwrap();
            callbacks.remove(&id);

            Ok(())
        } else {
            Err(ModuleRegistryError::UnknownLoaderCallbackId(id))
        }
    }

    #[inline]
    fn get_loader_from_type(
        &self,
        r#type: &str,
    ) -> Result<&'static IModuleLoader, ModuleRegistryError> {
        let LoaderCollection { loader, .. } = self.get_loader_from_type_inner(r#type)?;
        Ok(*loader)
    }

    #[inline]
    fn get_loader_from_type_inner(
        &self,
        r#type: &str,
    ) -> Result<&LoaderCollection, ModuleRegistryError> {
        if let Some(id) = self.loader_type_map.get(r#type) {
            Ok(self.loaders.get(id).unwrap())
        } else {
            Err(ModuleRegistryError::UnknownLoaderType(String::from(r#type)))
        }
    }

    #[inline]
    fn get_loader_from_type_inner_mut(
        &mut self,
        r#type: &str,
    ) -> Result<&mut LoaderCollection, ModuleRegistryError> {
        if let Some(id) = self.loader_type_map.get(r#type) {
            Ok(self.loaders.get_mut(id).unwrap())
        } else {
            Err(ModuleRegistryError::UnknownLoaderType(String::from(r#type)))
        }
    }

    #[inline]
    fn register_interface(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: ObjArc<IModuleInterface>,
    ) -> Result<InterfaceId, ModuleRegistryError> {
        if self.interface_map.contains_key(descriptor) {
            return Err(ModuleRegistryError::DuplicateInterface(*descriptor));
        }

        if let Some(id) = self.interface_id_gen.next() {
            self.interfaces.insert(
                id.get(),
                InterfaceCollection {
                    interface,
                    callbacks: Default::default(),
                },
            );

            self.interface_map.insert(*descriptor, id.get());
            Ok(id.get())
        } else {
            Err(ModuleRegistryError::IdExhaustion)
        }
    }

    #[inline]
    fn unregister_interface(
        &mut self,
        id: InterfaceId,
    ) -> Result<ObjArc<IModuleInterface>, ModuleRegistryError> {
        let interface = self.interfaces.remove(&id);

        if interface.is_none() {
            return Err(ModuleRegistryError::UnknownInterfaceId(id));
        }

        let InterfaceCollection {
            interface,
            callbacks,
        } = interface.unwrap();
        self.interface_map.retain(|_, i_id| *i_id != id);

        let i_registry = IModuleRegistryInner::from_object_mut(self.coerce_obj_mut());
        for (_, callback) in callbacks {
            callback(i_registry, interface.clone())
        }

        Ok(interface)
    }

    #[inline]
    fn register_interface_callback(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: InterfaceCallback,
    ) -> Result<InterfaceCallbackId, ModuleRegistryError> {
        if let Some(id) = self.interface_callback_id_gen.next() {
            let InterfaceCollection { callbacks, .. } =
                self.get_interface_from_descriptor_inner_mut(descriptor)?;
            callbacks.insert(id.get(), callback);

            let interface_id =
                unsafe { std::ptr::read(self.interface_map.get(descriptor).unwrap()) };
            self.interface_callback_map.insert(id.get(), interface_id);

            Ok(id.get())
        } else {
            Err(ModuleRegistryError::IdExhaustion)
        }
    }

    #[inline]
    fn unregister_interface_callback(
        &mut self,
        id: InterfaceCallbackId,
    ) -> Result<(), ModuleRegistryError> {
        if let Some(l_id) = self.interface_callback_map.remove(&id) {
            let collection = self.interfaces.get_mut(&l_id);
            if collection.is_none() {
                return Ok(());
            }

            let InterfaceCollection { callbacks, .. } = collection.unwrap();
            callbacks.remove(&id);

            Ok(())
        } else {
            Err(ModuleRegistryError::UnknownInterfaceCallbackId(id))
        }
    }

    #[inline]
    fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<ObjArc<IModuleInterface>, ModuleRegistryError> {
        let InterfaceCollection { interface, .. } =
            self.get_interface_from_descriptor_inner(descriptor)?;
        Ok(interface.clone())
    }

    #[inline]
    fn get_interface_from_descriptor_inner(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<&InterfaceCollection, ModuleRegistryError> {
        if let Some(id) = self.interface_map.get(descriptor) {
            Ok(self.interfaces.get(id).unwrap())
        } else {
            Err(ModuleRegistryError::UnknownInterface(*descriptor))
        }
    }

    #[inline]
    fn get_interface_from_descriptor_inner_mut(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<&mut InterfaceCollection, ModuleRegistryError> {
        if let Some(id) = self.interface_map.get(descriptor) {
            Ok(self.interfaces.get_mut(id).unwrap())
        } else {
            Err(ModuleRegistryError::UnknownInterface(*descriptor))
        }
    }

    #[inline]
    fn get_interface_descriptors_from_name(&self, name: &str) -> Vec<ModuleInterfaceDescriptor> {
        self.interface_map
            .keys()
            .filter(|x| x.name == name)
            .cloned()
            .collect()
    }

    #[inline]
    fn get_compatible_interface_descriptors(
        &self,
        name: &str,
        version: &Version,
        extensions: &[ArrayString<128>],
    ) -> Vec<ModuleInterfaceDescriptor> {
        self.interface_map
            .keys()
            .filter(|x| {
                x.name == name
                    && version.is_compatible(&x.version)
                    && extensions
                        .as_ref()
                        .iter()
                        .all(|ext| x.extensions.contains(ext))
            })
            .cloned()
            .collect()
    }
}

impl ObjectID for ModuleRegistryInner {
    const OBJECT_ID: &'static str = "fimo::modules::core::module::module_registry_inner";
}

impl CoerceObject<ModuleRegistryInnerVTable> for ModuleRegistryInner {
    fn get_vtable() -> &'static ModuleRegistryInnerVTable {
        static VTABLE: ModuleRegistryInnerVTable =
            ModuleRegistryInnerVTable::new::<ModuleRegistryInner>(
                |ptr, r#type, loader| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    let r#type = unsafe { &*r#type };
                    registry
                        .register_loader(r#type, loader)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, id| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    registry
                        .unregister_loader(id)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, r#type, callback| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    let r#type = unsafe { &*r#type };
                    registry
                        .register_loader_callback(r#type, callback)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, id| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    registry
                        .unregister_loader_callback(id)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, r#type| {
                    let registry = unsafe { &*(ptr as *const ModuleRegistryInner) };
                    let r#type = unsafe { &*r#type };
                    registry
                        .get_loader_from_type(r#type)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, descriptor, interface| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    let descriptor = unsafe { &*descriptor };
                    registry
                        .register_interface(descriptor, interface)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, id| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    registry
                        .unregister_interface(id)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, descriptor, callback| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    let descriptor = unsafe { &*descriptor };
                    registry
                        .register_interface_callback(descriptor, callback)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, id| {
                    let registry = unsafe { &mut *(ptr as *mut ModuleRegistryInner) };
                    registry
                        .unregister_interface_callback(id)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, descriptor| {
                    let registry = unsafe { &*(ptr as *const ModuleRegistryInner) };
                    let descriptor = unsafe { &*descriptor };
                    registry
                        .get_interface_from_descriptor(descriptor)
                        .map_err(|e| Error::new(ErrorKind::Unknown, e))
                },
                |ptr, name| {
                    let registry = unsafe { &*(ptr as *const ModuleRegistryInner) };
                    let name = unsafe { &*name };
                    registry.get_interface_descriptors_from_name(name)
                },
                |ptr, name, version, extensions| {
                    let registry = unsafe { &*(ptr as *const ModuleRegistryInner) };
                    let name = unsafe { &*name };
                    let version = unsafe { &*version };
                    let extensions = unsafe { &*extensions };
                    registry.get_compatible_interface_descriptors(name, version, extensions)
                },
            );
        &VTABLE
    }
}

impl CoerceObjectMut<ModuleRegistryInnerVTable> for ModuleRegistryInner {}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct IdWrapper<T> {
    id: T,
}

impl<T> IdWrapper<T> {
    fn get(&self) -> T {
        unsafe { std::ptr::read(&self.id) }
    }
}

impl<T> Clone for IdWrapper<T> {
    fn clone(&self) -> Self {
        Self { id: self.get() }
    }
}

impl Default for IdWrapper<LoaderId> {
    fn default() -> Self {
        Self {
            id: unsafe { LoaderId::from_raw(0) },
        }
    }
}

impl Default for IdWrapper<LoaderCallbackId> {
    fn default() -> Self {
        Self {
            id: unsafe { LoaderCallbackId::from_usize(0) },
        }
    }
}

impl Default for IdWrapper<InterfaceId> {
    fn default() -> Self {
        Self {
            id: unsafe { InterfaceId::from_raw(0) },
        }
    }
}

impl Default for IdWrapper<InterfaceCallbackId> {
    fn default() -> Self {
        Self {
            id: unsafe { InterfaceCallbackId::from_usize(0) },
        }
    }
}

impl Step for IdWrapper<LoaderId> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        let start = start.get().into();
        let end = end.get().into();
        usize::steps_between(&start, &end)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::forward_checked(start, count).map(|id| Self {
            id: unsafe { LoaderId::from_raw(id) },
        })
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::backward_checked(start, count).map(|id| Self {
            id: unsafe { LoaderId::from_raw(id) },
        })
    }
}

impl Step for IdWrapper<LoaderCallbackId> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        let start = start.get().into();
        let end = end.get().into();
        usize::steps_between(&start, &end)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::forward_checked(start, count).map(|id| Self {
            id: unsafe { LoaderCallbackId::from_usize(id) },
        })
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::backward_checked(start, count).map(|id| Self {
            id: unsafe { LoaderCallbackId::from_usize(id) },
        })
    }
}

impl Step for IdWrapper<InterfaceId> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        let start = start.get().into();
        let end = end.get().into();
        usize::steps_between(&start, &end)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::forward_checked(start, count).map(|id| Self {
            id: unsafe { InterfaceId::from_raw(id) },
        })
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::backward_checked(start, count).map(|id| Self {
            id: unsafe { InterfaceId::from_raw(id) },
        })
    }
}

impl Step for IdWrapper<InterfaceCallbackId> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        let start = start.get().into();
        let end = end.get().into();
        usize::steps_between(&start, &end)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::forward_checked(start, count).map(|id| Self {
            id: unsafe { InterfaceCallbackId::from_usize(id) },
        })
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let start = start.get().into();
        usize::backward_checked(start, count).map(|id| Self {
            id: unsafe { InterfaceCallbackId::from_usize(id) },
        })
    }
}

/// Basic module information.
#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name.
    pub name: String,
    /// Module version.
    pub version: semver::Version,
}

/// The manifest of a module.
#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "schema")]
pub enum ModuleManifest {
    /// Version `0` manifest schema.
    #[serde(rename = "0")]
    V0 {
        /// Module information.
        info: ModuleInfo,
        /// Required module loader.
        loader: String,
        /// Required dependencies.
        load_deps: Vec<ModuleInterfaceInfo>,
    },
}

impl ModuleManifest {
    /// Tries to load the manifest from a module.
    pub fn load_from_module(module_path: impl AsRef<Path>) -> Result<Self, ModuleRegistryError> {
        let manifest_path = module_path.as_ref().join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path)?;
        serde_json::from_reader(BufReader::new(file)).map_err(From::from)
    }
}

/// Basic module interface info.
#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInterfaceInfo {
    /// Name of the interface.
    pub name: String,
    /// Version of the interface.
    pub version: fimo_version_core::Version,
    /// Interface extensions.
    pub extensions: Vec<String>,
}

/// Errors from the `ModuleRegistry`.
#[derive(Debug)]
pub enum ModuleRegistryError {
    /// The loader id does not exist.
    UnknownLoaderId(LoaderId),
    /// The loader has not been registered.
    UnknownLoaderType(String),
    /// Tried to register a loader twice.
    DuplicateLoaderType(String),
    /// The loader callback id does not exist.
    UnknownLoaderCallbackId(LoaderCallbackId),
    /// The interface id does not exist.
    UnknownInterfaceId(InterfaceId),
    /// The interface has not been registered.
    UnknownInterface(ModuleInterfaceDescriptor),
    /// Tried to register an interface twice.
    DuplicateInterface(ModuleInterfaceDescriptor),
    /// The interface callback id does not exist.
    UnknownInterfaceCallbackId(InterfaceCallbackId),
    /// De-/Serialisation error.
    SerdeError(serde_json::Error),
    /// IO error.
    IOError(std::io::Error),
    /// Exhausted all possible ids
    IdExhaustion,
}

impl std::error::Error for ModuleRegistryError {}

impl std::fmt::Display for ModuleRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleRegistryError::UnknownLoaderId(id) => {
                write!(f, "unknown loader id: {:?}", id)
            }
            ModuleRegistryError::UnknownLoaderType(loader_type) => {
                write!(f, "unknown loader type: {}", loader_type)
            }
            ModuleRegistryError::DuplicateLoaderType(loader_type) => {
                write!(f, "duplicated loader type: {}", loader_type)
            }
            ModuleRegistryError::UnknownLoaderCallbackId(id) => {
                write!(f, "unknown loader callback id: {:?}", id)
            }
            ModuleRegistryError::UnknownInterfaceId(id) => {
                write!(f, "unknown interface id: {:?}", id)
            }
            ModuleRegistryError::UnknownInterface(interface) => {
                write!(f, "unknown interface: {}", interface)
            }
            ModuleRegistryError::DuplicateInterface(interface) => {
                write!(f, "duplicated interface: {}", interface)
            }
            ModuleRegistryError::UnknownInterfaceCallbackId(id) => {
                write!(f, "unknown interface callback id: {:?}", id)
            }
            ModuleRegistryError::SerdeError(err) => {
                write!(f, "de-/serialisation error: {}", err)
            }
            ModuleRegistryError::IOError(err) => {
                write!(f, "io error: {}", err)
            }
            ModuleRegistryError::IdExhaustion => {
                write!(f, "no more ids available")
            }
        }
    }
}

impl From<serde_json::Error> for ModuleRegistryError {
    fn from(err: serde_json::Error) -> Self {
        ModuleRegistryError::SerdeError(err)
    }
}

impl From<std::io::Error> for ModuleRegistryError {
    fn from(err: std::io::Error) -> Self {
        ModuleRegistryError::IOError(err)
    }
}
