//! Implementation of basic fimo module loaders.
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use std::any::Any;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

pub mod ffi_loader;
pub mod rust_loader;

/// Module information.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ModuleInfo {
    /// Module name.
    pub name: fimo_ffi_core::ArrayString<32>,
    /// Module version.
    pub version: fimo_ffi_core::ArrayString<32>,
}

/// A descriptor for a module interface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ModuleInterfaceDescriptor {
    /// Name of the interface.
    pub name: fimo_ffi_core::ArrayString<32>,
    /// Version of the interface.
    pub version: fimo_version_core::Version,
    /// Available interface extensions.
    pub extensions: fimo_ffi_core::ConstSpan<fimo_ffi_core::ArrayString<32>>,
}

/// A wrapper around a `ModuleInterfaceDescriptor` with custom a
/// `PartialEq` implementation, which check for compatability
/// instead of equality.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialOrd)]
pub struct ModuleInterfaceDescriptorCompatability(pub ModuleInterfaceDescriptor);

/// A raw pointer to module internals.
#[repr(C, i8)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum ModulePtr {
    /// A single pointer.
    Slim(*const u8),
    /// Two pointers.
    Fat((*const u8, *const u8)),
    /// Unspecified layout.
    Other([u8; 32]),
}

/// A module interface.
pub trait ModuleInterface: Send + Sync {
    /// Fetches an internal [ModulePtr] to the interface.
    ///
    /// The ptr remains valid until the interface is dropped.
    fn get_raw_ptr(&self) -> ModulePtr;

    /// Fetches the parent instance.
    fn get_instance(&self) -> Arc<dyn ModuleInstance>;

    /// Casts the interface to a `&dyn Any`.
    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static);
}

/// A module instance.
pub trait ModuleInstance: Send + Sync {
    /// Fetches the parent module.
    ///
    /// # Note
    ///
    /// The parent module should be stored as an `Arc<impl Module>`
    /// internally, to ensure, that the module outlives the instance.
    fn get_module(&self) -> Arc<dyn Module>;

    /// Fetches a slice of available interfaces.
    ///
    /// The resulting descriptors can be used to instantiate the interfaces.
    fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor];

    /// Fetches the interface described by the interface descriptor.
    ///
    /// The interface is instantiated if it does not already exist.
    /// Multiple calls with the same interface will retrieve the same
    /// instance if is has not already been dropped.
    ///
    /// # Note
    ///
    /// Implementations should only store `Weak<impl ModuleInterface>` internally and
    /// try to upgrade them with [fimo_ffi_core::Weak::upgrade].
    fn get_interface(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>;

    /// Fetches the dependencies of an interface.
    fn get_interface_dependencies(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<&[ModuleInterfaceDescriptor], Box<dyn Error>>;

    /// Casts the instance to a `&dyn Any`.
    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static);
}

/// A module.
pub trait Module: Send + Sync {
    /// Fetches the path to the module root.
    fn get_module_path(&self) -> &Path;

    /// Fetches a reference to the modules `ModuleInfo`.
    fn get_module_info(&self) -> &ModuleInfo;

    /// Fetches a copy of the loader which loaded the module.
    ///
    /// # Note
    ///
    /// The loader should be stored as an `Arc<impl ModuleLoader>`
    /// internally, to ensure, that the module loader outlives the module.
    fn get_module_loader(&self) -> Arc<dyn ModuleLoader>;

    /// Instantiates the module.
    ///
    /// A module may disallow multiple instantiations.
    ///
    /// # Note
    ///
    /// This function must result in an unique instance, or an error, each time it is called.
    /// The resulting instance should not be stored internally as an `Arc<impl ModuleInstance>`,
    /// to prevent cyclic references.
    fn create_instance(&self) -> Result<Arc<dyn ModuleInstance>, Box<dyn Error>>;

    /// Casts the module to a `&dyn Any`.
    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static);
}

/// A module loader.
pub trait ModuleLoader: Send + Sync {
    /// Loads a new module from a path.
    ///
    /// # Note
    ///
    /// The resulting module should not be stored internally as an `Arc<impl Module>`, to
    /// prevent cyclic references.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    unsafe fn load_module(&self, path: &Path) -> Result<Arc<dyn Module>, Box<dyn Error>>;

    /// Casts the module loader to a `&dyn Any`.
    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static);
}

impl std::fmt::Display for ModuleInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

impl std::fmt::Display for ModuleInterfaceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

impl std::fmt::Display for ModuleInterfaceDescriptorCompatability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl PartialEq for ModuleInterfaceDescriptorCompatability {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.0.name, &other.0.name)
            && self.0.version.is_compatible(&other.0.version)
            && self
                .0
                .extensions
                .iter()
                .all(|e| other.0.extensions.contains(e))
    }
}

impl PartialEq<ModuleInterfaceDescriptor> for ModuleInterfaceDescriptorCompatability {
    fn eq(&self, other: &ModuleInterfaceDescriptor) -> bool {
        PartialEq::eq(&self.0.name, &other.name)
            && self.0.version.is_compatible(&other.version)
            && self
                .0
                .extensions
                .iter()
                .all(|e| other.extensions.contains(e))
    }
}

impl PartialEq<ModuleInterfaceDescriptorCompatability> for ModuleInterfaceDescriptor {
    fn eq(&self, other: &ModuleInterfaceDescriptorCompatability) -> bool {
        PartialEq::eq(&self.name, &other.0.name)
            && self.version.is_compatible(&other.0.version)
            && self
                .extensions
                .iter()
                .all(|e| other.0.extensions.contains(e))
    }
}
