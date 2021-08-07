//! Implementation of the module.
use crate::{core_interface::module_registry::ModuleRegistry, CoreInterface};
use fimo_core_interface::rust::{
    CallbackHandle, InterfaceCallback, InterfaceGuardInternal, InterfaceMutex, LoaderCallback,
    TryLockError,
};
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    Module, ModuleInfo, ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader,
    ModulePtr,
};
use fimo_version_core::Version;
use parking_lot::Mutex;
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::mem::MaybeUninit;
use std::sync::{Arc, Weak};

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

/// Core module.
pub struct FimoCore {
    available_interfaces: Vec<ModuleInterfaceDescriptor>,
    interfaces: Mutex<HashMap<ModuleInterfaceDescriptor, Interface>>,
    interface_dependencies: HashMap<ModuleInterfaceDescriptor, Vec<ModuleInterfaceDescriptor>>,
    // Drop the parent for last
    parent: Arc<dyn Module>,
}

/// Error resulting from an unknown interface.
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct UnknownInterfaceError {
    interface: ModuleInterfaceDescriptor,
}

/// Error from the [FimoCore::get_interface] function.
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

struct MutexWrapper<T> {
    data: Mutex<T>,
    parent: Arc<dyn ModuleInstance>,
}

type InterfaceBuilder =
    fn(Arc<dyn ModuleInstance>) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>;

impl FimoCore {
    /// Constructs a new `FimoCore` instance.
    pub fn new(parent: Arc<dyn Module>) -> Arc<Self> {
        let core_info = ModuleInterfaceDescriptor {
            name: unsafe {
                ArrayString::from_utf8_unchecked(crate::core_interface::INTERFACE_NAME.as_bytes())
            },
            version: crate::core_interface::INTERFACE_VERSION,
            extensions: Default::default(),
        };

        let mut interfaces = HashMap::new();
        let mut interface_dependencies = HashMap::new();

        interfaces.insert(
            core_info,
            Interface {
                ptr: None,
                builder: |instance| {
                    Ok(Arc::new(MutexWrapper {
                        data: Mutex::new(CoreInterface::new()),
                        parent: instance,
                    }))
                },
            },
        );

        interface_dependencies.insert(core_info, vec![]);

        Arc::new(Self {
            parent,
            available_interfaces: vec![core_info],
            interfaces: Mutex::new(interfaces),
            interface_dependencies,
        })
    }

    /// Extracts the available interfaces.
    pub fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        self.available_interfaces.as_slice()
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
        if let Some(int) = guard.get_mut(interface) {
            if let Some(ptr) = &int.ptr {
                if let Some(arc) = ptr.upgrade() {
                    return Ok(arc);
                }
            }

            // SAFETY: A `FimoCore` is always in an `Arc`.
            let self_arc = unsafe {
                Arc::increment_strong_count(self as *const Self);
                Arc::from_raw(self as *const Self)
            };

            (int.builder)(self_arc).map_or_else(
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
}

impl ModuleInstance for FimoCore {
    fn get_raw_ptr(&self) -> ModulePtr {
        fimo_core_interface::to_fimo_module_instance_raw_ptr!(self)
    }

    fn get_module(&self) -> Arc<dyn Module> {
        self.parent.clone()
    }

    fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        self.available_interfaces.as_slice()
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

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

fimo_core_interface::impl_fimo_module_instance! {FimoCore}

impl fimo_core_interface::rust::FimoModuleInstanceExt for FimoCore {
    fn set_core_interface(
        &mut self,
        _: Arc<InterfaceMutex<dyn fimo_core_interface::rust::FimoCore>>,
    ) {
    }
}

impl Debug for FimoCore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FimoCore)")
    }
}

impl Display for UnknownInterfaceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown interface: {}", self.interface)
    }
}

impl Error for UnknownInterfaceError {}

impl Display for GetInterfaceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GetInterfaceError::UnknownInterface(err) => Display::fmt(err, f),
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

impl InterfaceGuardInternal<dyn fimo_core_interface::rust::FimoCore>
    for MutexWrapper<CoreInterface>
{
    fn lock(&self) -> *mut dyn fimo_core_interface::rust::FimoCore {
        std::mem::forget(self.data.lock());
        unsafe { self.data_ptr() }
    }

    fn try_lock(&self) -> Result<*mut dyn fimo_core_interface::rust::FimoCore, TryLockError> {
        if let Some(guard) = self.data.try_lock() {
            std::mem::forget(guard);
            unsafe { Ok(self.data_ptr()) }
        } else {
            Err(TryLockError::WouldBlock)
        }
    }

    unsafe fn unlock(&self) {
        self.data.force_unlock();
    }

    unsafe fn data_ptr(&self) -> *mut dyn fimo_core_interface::rust::FimoCore {
        self.data.data_ptr()
    }
}

