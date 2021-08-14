//! Implementation of the module.
use crate::{module_registry::ModuleRegistry, settings_registry::SettingsRegistry, CoreInterface};
use fimo_core_interface::rust::{
    CallbackHandle, InterfaceCallback, InterfaceGuardInternal, LoaderCallback, SettingsItem,
    SettingsUpdateCallback, TryLockError,
};
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    ModuleInfo, ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader, ModulePtr,
};
use fimo_version_core::Version;
use parking_lot::Mutex;
use std::any::Any;
use std::error::Error;
use std::mem::MaybeUninit;
use std::sync::Arc;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

struct MutexWrapper<T> {
    data: Mutex<T>,
    parent: Arc<dyn ModuleInstance>,
}

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

    fn as_settings_registry(&self) -> &(dyn fimo_core_interface::rust::SettingsRegistry + 'static) {
        self.as_settings_registry()
    }

    fn as_settings_registry_mut(
        &mut self,
    ) -> &mut (dyn fimo_core_interface::rust::SettingsRegistry + 'static) {
        self.as_settings_registry_mut()
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

impl fimo_core_interface::rust::SettingsRegistry for SettingsRegistry {
    fn is_null(&self, item: &str) -> Option<bool> {
        self.is_null(item)
    }

    fn is_bool(&self, item: &str) -> Option<bool> {
        self.is_bool(item)
    }

    fn is_u64(&self, item: &str) -> Option<bool> {
        self.is_u64(item)
    }

    fn is_f64(&self, item: &str) -> Option<bool> {
        self.is_f64(item)
    }

    fn is_string(&self, item: &str) -> Option<bool> {
        self.is_string(item)
    }

    fn is_number(&self, item: &str) -> Option<bool> {
        self.is_number(item)
    }

    fn is_array(&self, item: &str) -> Option<bool> {
        self.is_array(item)
    }

    fn is_object(&self, item: &str) -> Option<bool> {
        self.is_object(item)
    }

    fn array_len(&self, item: &str) -> Option<usize> {
        self.array_len(item)
    }

    fn read_all(&self) -> SettingsItem {
        self.read_all()
    }

    fn read(&self, item: &str) -> Option<SettingsItem> {
        self.read(item)
    }

    fn write(&mut self, item: &str, value: SettingsItem) -> Option<SettingsItem> {
        self.write(item, value)
    }

    fn remove(&mut self, item: &str) -> Option<SettingsItem> {
        self.remove(item)
    }

    fn register_callback(
        &mut self,
        item: &str,
        callback: Box<SettingsUpdateCallback>,
    ) -> Option<CallbackHandle<SettingsUpdateCallback>> {
        self.register_callback(item, callback)
    }

    fn unregister_callback(&mut self, item: &str, handle: CallbackHandle<SettingsUpdateCallback>) {
        self.unregister_callback(item, handle)
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(String::from(&crate::INTERFACE_VERSION).as_bytes())
        },
    }
}

#[allow(dead_code)]
fn get_core_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked(crate::INTERFACE_NAME.as_bytes()) },
        version: crate::INTERFACE_VERSION,
        extensions: Default::default(),
    }
}
