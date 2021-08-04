use fimo_module_core::{ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader};
use serde::{Deserialize, Serialize};
use std::collections::{btree_map, hash_map, BTreeMap, HashMap};
use std::fs::File;
use std::io::BufReader;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::Arc;

pub const MODULE_MANIFEST_PATH: &str = "module.json";

#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "schema")]
pub enum ModuleManifest {
    /// Version `0` manifest schema.
    #[serde(rename = "0")]
    V0 {
        info: ModuleInfo,
        loader: String,
        load_deps: Vec<ModuleInterfaceInfo>,
    },
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub version: semver::Version,
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInterfaceInfo {
    pub name: String,
    pub version: fimo_version_core::Version,
    pub extensions: Vec<String>,
}

pub type LoaderCallback = dyn FnOnce(Arc<dyn ModuleLoader>) + Sync + Send;
pub type InterfaceCallback = dyn FnOnce(Arc<dyn ModuleInterface>) + Sync + Send;

#[derive(Debug)]
pub enum ModuleRegistryError {
    UnknownLoaderType(String),
    DuplicateLoaderType(String),
    UnknownInterface(ModuleInterfaceDescriptor),
    DuplicateInterface(ModuleInterfaceDescriptor),
    SerdeError(serde_json::Error),
    IOError(std::io::Error),
}

impl std::fmt::Display for ModuleRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for ModuleRegistryError {}

#[repr(transparent)]
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct CallbackHandle<T: ?Sized>(*const T);

impl<T: ?Sized> CallbackHandle<T> {
    fn new(id: *const T) -> Self {
        Self { 0: id }
    }

    fn as_ptr(&self) -> *const T {
        self.0
    }
}

pub struct ModuleRegistry {
    loaders: HashMap<String, Arc<dyn ModuleLoader>>,
    interfaces: BTreeMap<ModuleInterfaceDescriptor, Arc<dyn ModuleInterface>>,
    loader_callbacks: HashMap<*const dyn ModuleLoader, Vec<Box<LoaderCallback>>>,
    interface_callbacks: HashMap<*const dyn ModuleInterface, Vec<Box<InterfaceCallback>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            interfaces: BTreeMap::new(),
            loader_callbacks: HashMap::new(),
            interface_callbacks: HashMap::new(),
        }
    }

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

    pub fn get_loader_from_type(
        &self,
        loader_type: impl AsRef<str>,
    ) -> Result<Arc<dyn ModuleLoader + 'static>, ModuleRegistryError> {
        self.get_loader_ref_from_type(loader_type)
            .map(|loader| loader.clone())
    }

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

    pub fn get_interface_from_descriptor(
        &self,
        descriptor: impl AsRef<ModuleInterfaceDescriptor>,
    ) -> Result<Arc<dyn ModuleInterface + 'static>, ModuleRegistryError> {
        self.get_interface_ref_from_descriptor(descriptor)
            .map(|interface| interface.clone())
    }

    pub fn get_module_manifest(
        &self,
        module_path: impl AsRef<Path>,
    ) -> Result<ModuleManifest, ModuleRegistryError> {
        let manifest_path = module_path.as_ref().join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path)?;
        serde_json::from_reader(BufReader::new(file)).map_err(From::from)
    }
}

impl From<std::io::Error> for ModuleRegistryError {
    fn from(err: std::io::Error) -> Self {
        ModuleRegistryError::IOError(err)
    }
}

impl From<serde_json::Error> for ModuleRegistryError {
    fn from(err: serde_json::Error) -> Self {
        ModuleRegistryError::SerdeError(err)
    }
}

/// Implementation of the module api.
#[derive(Debug)]
pub struct ModuleAPI<'i> {
    phantom: PhantomData<fn() -> &'i ()>,
}

impl Default for ModuleAPI<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'i> ModuleAPI<'i> {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
