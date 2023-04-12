use crate::{Error, ErrorKind, InterfaceDescriptor, ModuleInfo, PathChar, Result};
use fimo_ffi::ptr::{metadata, CastInto, DowncastSafeInterface, IBase, IBaseExt};
use fimo_ffi::{interface, DynObj, ObjArc, Version};
use std::marker::Unsize;
use std::path::{Path, PathBuf};

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "6533e721-5402-46bc-91e5-882b0e4ffec9",
    )]

    /// Interface of a module loader.
    pub frozen interface IModuleLoader : 'static + marker IBase + marker Send + marker Sync {
        /// Casts the [`IModuleLoader`] to an internal object.
        fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

        /// Removes all modules that aren't referenced by anyone from the cache,
        /// unloading them in the process.
        fn evict_module_cache(&self);

        /// Loads a new module from a path to the module root.
        ///
        /// An implementation is allowed to keep an internal reference count to
        /// each module and return an unique [`ObjArc`].
        ///
        /// # Safety
        ///
        /// - The module must be exposed in a way understood by the module loader.
        /// - The module ABI must match the loader ABI.
        ///
        /// Violating these invariants may lead to undefined behavior.
        unsafe fn load_module(&'static self, path: &Path) -> Result<ObjArc<DynObj<dyn IModule>>>;

        /// Loads a new module from a path to the module library.
        ///
        /// An implementation is allowed to keep an internal reference count to
        /// each module and return an unique [`ObjArc`].
        ///
        /// # Safety
        ///
        /// - The module must be exposed in a way understood by the module loader.
        /// - The module ABI must match the loader ABI.
        ///
        /// Violating these invariants may lead to undefined behavior.
        unsafe fn load_module_raw(&'static self, path: &Path) -> Result<ObjArc<DynObj<dyn IModule>>>;
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "ccca2ad2-38e4-4c0d-9975-8f8e472ab03a",
    )]

    /// Interface of a module.
    pub frozen interface IModule : marker IBase + marker Send + marker Sync {
        /// Casts the [`IModule`] to an internal object.
        fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

        /// Fetches the path to the module root.
        #[interface_cfg(mapping = "exclude")]
        fn module_path(&self) -> PathBuf {
            let path = self.module_path_slice();
            module_path_slice_to_path(path).into_owned()
        }

        /// Fetches the path to the module root.
        fn module_path_slice(&self) -> &[PathChar];

        /// Fetches a reference to the modules [`ModuleInfo`].
        fn module_info(&self) -> &ModuleInfo;

        /// Fetches a reference to the modules [`IModuleLoader`].
        fn module_loader(&self) -> &'static DynObj<dyn IModuleLoader>;

        /// Binds a service to the module.
        ///
        /// Services are lightweight optional interfaces shared
        /// globally to the entire module.
        fn bind_service(&self, service: &'static DynObj<dyn IModuleInterface>);

        /// Instantiates the module.
        ///
        /// A module may disallow multiple instantiations.
        ///
        /// # Note
        ///
        /// This function will result in an unique instance, or an error, each time it is called.
        fn new_instance(&self) -> Result<ObjArc<DynObj<dyn IModuleInstance>>>;
    }
}

#[inline]
#[cfg(unix)]
pub(crate) fn module_path_slice_to_path(path: &[PathChar]) -> std::borrow::Cow<'_, Path> {
    use std::os::unix::ffi::OsStrExt;
    let os_str = std::ffi::OsStr::from_bytes(path);
    std::borrow::Cow::Borrowed(os_str.as_ref())
}

#[inline]
#[cfg(windows)]
pub(crate) fn module_path_slice_to_path(path: &[PathChar]) -> std::borrow::Cow<'_, Path> {
    use std::os::windows::ffi::OsStringExt;
    let os_str = std::ffi::OsString::from_wide(path);
    std::borrow::Cow::Owned(os_str.into())
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "e0c7335e-4cfe-44fc-909b-2e02f3f139b1",
    )]

    /// Interface of a module instance.
    pub frozen interface IModuleInstance : marker IBase + marker Send + marker Sync {
        /// Casts the [`IModuleInstance`] to an internal object.
        fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

        /// Fetches the parent module.
        fn module(&self) -> ObjArc<DynObj<dyn IModule>>;

        /// Fetches a span of the available interfaces.
        ///
        /// The resulting descriptors can be used to instantiate the interfaces.
        fn available_interfaces(&self) -> &[InterfaceDescriptor];

        /// Fetches the interface described by the interface descriptor.
        ///
        /// The interface is instantiated if it does not already exist.
        /// Multiple calls with the same interface will retrieve the same
        /// instance if is has not already been dropped.
        fn interface(
            &self,
            i: &InterfaceDescriptor,
        ) -> Result<ObjArc<DynObj<dyn IModuleInterface>>>;

        /// Fetches the dependencies of an interface.
        fn dependencies(&self, i: &InterfaceDescriptor) -> Result<&[InterfaceDescriptor]>;

        /// Binds an interface to an interface of the module instance.
        ///
        /// This interface will be used the next time the interface is constructed.
        ///
        /// May return an error if the interface does not require the interface.
        fn bind_interface(
            &self,
            desc: &InterfaceDescriptor,
            interface: ObjArc<DynObj<dyn IModuleInterface>>,
        ) -> Result<()>;
    }
}

