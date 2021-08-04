//! Implementation of the `ModuleRegistry` type.
use fimo_ffi_core::ArrayString;
use fimo_module_core::{ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader};
use serde::{Deserialize, Serialize};
use std::collections::{btree_map, hash_map, BTreeMap, HashMap};
use std::fs::File;
use std::io::BufReader;
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::Arc;

/// Path from module root to manifest file.
pub const MODULE_MANIFEST_PATH: &str = "module.json";

/// The module registry.
pub struct ModuleRegistry {
    loaders: HashMap<String, Arc<dyn ModuleLoader>>,
    interfaces: BTreeMap<ModuleInterfaceDescriptor, Arc<dyn ModuleInterface>>,
    loader_callbacks: HashMap<*const dyn ModuleLoader, Vec<Box<LoaderCallback>>>,
    interface_callbacks: HashMap<*const dyn ModuleInterface, Vec<Box<InterfaceCallback>>>,
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

/// Basic module information.
#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name.
    pub name: String,
    /// Module version.
    pub version: semver::Version,
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

/// Handle to a registered callback.
#[repr(transparent)]
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct CallbackHandle<T: ?Sized>(*const T);

/// Errors from the `ModuleRegistry`.
#[derive(Debug)]
pub enum ModuleRegistryError {
    /// The loader has not been registered.
    UnknownLoaderType(String),
    /// Tried to register a loader twice.
    DuplicateLoaderType(String),
    /// The interface has not been registered.
    UnknownInterface(ModuleInterfaceDescriptor),
    /// Tried to register an interface twice.
    DuplicateInterface(ModuleInterfaceDescriptor),
    /// De-/Serialisation error.
    SerdeError(serde_json::Error),
    /// IO error.
    IOError(std::io::Error),
}

/// Type of a loader callback.
pub type LoaderCallback = dyn FnOnce(Arc<dyn ModuleLoader>) + Sync + Send;

/// Type of an interface callback.
pub type InterfaceCallback = dyn FnOnce(Arc<dyn ModuleInterface>) + Sync + Send;

impl ModuleRegistry {
    /// Creates a new `ModuleRegistry`.
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut registry = Self {
            loaders: HashMap::new(),
            interfaces: BTreeMap::new(),
            loader_callbacks: HashMap::new(),
            interface_callbacks: HashMap::new(),
        };

        if cfg!(feature = "rust_module_loader") {
            registry
                .register_loader(
                    fimo_module_core::rust_loader::MODULE_LOADER_TYPE,
                    fimo_module_core::rust_loader::RustLoader::new(),
                )
                .unwrap();
        }

        if cfg!(feature = "ffi_module_loader") {
            registry
                .register_loader(
                    fimo_module_core::ffi_loader::MODULE_LOADER_TYPE,
                    fimo_module_core::ffi_loader::FFIModuleLoader::new(),
                )
                .unwrap();
        }

        registry
    }

