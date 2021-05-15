use crate::base_interface::BaseInterfaceWrapper;
use emf_core_base_rs::ffi;
use emf_core_base_rs::ffi::collections::ConstSpan;
use emf_core_base_rs::ffi::module::native_module::NativeModule;
use emf_core_base_rs::ffi::module::{
    Interface, InterfaceDescriptor, InterfaceName, ModuleInfo, ModuleName, ModuleStatus,
    ModuleVersion,
};
use emf_core_base_rs::ffi::CBASE_INTERFACE_NAME;
use fimo_version_rs::is_compatible;
use std::ptr::NonNull;

/// Base module.
#[derive(Debug)]
pub struct FimoBase {
    module_info: ModuleInfo,
    module_status: ModuleStatus,
    interfaces: Vec<InterfaceDescriptor>,
    interface_ptr: Vec<Interface>,
}

impl Default for FimoBase {
    #[inline]
    fn default() -> Self {
        FimoBase::new()
    }
}

impl Drop for FimoBase {
    fn drop(&mut self) {}
}

impl FimoBase {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            module_info: ModuleInfo {
                name: ModuleName::from("fimo_base"),
                version: ModuleVersion::from("0.1.0-unstable"),
            },
            module_status: ModuleStatus::Terminated,
            interfaces: Vec::new(),
            interface_ptr: Vec::new(),
        }
    }

    /// Interprets the pointer as an instance.
    #[inline]
    pub fn from_module(module: Option<NonNull<NativeModule>>) -> Option<&'static Self> {
        match module {
            None => None,
            Some(module) => unsafe { module.cast::<Self>().as_ptr().as_ref() },
        }
    }

    /// Interprets the pointer as a mutable instance.
    #[inline]
    pub fn from_module_mut(module: Option<NonNull<NativeModule>>) -> Option<&'static mut Self> {
        match module {
            None => None,
            Some(module) => unsafe { module.cast::<Self>().as_ptr().as_mut() },
        }
    }

    /// Initializes the instance.
    #[inline]
    pub fn initialize(&mut self) -> ffi::collections::Result<i8, ffi::module::Error> {
        match self.module_status {
            ModuleStatus::Terminated => (),
            _ => return ffi::collections::Result::new_err(ffi::module::Error::ModuleStateInvalid),
        };

        let base_interface = Box::new(BaseInterfaceWrapper::new());

        let base_descriptor = InterfaceDescriptor {
            name: InterfaceName::from(CBASE_INTERFACE_NAME),
            version: base_interface.version(),
            extensions: ConstSpan::from(base_interface.extensions()),
        };

        let base_interface = NonNull::from(Box::leak(base_interface)).cast();

        self.interfaces.push(base_descriptor);
        self.interface_ptr.push(Interface {
            interface: base_interface,
        });

        self.module_status = ModuleStatus::Ready;
        ffi::collections::Result::new_ok(0)
    }

    /// Terminates the instance.
    #[inline]
    pub fn terminate(&mut self) -> ffi::collections::Result<i8, ffi::module::Error> {
        match self.module_status {
            ModuleStatus::Ready => (),
            _ => return ffi::collections::Result::new_err(ffi::module::Error::ModuleStateInvalid),
        };

        // Safety: We know the type and validity of the pointer.
        drop(unsafe {
            Box::<BaseInterfaceWrapper>::from_raw(self.interface_ptr[0].interface.cast().as_ptr())
        });

        self.interfaces.clear();
        self.interface_ptr.clear();

        self.module_status = ModuleStatus::Terminated;
        ffi::collections::Result::new_ok(0)
    }

    /// Fetches an interface from the module.
    #[inline]
    pub fn get_interface(
        &self,
        interface: &InterfaceDescriptor,
    ) -> ffi::collections::Result<Interface, ffi::module::Error> {
        if let Some(i) = self.interfaces.iter().position(|&v| {
            interface.name == v.name
                && is_compatible(&interface.version, &v.version)
                && interface
                    .extensions
                    .iter()
                    .all(|ex| v.extensions.contains(ex))
        }) {
            ffi::collections::Result::new_ok(self.interface_ptr[i])
        } else {
            ffi::collections::Result::new_err(ffi::module::Error::InterfaceNotFound)
        }
    }

    /// Fetches the module info of the module.
    #[inline]
    pub fn get_module_info(&self) -> &ModuleInfo {
        &self.module_info
    }

    /// Fetches the load dependencies of the module.
    #[inline]
    pub fn get_load_dependencies() -> ConstSpan<InterfaceDescriptor> {
        ConstSpan::new()
    }

    /// Fetches the runtime dependencies of the module.
    #[inline]
    pub fn get_runtime_dependencies(&self) -> ConstSpan<InterfaceDescriptor> {
        ConstSpan::new()
    }

    /// Fetches the exportable interfaces of the module.
    #[inline]
    pub fn get_exportable_interfaces(&self) -> ConstSpan<InterfaceDescriptor> {
        ConstSpan::from(&self.interfaces)
    }

    /// Returns the module status.
    #[inline]
    pub fn module_status(&self) -> ModuleStatus {
        self.module_status
    }
}
