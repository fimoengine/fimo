use crate::{ModuleInfo, ModuleInterfaceDescriptor, PathChar, Result};
use fimo_ffi::span::SpanInner;
use fimo_ffi::vtable::BaseInterface;
use fimo_ffi::{fimo_object, fimo_vtable, ObjArc, Object};
use fimo_version_core::Version;
use std::path::{Path, PathBuf};

/// Marker type that implements `Send` and `Sync`.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SendSyncMarker;

fimo_object! {
    /// A type-erased module loader.
    pub struct ModuleLoader<vtable = ModuleLoaderVTable>;
}

impl ModuleLoader {
    /// Casts the `ModuleLoader` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<BaseInterface> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.inner)(ptr);

            // safety: we know that both ptr and vtable are valid so we can dereference.
            &*fimo_ffi::object::from_raw_parts(ptr, vtable)
        }
    }

    /// Removes all modules that aren't referenced by anyone from the cache,
    /// unloading them in the process.
    #[inline]
    pub fn evict_module_cache(&self) {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.evict_module_cache)(ptr) }
    }

    /// Loads a new module from a path to the module root.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    #[inline]
    pub unsafe fn load_module<P: AsRef<Path>>(&'static self, path: P) -> Result<ObjArc<Module>> {
        let (ptr, vtable) = self.into_raw_parts();
        Self::load_inner(path, ptr, vtable.load_module)
    }

    /// Loads a new module from a path to the module library.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    #[inline]
    pub unsafe fn load_module_raw<P: AsRef<Path>>(
        &'static self,
        path: P,
    ) -> Result<ObjArc<Module>> {
        let (ptr, vtable) = self.into_raw_parts();
        Self::load_inner(path, ptr, vtable.load_module_raw)
    }

    #[cfg(unix)]
    #[inline]
    fn load_inner<P: AsRef<Path>>(
        path: P,
        ptr: *const (),
        f: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<Module>>,
    ) -> Result<ObjArc<Module>> {
        use std::os::unix::ffi::OsStrExt;

        let path = path.as_ref();
        let os_str = path.as_os_str();
        let bytes = OsStrExt::as_bytes(os_str);
        unsafe { f(ptr, bytes.into()) }
    }

    #[cfg(windows)]
    #[inline]
    fn load_inner<P: AsRef<Path>>(
        path: P,
        ptr: *const (),
        f: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<Module>>,
    ) -> Result<ObjArc<Module>> {
        use std::os::windows::ffi::OsStrExt;

        let path = path.as_ref();
        let os_str = path.as_os_str();
        let buf: Vec<PathChar> = OsStrExt::encode_wide(os_str).collect();
        let bytes = buf.as_slice();
        unsafe { f(ptr, bytes.into()) }
    }
}

fimo_vtable! {
    /// VTable of a module loader.
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct ModuleLoaderVTable<id = "fimo::module::interfaces::module_loader", marker = SendSyncMarker> {
        /// Fetches an internal vtable for the loader.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static BaseInterface,
        /// Removes all modules that aren't referenced by anyone from the cache,
        /// unloading them in the process.
        pub evict_module_cache: unsafe extern "C" fn(*const ()),
        /// Loads a new module from a path to the module root.
        ///
        /// # Safety
        ///
        /// - The module must be exposed in a way understood by the module loader.
        /// - The module ABI must match the loader ABI.
        ///
        /// Violating these invariants may lead to undefined behaviour.
        pub load_module: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<Module>>,
        /// Loads a new module from a path to the module library.
        ///
        /// # Safety
        ///
        /// - The module must be exposed in a way understood by the module loader.
        /// - The module ABI must match the loader ABI.
        ///
        /// Violating these invariants may lead to undefined behaviour.
        pub load_module_raw: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<Module>>,
    }
}

fimo_object! {
    /// A type-erased module.
    pub struct Module<vtable = ModuleVTable>;
}

impl Module {
    /// Casts the `Module` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<BaseInterface> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.inner)(ptr);

            // safety: we know that both ptr and vtable are valid so we can dereference.
            &*fimo_ffi::object::from_raw_parts(ptr, vtable)
        }
    }

    /// Fetches the path to the module root.
    #[inline]
    #[cfg(unix)]
    pub fn module_path(&self) -> PathBuf {
        use std::os::unix::ffi::OsStrExt;
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let path = (vtable.module_path)(ptr);
            let path: &[PathChar] = path.into();
            let os_str = std::ffi::OsStr::from_bytes(path);
            os_str.into()
        }
    }

    /// Fetches the path to the module root.
    #[inline]
    #[cfg(windows)]
    pub fn module_path(&self) -> PathBuf {
        use std::os::windows::ffi::OsStringExt;
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let path = (vtable.module_path)(ptr);
            let path: &[PathChar] = path.into();
            let os_str = std::ffi::OsString::from_wide(path);
            os_str.into()
        }
    }

    /// Fetches a reference to the modules [`ModuleInfo`].
    #[inline]
    pub fn module_info(&self) -> &ModuleInfo {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.module_info)(ptr) }
    }

    /// Fetches a reference to the modules [`ModuleInfo`].
    #[inline]
    pub fn module_loader(&self) -> &'static ModuleLoader {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.module_loader)(ptr) }
    }

    /// Instantiates the module.
    ///
    /// A module may disallow multiple instantiations.
    ///
    /// # Note
    ///
    /// This function will result in an unique instance, or an error, each time it is called.
    #[inline]
    pub fn new_instance(&self) -> Result<ObjArc<ModuleInstance>> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.new_instance)(ptr) }
    }
}