/// Marker trait for interfaces.
pub trait FimoInterface<'a>: IModuleInterface + DowncastSafeInterface<'a> {
    /// Name of the interface.
    const NAME: &'static str;
    /// Version of the interface.
    const VERSION: Version;
    /// Required extensions.
    const EXTENSIONS: &'static [&'static str];

    /// Constructs a new [`InterfaceDescriptor`] describing the interface.
    #[inline]
    #[must_use]
    fn new_descriptor() -> InterfaceDescriptor {
        InterfaceDescriptor {
            name: Self::NAME.into(),
            version: Self::VERSION,
            extensions: Self::EXTENSIONS
                .iter()
                .cloned()
                .map(|ext| ext.into())
                .collect(),
        }
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "9b0e35ac-b20d-4c75-8b42-16d99a8cf182",
    )]

    /// Interface of a module interface.
    pub frozen interface IModuleInterface : marker IBase + marker Send + marker Sync {
        /// Casts the [`IModuleInterface`] to an internal object.
        fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

        /// Extracts the name of the implemented interface.
        fn name(&self) -> &str;

        /// Extracts the version of the implemented interface.
        fn version(&self) -> Version;

        /// Extracts the implemented extensions of the interface.
        fn extensions(&self) -> fimo_ffi::Vec<fimo_ffi::String>;

        /// Constructs the descriptor of the given interface.
        #[interface_cfg(mapping = "exclude")]
        fn descriptor(&self) -> InterfaceDescriptor {
            let name = self.name().into();
            let version = self.version();
            let extensions = self.extensions();

            InterfaceDescriptor::new(name, version, extensions)
        }

        /// Fetches an extension from the interface.
        fn extension(&self, name: &str) -> Option<&DynObj<dyn IBase + Send + Sync>>;

        /// Fetches the parent instance.
        fn instance(&self) -> ObjArc<DynObj<dyn IModuleInstance>>;
    }
}

/// Tries to downcast the interface.
#[inline]
pub fn try_downcast<'a, T, U>(obj: &fimo_ffi::DynObj<U>) -> Result<&DynObj<T>>
where
    T: FimoInterface<'a> + Unsize<dyn IBase + 'a> + ?Sized,
    U: CastInto<'a, dyn IModuleInterface + 'a> + ?Sized,
{
    let obj: &DynObj<dyn IModuleInterface + 'a> = obj.cast_super();

    // before downcasting we must ensure that the versions
    // are compatible and that all extensions are present.
    if !T::VERSION.is_compatible(&obj.version()) {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!(
                "interface version incompatible, requested `{}`, available `{}`",
                T::VERSION,
                obj.version()
            ),
        ));
    }

    for &ext in T::EXTENSIONS {
        if obj.extension(ext).is_none() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!("extension `{}` not found", ext),
            ));
        }
    }

    let inner: &DynObj<dyn IBase> = obj.as_inner().cast_super();
    match inner.downcast_interface::<T>() {
        Some(i) => unsafe { Ok(&*(i as *const DynObj<T>)) },
        None => Err(Error::new(
            ErrorKind::InvalidArgument,
            format!(
                "interface `{}` not available, got `{}`",
                T::NAME,
                metadata(inner).interface_name(),
            ),
        )),
    }
}

/// Tries to downcast the interface [`ObjArc`].
#[inline]
pub fn try_downcast_arc<'a, T, U>(obj: ObjArc<DynObj<U>>) -> Result<ObjArc<DynObj<T>>>
where
    T: FimoInterface<'a> + Unsize<dyn IBase + 'a> + ?Sized,
    U: CastInto<'a, dyn IModuleInterface + 'a> + ?Sized,
{
    // the inner object always equals the original object, except for the different vtable.
    // because of that we can simply perform the casting ourselves and rebuild the arc.
    let inner = try_downcast::<T, _>(&*obj)? as *const _;
    let (_, alloc) = ObjArc::into_raw_parts(obj);
    unsafe { Ok(ObjArc::from_raw_parts(inner, alloc)) }
}