impl ModuleInterface for MutexWrapper<CoreInterface> {
    fn get_raw_ptr(&self) -> ModulePtr {
        let guard = self as &dyn InterfaceGuardInternal<dyn fimo_core_interface::rust::FimoCore>;
        unsafe { ModulePtr::Fat(std::mem::transmute(guard)) }
    }

    fn get_instance(&self) -> Arc<dyn ModuleInstance> {
        self.parent.clone()
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl fimo_core_interface::rust::FimoCore for CoreInterface {
    fn get_interface_version(&self) -> Version {
        self.get_interface_version()
    }

    fn find_extension(&self, extension: &str) -> Option<&(dyn Any + 'static)> {
        self.find_extension(extension)
    }

    fn find_extension_mut(&mut self, extension: &str) -> Option<&mut (dyn Any + 'static)> {
        self.find_extension_mut(extension)
    }

    fn as_module_registry(&self) -> &(dyn fimo_core_interface::rust::ModuleRegistry + 'static) {
        self.as_module_registry()
    }

    fn as_module_registry_mut(
        &mut self,
    ) -> &mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static) {
        self.as_module_registry_mut()
    }

    fn as_any(&self) -> &(dyn Any + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static) {
        self
    }
}

impl fimo_core_interface::rust::ModuleRegistry for ModuleRegistry {
    fn register_loader(
        &mut self,
        loader_type: &str,
        loader: &'static (dyn ModuleLoader + 'static),
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        self.register_loader(loader_type, loader)
            .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
    }

    fn unregister_loader(
        &mut self,
        loader_type: &str,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        self.unregister_loader(loader_type)
            .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
    }

    fn register_loader_callback(
        &mut self,
        loader_type: &str,
        callback: Box<LoaderCallback>,
        callback_handle: &mut MaybeUninit<CallbackHandle<LoaderCallback>>,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        unsafe {
            self.register_loader_callback(
                loader_type,
                callback,
                std::mem::transmute(callback_handle),
            )
            .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
        }
    }

    fn unregister_loader_callback(
        &mut self,
        loader_type: &str,
        callback_handle: CallbackHandle<LoaderCallback>,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        unsafe {
            self.unregister_loader_callback(loader_type, std::mem::transmute(callback_handle))
                .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
        }
    }

    fn get_loader_from_type(
        &self,
        loader_type: &str,
    ) -> Result<&'static (dyn ModuleLoader + 'static), Box<dyn Error>> {
        self.get_loader_from_type(loader_type)
            .map_err(|e| Box::new(e) as _)
    }

    fn register_interface(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: Arc<dyn ModuleInterface + 'static>,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        self.register_interface(descriptor, interface)
            .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
    }

    fn unregister_interface(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        self.unregister_interface(descriptor)
            .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
    }

    fn register_interface_callback(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: Box<InterfaceCallback>,
        callback_handle: &mut MaybeUninit<CallbackHandle<InterfaceCallback>>,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        unsafe {
            self.register_interface_callback(
                descriptor,
                callback,
                std::mem::transmute(callback_handle),
            )
            .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
        }
    }

    fn unregister_interface_callback(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback_handle: CallbackHandle<InterfaceCallback>,
    ) -> Result<&mut (dyn fimo_core_interface::rust::ModuleRegistry + 'static), Box<dyn Error>>
    {
        unsafe {
            self.unregister_interface_callback(descriptor, std::mem::transmute(callback_handle))
                .map_or_else(|e| Err(Box::new(e) as _), |r| Ok(r as _))
        }
    }

    fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface + 'static>, Box<dyn Error>> {
        self.get_interface_from_descriptor(descriptor)
            .map_err(|e| Box::new(e) as _)
    }

    fn get_interface_descriptors_from_name(
        &self,
        interface_name: &str,
    ) -> Vec<ModuleInterfaceDescriptor> {
        self.get_interface_descriptors_from_name(interface_name)
    }

    fn get_compatible_interface_descriptors(
        &self,
        interface_name: &str,
        interface_version: &Version,
        interface_extensions: &[ArrayString<32>],
    ) -> Vec<ModuleInterfaceDescriptor> {
        self.get_compatible_interface_descriptors(
            interface_name,
            interface_version,
            interface_extensions,
        )
    }

    fn as_any(&self) -> &(dyn Any + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static) {
        self
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(
                String::from(&crate::core_interface::INTERFACE_VERSION).as_bytes(),
            )
        },
    }
}