    /// Registers a new module loader to the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    pub fn register_loader(
        &mut self,
        loader_type: impl AsRef<str>,
        loader: Arc<impl ModuleLoader + 'static>,
    ) -> Result<&Self, ModuleRegistryError> {
        let entry = self.loaders.entry(String::from(loader_type.as_ref()));

        if let hash_map::Entry::Occupied(_) = &entry {
            return Err(ModuleRegistryError::DuplicateLoaderType(String::from(
                loader_type.as_ref(),
            )));
        };

        let loader_ptr = Arc::as_ptr(&loader);

        entry.or_insert(loader);
        self.loader_callbacks.insert(loader_ptr, Vec::new());

        Ok(self)
    }

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before removing.
    pub fn unregister_loader(
        &mut self,
        loader_type: impl AsRef<str>,
    ) -> Result<&Self, ModuleRegistryError> {
        // Remove loader from registry
        let loader = match self.loaders.remove(loader_type.as_ref()) {
            None => {
                return Err(ModuleRegistryError::UnknownLoaderType(String::from(
                    loader_type.as_ref(),
                )));
            }
            Some(loader) => loader,
        };

        // Remove and call callbacks
        let loader_ptr = Arc::as_ptr(&loader);
        let callbacks = self.loader_callbacks.remove(&loader_ptr).unwrap();

        for callback in callbacks {
            callback(loader.clone())
        }

        Ok(self)
    }

    /// Registers a loader-removal callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the loader is removed.
    pub fn register_loader_callback(
        &mut self,
        loader_type: impl AsRef<str>,
        callback: Box<LoaderCallback>,
        callback_handle: &mut MaybeUninit<CallbackHandle<LoaderCallback>>,
    ) -> Result<&Self, ModuleRegistryError> {
        let loader_ptr = Arc::as_ptr(self.get_loader_ref_from_type(loader_type)?);
        let callback_ptr = &*callback as *const _;

        // Insert callback at the end
        let callbacks = self.loader_callbacks.get_mut(&loader_ptr).unwrap();
        callbacks.push(callback);

        callback_handle.write(CallbackHandle::new(callback_ptr));

        Ok(self)
    }

    /// Unregisters a loader-removal callback from the `ModuleRegistry`.
    ///
    /// The callback will not be called.
    pub fn unregister_loader_callback(
        &mut self,
        loader_type: impl AsRef<str>,
        callback_handle: CallbackHandle<LoaderCallback>,
    ) -> Result<&Self, ModuleRegistryError> {
        let loader_ptr = Arc::as_ptr(self.get_loader_ref_from_type(loader_type)?);

        // Remove callback from vec
        self.loader_callbacks
            .get_mut(&loader_ptr)
            .unwrap()
            .retain(|x| callback_handle.as_ptr() != &*x);

        Ok(self)
    }

    fn get_loader_ref_from_type(
        &self,
        loader_type: impl AsRef<str>,
    ) -> Result<&Arc<dyn ModuleLoader + 'static>, ModuleRegistryError> {
        match self.loaders.get(loader_type.as_ref()) {
            None => Err(ModuleRegistryError::UnknownLoaderType(String::from(
                loader_type.as_ref(),
            ))),
            Some(loader) => Ok(loader),
        }
    }

    /// Extracts a loader from the `ModuleRegistry` using the registration type.
    pub fn get_loader_from_type(
        &self,
        loader_type: impl AsRef<str>,
    ) -> Result<Arc<dyn ModuleLoader + 'static>, ModuleRegistryError> {
        self.get_loader_ref_from_type(loader_type)
            .map(|loader| loader.clone())
    }

    /// Registers a new interface to the `ModuleRegistry`.
    pub fn register_interface(
        &mut self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
        interface: Arc<impl ModuleInterface + 'static>,
    ) -> Result<&Self, ModuleRegistryError> {
        // Check if the interface already exists
        let entry = self.interfaces.entry(*descriptor.as_ref());

        if let btree_map::Entry::Occupied(_) = &entry {
            return Err(ModuleRegistryError::DuplicateInterface(
                *descriptor.as_ref(),
            ));
        };

        let interface_ptr = Arc::as_ptr(&interface);

        entry.or_insert(interface);
        self.interface_callbacks.insert(interface_ptr, Vec::new());

        Ok(self)
    }

    /// Unregisters an existing interface from the `ModuleRegistry`.
    ///
    /// This function calls the interface-remove callbacks that are registered
    /// with the interface before removing it.
    pub fn unregister_interface(
        &mut self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
    ) -> Result<&Self, ModuleRegistryError> {
        // Remove interface from registry
        let interface = match self.interfaces.remove(descriptor.as_ref()) {
            None => return Err(ModuleRegistryError::UnknownInterface(*descriptor.as_ref())),
            Some(interface) => interface,
        };

        // Remove and call callbacks
        let interface_ptr = Arc::as_ptr(&interface);
        let callbacks = self.interface_callbacks.remove(&interface_ptr).unwrap();

        for callback in callbacks {
            callback(interface.clone())
        }

        Ok(self)
    }

    /// Registers an interface-removed callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
    pub fn register_interface_callback(
        &mut self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
        callback: Box<InterfaceCallback>,
        callback_handle: &mut MaybeUninit<CallbackHandle<InterfaceCallback>>,
    ) -> Result<&Self, ModuleRegistryError> {
        let interface_ptr = Arc::as_ptr(self.get_interface_ref_from_descriptor(descriptor)?);
        let callback_ptr = &*callback as *const _;

        // Insert callback at the end
        let callbacks = self.interface_callbacks.get_mut(&interface_ptr).unwrap();
        callbacks.push(callback);

        callback_handle.write(CallbackHandle::new(callback_ptr));

        Ok(self)
    }

    /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
    pub fn unregister_interface_callback(
        &mut self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
        callback_handle: CallbackHandle<InterfaceCallback>,
    ) -> Result<&Self, ModuleRegistryError> {
        let interface_ptr = Arc::as_ptr(self.get_interface_ref_from_descriptor(descriptor)?);

        // Remove callback from vec
        self.interface_callbacks
            .get_mut(&interface_ptr)
            .unwrap()
            .retain(|x| callback_handle.as_ptr() != &*x);

        Ok(self)
    }

    fn get_interface_ref_from_descriptor(
        &self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
    ) -> Result<&Arc<dyn ModuleInterface + 'static>, ModuleRegistryError> {
        match self.interfaces.get(descriptor.as_ref()) {
            None => Err(ModuleRegistryError::UnknownInterface(*descriptor.as_ref())),
            Some(interface) => Ok(interface),
        }
    }

    /// Extracts an interface from the `ModuleRegistry`.
    pub fn get_interface_from_descriptor(
        &self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
    ) -> Result<Arc<dyn ModuleInterface + 'static>, ModuleRegistryError> {
        self.get_interface_ref_from_descriptor(descriptor)
            .map(|interface| interface.clone())
    }

    /// Extracts all interface descriptors with the same name.
    pub fn get_interface_descriptors_from_name(
        &self,
        interface_name: impl AsRef<str>,
    ) -> Vec<ModuleInterfaceDescriptor> {
        self.interfaces
            .keys()
            .filter(|x| x.name == interface_name.as_ref())
            .cloned()
            .collect()
    }

    /// Extracts all descriptors of compatible interfaces.
    pub fn get_compatible_interface_descriptors(
        &self,
        interface_name: impl AsRef<str>,
        interface_version: impl AsRef<fimo_version_core::Version>,
        interface_extensions: impl AsRef<[ArrayString<32>]>,
    ) -> Vec<ModuleInterfaceDescriptor> {
        self.interfaces
            .keys()
            .filter(|x| {
                x.name == interface_name.as_ref()
                    && interface_version.as_ref().is_compatible(&x.version)
                    && interface_extensions
                        .as_ref()
                        .iter()
                        .all(|ext| x.extensions.contains(ext))
            })
            .cloned()
            .collect()
    }
}

