//! Definition of the Rust `fimo-core` interface.
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader, ModulePtr,
};
use fimo_version_core::Version;
use std::any::Any;
use std::error::Error;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Version the library was linked with.
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Implements the required trait for a fimo module.
#[macro_export]
macro_rules! impl_fimo_module_instance {
    ($instance: ty) => {
        impl $crate::rust::FimoModuleInstanceExtAPIStable for $instance {
            fn pkg_version(&self) -> &str {
                $crate::rust::PKG_VERSION
            }

            fn as_fimo_module_instance(
                &self,
            ) -> &(dyn $crate::rust::FimoModuleInstanceExt + 'static) {
                self
            }

            fn as_fimo_module_instance_mut(
                &mut self,
            ) -> &mut (dyn $crate::rust::FimoModuleInstanceExt + 'static) {
                self
            }
        }
    };
}

/// Casts an expression to a [fimo_module_core::ModulePtr].
#[macro_export]
macro_rules! to_fimo_module_instance_raw_ptr {
    ($self: expr) => {
        let ext_stable = $self as &dyn $crate::rust::FimoModuleInstanceExtAPIStable;
        unsafe { fimo_module_core::ModulePtr::Fat(std::mem::transmute(ext_stable)) }
    };
}

/// A wrapped interface.
#[repr(transparent)]
pub struct InterfaceMutex<T: ?Sized> {
    guard: dyn InterfaceGuardInternal<T>,
}

/// A RAII lock from a `InterfaceMutex`.
pub struct InterfaceGuard<'a, T: ?Sized> {
    interface: *mut T,
    guard: &'a dyn InterfaceGuardInternal<T>,
}

/// An error from the `try_lock` function.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum TryLockError {
    /// Operation would result in blocking.
    WouldBlock,
}

/// Handle to a registered callback.
#[repr(transparent)]
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct CallbackHandle<T: ?Sized>(*const T);

/// A trait implementing the functionality of a mutex.
pub trait InterfaceGuardInternal<T: ?Sized>: Send + Sync {
    /// Locks the interface and extracts a pointer to itself.
    fn lock(&self) -> *mut T;

    /// Attempts to acquire the mutex without blocking.
    fn try_lock(&self) -> Result<*mut T, TryLockError>;

    /// Unlocks the interface.
    ///
    /// # Safety
    ///
    /// May only be called if the current context holds the mutex.
    unsafe fn unlock(&self);

    /// Extracts a pointer to the interface without locking.
    ///
    /// # Safety
    ///
    /// May only be called if the current context holds the mutex.
    unsafe fn data_ptr(&self) -> *mut T;
}

/// Trait describing the `fimo-core` interface.
pub trait FimoCore {
    /// Extracts the interface version.
    fn get_interface_version(&self) -> Version;