fimo_vtable! {
    /// VTable of a module.
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct ModuleVTable<id = "fimo::module::interfaces::module", marker = SendSyncMarker> {
        /// Fetches an internal vtable for the module.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static BaseInterface,
        /// Fetches the path to the module root.
        pub module_path: unsafe extern "C" fn(*const ()) -> SpanInner<PathChar, false>,
        /// Fetches a pointer to the modules [`ModuleInfo`].
        pub module_info: unsafe extern "C" fn(*const ()) -> *const ModuleInfo,
        /// Fetches a pointer to the [`ModuleLoader`] which loaded the module.
        pub module_loader: unsafe extern "C" fn(*const ()) -> &'static ModuleLoader,
        /// Instantiates the module.
        ///
        /// A module may disallow multiple instantiations.
        ///
        /// # Note
        ///
        /// This function will result in an unique instance, or an error, each time it is called.
        pub new_instance: unsafe extern "C" fn(*const ()) -> Result<ObjArc<ModuleInstance>>,
    }
}

fimo_object! {
    /// A type-erased module instance.
    pub struct ModuleInstance<vtable = ModuleInstanceVTable>;
}

impl ModuleInstance {
    /// Casts the `ModuleInstance` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<BaseInterface> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.inner)(ptr);

            // safety: we know that both ptr and vtable are valid so we can dereference.
            &*fimo_ffi::object::from_raw_parts(ptr, vtable)
        }
    }

    /// Fetches the parent module.
    #[inline]
    pub fn module(&self) -> ObjArc<Module> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.module)(ptr) }
    }

    /// Fetches a span of the available interfaces.
    ///
    /// The resulting descriptors can be used to instantiate the interfaces.
    #[inline]
    pub fn available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.available_interfaces)(ptr).into() }
    }

    /// Fetches the interface described by the interface descriptor.
    ///
    /// The interface is instantiated if it does not already exist.
    /// Multiple calls with the same interface will retrieve the same
    /// instance if is has not already been dropped.
    #[inline]
    pub fn interface(&self, i: &ModuleInterfaceDescriptor) -> Result<ObjArc<ModuleInterface>> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.interface)(ptr, i) }
    }

    /// Fetches the dependencies of an interface.
    #[inline]
    pub fn dependencies(
        &self,
        i: &ModuleInterfaceDescriptor,
    ) -> Result<&[ModuleInterfaceDescriptor]> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.dependencies)(ptr, i).map(|d| d.into()) }
    }

    /// Provides a core interface to the module instance.
    ///
    /// This interface may be used to fetch other loaded interfaces.
    ///
    /// May return an error if the instance does not require the interface.
    #[inline]
    pub fn set_core(
        &self,
        i: &ModuleInterfaceDescriptor,
        core: ObjArc<ModuleInterface>,
    ) -> Result<()> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.set_core)(ptr, i, core) }
    }
}

fimo_vtable! {
    /// VTable of a module instance.
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct ModuleInstanceVTable<id = "fimo::module::interfaces::module_instance", marker = SendSyncMarker> {
        /// Fetches an internal vtable for the instance.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static BaseInterface,
        /// Fetches the parent module.
        pub module: unsafe extern "C" fn(*const ()) -> ObjArc<Module>,
        /// Fetches a span of the available interfaces.
        ///
        /// The resulting descriptors can be used to instantiate the interfaces.
        pub available_interfaces:
            unsafe extern "C" fn(*const ()) -> SpanInner<ModuleInterfaceDescriptor, false>,
        /// Fetches the interface described by the interface descriptor.
        ///
        /// The interface is instantiated if it does not already exist.
        /// Multiple calls with the same interface will retrieve the same
        /// instance if is has not already been dropped.
        pub interface: unsafe extern "C" fn(
            *const (),
            *const ModuleInterfaceDescriptor
        ) -> Result<ObjArc<ModuleInterface>>,
        /// Fetches the dependencies of an interface.
        pub dependencies: unsafe extern "C" fn(
            *const (),
            *const ModuleInterfaceDescriptor
        ) -> Result<SpanInner<ModuleInterfaceDescriptor, false>>,
        /// Provides a core interface to the module instance.
        ///
        /// This interface may be used to fetch other loaded interfaces.
        ///
        /// May return an error if the instance does not require the interface.
        pub set_core: unsafe extern "C" fn(
            *const (),
            *const ModuleInterfaceDescriptor,
            ObjArc<ModuleInterface>
        ) -> Result<()>,
    }
}

fimo_object! {
    /// A type-erased module interface.
    pub struct ModuleInterface<vtable = ModuleInterfaceVTable>;
}

impl ModuleInterface {
    /// Casts the `ModuleInterface` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<BaseInterface> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.inner)(ptr);

            // safety: we know that both ptr and vtable are valid so we can dereference.
            &*fimo_ffi::object::from_raw_parts(ptr, vtable)
        }
    }

    /// Extracts the version of the implemented interface.
    #[inline]
    pub fn version(&self) -> Version {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.version)(ptr) }
    }

    /// Fetches the parent instance.
    #[inline]
    pub fn instance(&self) -> ObjArc<ModuleInstance> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.instance)(ptr) }
    }
}

fimo_vtable! {
    /// VTable of a module interface.
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct ModuleInterfaceVTable<id = "fimo::module::interfaces::module_interface", marker = SendSyncMarker> {
        /// Fetches an internal vtable for the interface.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static BaseInterface,
        /// Extracts the version of the implemented interface.
        pub version: unsafe extern "C" fn(*const ()) -> Version,
        /// Fetches the parent instance.
        pub instance: unsafe extern "C" fn(*const ()) -> ObjArc<ModuleInstance>,
    }
}