impl ModuleManifest {
    /// Tries to load the manifest from a module.
    pub fn load_from_module(module_path: impl AsRef<Path>) -> Result<Self, ModuleRegistryError> {
        let manifest_path = module_path.as_ref().join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path)?;
        serde_json::from_reader(BufReader::new(file)).map_err(From::from)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> CallbackHandle<T> {
    fn new(id: *const T) -> Self {
        Self { 0: id }
    }

    fn as_ptr(&self) -> *const T {
        self.0
    }
}

impl std::fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(ModuleRegistry)")
    }
}

impl std::error::Error for ModuleRegistryError {}

impl std::fmt::Display for ModuleRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleRegistryError::UnknownLoaderType(loader_type) => {
                write!(f, "unknown loader type: {}", loader_type)
            }
            ModuleRegistryError::DuplicateLoaderType(loader_type) => {
                write!(f, "duplicated loader type: {}", loader_type)
            }
            ModuleRegistryError::UnknownInterface(interface) => {
                write!(f, "unknown interface: {}", interface)
            }
            ModuleRegistryError::DuplicateInterface(interface) => {
                write!(f, "duplicated interface: {}", interface)
            }
            ModuleRegistryError::SerdeError(err) => {
                write!(f, "de-/serialisation error: {}", err)
            }
            ModuleRegistryError::IOError(err) => {
                write!(f, "io error: {}", err)
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