    /// Extracts a reference to an extension from the interface.
    fn find_extension(&self, extension: &str) -> Option<&(dyn Any + 'static)>;

    /// Extracts a mutable reference to an extension from the interface.
    fn find_extension_mut(&mut self, extension: &str) -> Option<&mut (dyn Any + 'static)>;

    /// Extracts a reference to the module registry.
    fn as_module_registry(&self) -> &(dyn ModuleRegistry + 'static);

    /// Extracts a mutable reference to the module registry.
    fn as_module_registry_mut(&mut self) -> &mut (dyn ModuleRegistry + 'static);

    /// Casts the interface to a `&(dyn Any + 'static)`.
    fn as_any(&self) -> &(dyn Any + 'static);

    /// Casts the interface to a `&mut (dyn Any + 'static)`.
    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static);
}

/// Trait describing a `ModuleRegistry`.
pub trait ModuleRegistry {
    /// Registers a new module loader to the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    fn register_loader(
        &mut self,
        loader_type: &str,
        loader: Arc<dyn ModuleLoader + 'static>,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before removing.
    fn unregister_loader(
        &mut self,
        loader_type: &str,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Registers a loader-removal callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the loader is removed.
    fn register_loader_callback(
        &mut self,
        loader_type: &str,
        callback: Box<LoaderCallback>,
        callback_handle: &mut MaybeUninit<CallbackHandle<LoaderCallback>>,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Unregisters a loader-removal callback from the `ModuleRegistry`.
    ///
    /// The callback will not be called.
    fn unregister_loader_callback(
        &mut self,
        loader_type: &str,
        callback_handle: CallbackHandle<LoaderCallback>,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Extracts a loader from the `ModuleRegistry` using the registration type.
    fn get_loader_from_type(
        &self,
        loader_type: &str,
    ) -> Result<Arc<dyn ModuleLoader + 'static>, Box<dyn Error>>;

    /// Registers a new interface to the `ModuleRegistry`.
    fn register_interface(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: Arc<dyn ModuleInterface + 'static>,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Unregisters an existing interface from the `ModuleRegistry`.
    ///
    /// This function calls the interface-remove callbacks that are registered
    /// with the interface before removing it.
    fn unregister_interface(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Registers an interface-removed callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
    fn register_interface_callback(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: Box<InterfaceCallback>,
        callback_handle: &mut MaybeUninit<CallbackHandle<InterfaceCallback>>,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
    fn unregister_interface_callback(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback_handle: CallbackHandle<InterfaceCallback>,
    ) -> Result<&mut (dyn ModuleRegistry + 'static), Box<dyn Error>>;

    /// Extracts an interface from the `ModuleRegistry`.
    fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface + 'static>, Box<dyn Error>>;

    /// Extracts all interface descriptors with the same name.
    fn get_interface_descriptors_from_name(
        &self,
        interface_name: &str,
    ) -> Vec<ModuleInterfaceDescriptor>;

    /// Extracts all descriptors of compatible interfaces.
    fn get_compatible_interface_descriptors(
        &self,
        interface_name: &str,
        interface_version: &Version,
        interface_extensions: &[ArrayString<32>],
    ) -> Vec<ModuleInterfaceDescriptor>;

    /// Casts the `ModuleRegistry` to a `&(dyn Any + 'static)`.
    fn as_any(&self) -> &(dyn Any + 'static);

    /// Casts the `ModuleRegistry` to a `&mut (dyn Any + 'static)`.
    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static);
}

/// API stable trait for identifying a fimo module.
///
/// Changing this trait is a breaking change because it is used to identify
/// version mismatches. Implementors must provide a `&dyn FimoModuleInstanceExtAPIStable`
/// with the [ModuleInstance::get_raw_ptr] function.
pub trait FimoModuleInstanceExtAPIStable: ModuleInstance {
    /// Extracts the linked package version of this crate.
    ///
    /// Must always be [PKG_VERSION].
    fn pkg_version(&self) -> &str;

    /// Casts the `&dyn FimoModuleInstanceExtAPIStable` to a
    /// `&(dyn FimoModuleInstanceExt + 'static)`.
    fn as_fimo_module_instance(&self) -> &(dyn FimoModuleInstanceExt + 'static);

    /// Casts the `&mut dyn FimoModuleInstanceExtAPIStable` to a
    /// `&mut (dyn FimoModuleInstanceExt + 'static)`.
    fn as_fimo_module_instance_mut(&mut self) -> &mut (dyn FimoModuleInstanceExt + 'static);
}

/// A trait describing a fimo module.
pub trait FimoModuleInstanceExt: FimoModuleInstanceExtAPIStable {
    /// Provides the `fimo-core` interface to the instance.
    fn set_core_interface(&mut self, fimo_core: Arc<InterfaceMutex<dyn FimoCore>>);
}

/// Type of a loader callback.
pub type LoaderCallback = dyn FnOnce(Arc<dyn ModuleLoader>) + Sync + Send;

/// Type of an interface callback.
pub type InterfaceCallback = dyn FnOnce(Arc<dyn ModuleInterface>) + Sync + Send;

impl<T: ?Sized> InterfaceMutex<T> {
    /// Constructs a new `InterfaceMutex<T>`.
    pub fn new(guard: &dyn InterfaceGuardInternal<T>) -> &Self {
        unsafe { std::mem::transmute(guard) }
    }

    /// Acquires this mutex.
    pub fn lock(&self) -> InterfaceGuard<'_, T> {
        InterfaceGuard {
            interface: self.guard.lock(),
            guard: &self.guard,
        }
    }

    /// Attempts to acquire this mutex without blocking.
    pub fn try_lock(&self) -> Result<InterfaceGuard<'_, T>, TryLockError> {
        self.guard.try_lock().map(|interface| InterfaceGuard {
            interface,
            guard: &self.guard,
        })
    }

    /// Extracts a pointer to the interface without locking.
    ///
    /// # Safety
    ///
    /// May only be called if the current context holds the mutex.
    pub unsafe fn data_ptr(&self) -> *mut T {
        self.guard.data_ptr()
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for InterfaceMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("InterfaceMutex");
        match self.try_lock() {
            Ok(guard) => {
                d.field("data", &&*guard);
            }
            Err(_) => {
                struct LockedPlaceholder;
                impl std::fmt::Debug for LockedPlaceholder {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("<locked>")
                    }
                }
                d.field("data", &LockedPlaceholder);
            }
        }
        d.finish_non_exhaustive()
    }
}

unsafe impl<T: ?Sized + Sync> Sync for InterfaceGuard<'_, T> {}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for InterfaceGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + std::fmt::Display> std::fmt::Display for InterfaceGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized> Drop for InterfaceGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { self.guard.unlock() }
    }
}

impl<T: ?Sized> Deref for InterfaceGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.interface }
    }
}

impl<T: ?Sized> DerefMut for InterfaceGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.interface }
    }
}

impl AsRef<dyn ModuleRegistry> for dyn FimoCore {
    fn as_ref(&self) -> &(dyn ModuleRegistry + 'static) {
        self.as_module_registry()
    }
}

impl AsMut<dyn ModuleRegistry> for dyn FimoCore {
    fn as_mut(&mut self) -> &mut (dyn ModuleRegistry + 'static) {
        self.as_module_registry_mut()
    }
}

/// Casts an generic interface to a `fimo-core` interface.
///
/// # Safety
///
/// This function is highly unsafe as the compiler can not check the
/// validity of the cast. It assumes that a `&dyn ModuleInterface` has
/// the same size of an `&InterfaceMutex<dyn FimoCore>` and
/// `(*const u8, *const u8)`, and shares the same alignment as an
/// `&InterfaceMutex<dyn FimoCore>`. `interface.get_raw_ptr()` must return
/// a `&dyn InterfaceGuardInternal<dyn FimoCore>` as a [ModulePtr::Fat].
pub unsafe fn cast_interface(
    interface: Arc<dyn ModuleInterface>,
) -> Option<Arc<InterfaceMutex<dyn FimoCore>>> {
    sa::assert_eq_size!(
        &dyn ModuleInterface,
        &InterfaceMutex<dyn FimoCore>,
        &dyn InterfaceGuardInternal<dyn FimoCore>,
        (*const u8, *const u8)
    );
    sa::assert_eq_align!(
        &dyn ModuleInterface,
        &InterfaceMutex<dyn FimoCore>,
        &dyn InterfaceGuardInternal<dyn FimoCore>
    );

    let interface_ptr = Arc::into_raw(interface);
    let interface_ref = &*interface_ptr;

    match interface_ref.get_raw_ptr() {
        ModulePtr::Fat(ptr) => {
            let guard: &dyn InterfaceGuardInternal<dyn FimoCore> = std::mem::transmute(ptr);
            let mutex_ptr = InterfaceMutex::new(guard);
            Some(Arc::from_raw(mutex_ptr as *const _))
        }
        _ => {
            drop(Arc::from_raw(interface_ptr));
            None
        }
    }
}

/// Casts a generic module instance to a fimo module instance.
///
/// # Safety
///
/// This function is highly unsafe as the compiler can not check the
/// validity of the cast. It assumes that a `&dyn ModuleInstance` has
/// the same size of a `&dyn FimoModuleInstanceExt`, `&dyn FimoModuleInstanceExtAPIStable`
/// and `(*const u8, *const u8)`, and shares the same alignment as a
/// `&dyn FimoModuleInstanceExt`. `instance.get_raw_ptr()` must return
/// a `&dyn FimoModuleInstanceExtAPIStable` as a [ModulePtr::Fat].
pub unsafe fn cast_instance(
    instance: Arc<dyn ModuleInstance>,
) -> Result<Arc<dyn FimoModuleInstanceExt>, std::io::Error> {
    sa::assert_eq_size!(
        &dyn ModuleInstance,
        &dyn FimoModuleInstanceExt,
        &dyn FimoModuleInstanceExtAPIStable,
        (*const u8, *const u8)
    );
    sa::assert_eq_align!(&dyn ModuleInstance, &dyn FimoModuleInstanceExt,);

    let instance_ptr = Arc::into_raw(instance);
    let instance_ref = &*instance_ptr;

    match instance_ref.get_raw_ptr() {
        ModulePtr::Fat(ptr) => {
            let ext_stable: &dyn FimoModuleInstanceExtAPIStable = std::mem::transmute(ptr);

            if PKG_VERSION != ext_stable.pkg_version() {
                drop(Arc::from_raw(instance_ptr));
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Version mismatch",
                ))
            } else {
                let ext = ext_stable.as_fimo_module_instance();
                Ok(Arc::from_raw(ext as *const _))
            }
        }
        _ => {
            drop(Arc::from_raw(instance_ptr));
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Pointer layout mismatch",
            ))
        }
    }
}
