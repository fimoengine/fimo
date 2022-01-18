use crate::{
    Error, ErrorKind, ModuleInfo, ModuleInterfaceDescriptor, PathChar, Result, SendSyncMarker,
};
use fimo_ffi::object::ObjectWrapper;
use fimo_ffi::vtable::IBaseInterface;
use fimo_ffi::{fimo_object, fimo_vtable, ObjArc, Object, Optional, SpanInner, StrInner};
use fimo_version_core::Version;
use std::path::{Path, PathBuf};

/// Defines a new interface.
///
/// # Examples
///
/// ```
/// #![feature(const_fn_trait_bound)]
/// #![feature(const_fn_fn_ptr_basics)]
///
/// use fimo_version_core::Version;
/// use fimo_module_core::{fimo_vtable, fimo_interface};
///
/// fimo_vtable! {
///     #![uuid(0xa0fe4d60, 0xa526, 0x4e9e, 0x97e2, 0x4c675aa6b324)]
///     struct VTable;
/// }
///
/// // interface without extensions.
/// fimo_interface! {
///     struct Simple<vtable = VTable> {
///         name: "MyInterface",
///         version: Version::new_short(1, 1, 0),
///     }
/// }
///
/// // interface without extensions.
/// fimo_interface! {
///     struct Complex<vtable = VTable> {
///         name: "MyInterface",
///         version: Version::new_short(1, 1, 0),
///         extensions: ["ext1", "ext2"]
///     }
/// }
///
/// ```
#[macro_export]
macro_rules! fimo_interface {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<vtable = $vtable:ty> {
            name: $i_name:literal,
            version: $i_version:expr,
            extensions: [ $($i_ext:literal),* ] $(,)?
        }
    ) => {
        $crate::fimo_object! {
            $(#[$attr])*
            $vis struct $name<vtable = $vtable>;
        }
        impl $crate::FimoInterface for $name {
            const NAME: &'static str = $i_name;
            const VERSION: fimo_version_core::Version = $i_version;
            const EXTENSIONS: &'static [&'static str] = &[ $($i_ext),* ];
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<vtable = $vtable:ty> {
            name: $i_name:literal,
            version: $i_version:expr $(,)?
        }
    ) => {
        $crate::fimo_interface! {
            $(#[$attr])*
            $vis struct $name<vtable = $vtable> {
                name: $i_name,
                version: $i_version,
                extensions: [],
            }
        }
    }
}

fimo_object! {
    /// A type-erased module loader.
    pub struct IModuleLoader<vtable = IModuleLoaderVTable>;
}

impl IModuleLoader {
    /// Casts the `IModuleLoader` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<IBaseInterface> {
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
    pub unsafe fn load_module<P: AsRef<Path>>(&'static self, path: P) -> Result<ObjArc<IModule>> {
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
    ) -> Result<ObjArc<IModule>> {
        let (ptr, vtable) = self.into_raw_parts();
        Self::load_inner(path, ptr, vtable.load_module_raw)
    }

    #[cfg(unix)]
    #[inline]
    fn load_inner<P: AsRef<Path>>(
        path: P,
        ptr: *const (),
        f: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<IModule>>,
    ) -> Result<ObjArc<IModule>> {
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
        f: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<IModule>>,
    ) -> Result<ObjArc<IModule>> {
        use std::os::windows::ffi::OsStrExt;

        let path = path.as_ref();
        let os_str = path.as_os_str();
        let buf: Vec<PathChar> = OsStrExt::encode_wide(os_str).collect();
        let bytes = buf.as_slice();
        unsafe { f(ptr, bytes.into()) }
    }
}

fimo_vtable! {
    /// VTable of a [`IModuleLoader`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0x6533e721, 0x5402, 0x46bc, 0x91e5, 0x882b0e4ffec9)]
    pub struct IModuleLoaderVTable {
        /// Fetches an internal vtable for the loader.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static IBaseInterface,
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
        pub load_module: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<IModule >>,
        /// Loads a new module from a path to the module library.
        ///
        /// # Safety
        ///
        /// - The module must be exposed in a way understood by the module loader.
        /// - The module ABI must match the loader ABI.
        ///
        /// Violating these invariants may lead to undefined behaviour.
        pub load_module_raw: unsafe extern "C" fn(*const (), SpanInner<PathChar, false>) -> Result<ObjArc<IModule >>,
    }
}

fimo_object! {
    /// Interface of a module.
    pub struct IModule<vtable = IModuleVTable>;
}

impl IModule {
    /// Casts the `IModule` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<IBaseInterface> {
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
    pub fn module_loader(&self) -> &'static IModuleLoader {
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
    pub fn new_instance(&self) -> Result<ObjArc<IModuleInstance>> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.new_instance)(ptr) }
    }
}

fimo_vtable! {
    /// VTable of a [`IModule`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0xccca2ad2, 0x38e4, 0x4c0d, 0x9975, 0x8f8e472ab03a)]
    pub struct IModuleVTable {
        /// Fetches an internal vtable for the module.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static IBaseInterface,
        /// Fetches the path to the module root.
        pub module_path: unsafe extern "C" fn(*const ()) -> SpanInner<PathChar, false>,
        /// Fetches a pointer to the modules [`ModuleInfo`].
        pub module_info: unsafe extern "C" fn(*const ()) -> *const ModuleInfo,
        /// Fetches a pointer to the [`ModuleLoader`] which loaded the module.
        pub module_loader: unsafe extern "C" fn(*const ()) -> &'static IModuleLoader,
        /// Instantiates the module.
        ///
        /// A module may disallow multiple instantiations.
        ///
        /// # Note
        ///
        /// This function will result in an unique instance, or an error, each time it is called.
        pub new_instance: unsafe extern "C" fn(*const ()) -> Result<ObjArc<IModuleInstance >>,
    }
}

fimo_object! {
    /// Interface of a module instance.
    pub struct IModuleInstance<vtable = IModuleInstanceVTable>;
}

impl IModuleInstance {
    /// Casts the `IModuleInstance` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<IBaseInterface> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.inner)(ptr);

            // safety: we know that both ptr and vtable are valid so we can dereference.
            &*fimo_ffi::object::from_raw_parts(ptr, vtable)
        }
    }

    /// Fetches the parent module.
    #[inline]
    pub fn module(&self) -> ObjArc<IModule> {
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
    pub fn interface(&self, i: &ModuleInterfaceDescriptor) -> Result<ObjArc<IModuleInterface>> {
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
        core: ObjArc<IModuleInterface>,
    ) -> Result<()> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.set_core)(ptr, i, core) }
    }
}

fimo_vtable! {
    /// VTable of a [`IModuleInstance`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0xe0c7335e, 0x4cfe, 0x44fc, 0x909b, 0x2e02f3f139b1)]
    pub struct IModuleInstanceVTable {
        /// Fetches an internal vtable for the instance.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static IBaseInterface,
        /// Fetches the parent module.
        pub module: unsafe extern "C" fn(*const ()) -> ObjArc<IModule>,
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
        ) -> Result<ObjArc<IModuleInterface >>,
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
            ObjArc<IModuleInterface>
        ) -> Result<()>,
    }
}

/// Marker trait for interfaces.
pub trait FimoInterface {
    /// Name of the interface.
    const NAME: &'static str;
    /// Version of the interface.
    const VERSION: Version;
    /// Required extensions.
    const EXTENSIONS: &'static [&'static str];

    /// Constructs a new [`ModuleInterfaceDescriptor`] describing the interface.
    #[inline]
    #[must_use]
    fn new_descriptor() -> ModuleInterfaceDescriptor {
        ModuleInterfaceDescriptor {
            name: Self::NAME.into(),
            version: Self::VERSION,
            extensions: Self::EXTENSIONS.iter().cloned().map(|ext| ext.into()).collect()
        }
    }
}

fimo_object! {
    /// Interface of a module interface.
    pub struct IModuleInterface<vtable = IModuleInterfaceVTable>;
}

impl IModuleInterface {
    /// Casts the `IModuleInterface` to an internal object.
    #[inline]
    pub fn as_inner(&self) -> &Object<IBaseInterface> {
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

    /// Fetches an extension from the interface.
    #[inline]
    pub fn extension(&self, name: &str) -> Option<&Object<IBaseInterface>> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            (vtable.extension)(ptr, name.into())
                .into_rust()
                .map(|e| &*e)
        }
    }

    /// Fetches the parent instance.
    #[inline]
    pub fn instance(&self) -> ObjArc<IModuleInstance> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.instance)(ptr) }
    }

    /// Tries to downcast the interface.
    #[inline]
    pub fn try_downcast<T: ObjectWrapper + FimoInterface + ?Sized>(
        &self,
    ) -> std::result::Result<&T, Error> {
        // before downcasting we must ensure that the versions
        // are compatible and that all extensions are present.
        if !T::VERSION.is_compatible(&self.version()) {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "interface version incompatible, requested `{}`, available `{}`",
                    T::VERSION,
                    self.version()
                ),
            ));
        }

        for &ext in T::EXTENSIONS {
            if self.extension(ext).is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidArgument,
                    format!("extension `{}` not found", ext),
                ));
            }
        }

        let inner = self.as_inner();
        match inner.try_cast::<T::VTable>() {
            Ok(i) => unsafe { Ok(&*T::from_object_raw(i)) },
            Err(e) => Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "interface type mismatch, requested `{}`, available `{}`",
                    e.required, e.available
                ),
            )),
        }
    }

    /// Tries to downcast the interface.
    #[inline]
    pub fn try_downcast_arc<T: ObjectWrapper + FimoInterface + ?Sized>(
        i: ObjArc<IModuleInterface>,
    ) -> std::result::Result<ObjArc<T>, Error> {
        // the inner object always equals the original object, except for the different vtable.
        // because of that we can simply perform the casting ourselves and rebuild the arc.
        let inner = i.try_downcast::<T>()? as *const _;
        let (_, alloc) = ObjArc::into_raw_parts(i);
        unsafe { Ok(ObjArc::from_raw_parts(inner, alloc)) }
    }
}

fimo_vtable! {
    /// VTable of a [`IModuleInterface`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0x9b0e35ac, 0xb20d, 0x4c75, 0x8b42, 0x16d99a8cf182)]
    pub struct IModuleInterfaceVTable {
        /// Fetches an internal vtable for the interface.
        pub inner: unsafe extern "C" fn(*const ()) -> &'static IBaseInterface,
        /// Extracts the version of the implemented interface.
        pub version: unsafe extern "C" fn(*const ()) -> Version,
        /// Fetches an extension from the interface.
        pub extension: unsafe extern "C" fn(*const (), StrInner<false>) -> Optional<*const Object<IBaseInterface >>,
        /// Fetches the parent instance.
        pub instance: unsafe extern "C" fn(*const ()) -> ObjArc<IModuleInstance>,
    }
}
