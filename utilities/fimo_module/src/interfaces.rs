use crate::{Error, ErrorKind, FFIResult, ModuleInfo, ModuleInterfaceDescriptor, PathChar, Result};
use fimo_ffi::obj_arc::RawObjArc;
use fimo_ffi::ptr::{
    from_raw, into_raw, metadata, CastInto, DowncastSafeInterface, IBase, IBaseExt, ObjInterface,
    ObjMetadata, ObjectId, RawObj,
};
use fimo_ffi::span::{ConstSpan, ConstSpanPtr};
use fimo_ffi::str::ConstStrPtr;
use fimo_ffi::{interface, vtable, ConstStr, DynObj, ObjArc, Optional, ReprC, Version};
use std::borrow::Cow;
use std::marker::Unsize;
use std::path::{Path, PathBuf};

/// Interface of a module loader.
#[interface(
    uuid = "6533e721-5402-46bc-91e5-882b0e4ffec9",
    vtable = "IModuleLoaderVTable"
)]
pub trait IModuleLoader: IBase + Send + Sync {
    /// Casts the [`IModuleLoader`] to an internal object.
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

    /// Removes all modules that aren't referenced by anyone from the cache,
    /// unloading them in the process.
    fn evict_module_cache(&self);

    /// Loads a new module from a path to the module root.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behavior.
    unsafe fn load_module(&'static self, path: &'_ Path) -> Result<ObjArc<DynObj<dyn IModule>>>;

    /// Loads a new module from a path to the module library.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behavior.
    unsafe fn load_module_raw(&'static self, path: &'_ Path)
        -> Result<ObjArc<DynObj<dyn IModule>>>;
}

impl<'a, T: CastInto<dyn IModuleLoader + 'a> + Send + Sync + ?Sized> IModuleLoader for DynObj<T>
where
    DynObj<T>: IBase + Send + Sync,
{
    #[inline]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        let vtable: &IModuleLoaderVTable = metadata(self).super_vtable();
        unsafe { &*from_raw((vtable.inner)(self as *const _ as _)) }
    }

    #[inline]
    fn evict_module_cache(&self) {
        let vtable: &IModuleLoaderVTable = metadata(self).super_vtable();
        unsafe { (vtable.evict_module_cache)(self as *const _ as _) }
    }

    #[inline]
    unsafe fn load_module(&'static self, path: &'_ Path) -> Result<ObjArc<DynObj<dyn IModule>>> {
        let vtable: &IModuleLoaderVTable = metadata(self).super_vtable();
        load_impl(path, self as *const _ as _, vtable.load_module)
    }

    #[inline]
    unsafe fn load_module_raw(
        &'static self,
        path: &'_ Path,
    ) -> Result<ObjArc<DynObj<dyn IModule>>> {
        let vtable: &IModuleLoaderVTable = metadata(self).super_vtable();
        load_impl(path, self as *const _ as _, vtable.load_module_raw)
    }
}

#[inline]
#[cfg(unix)]
unsafe fn load_impl(
    path: &Path,
    ptr: *const (),
    f: unsafe extern "C-unwind" fn(
        *const (),
        ConstSpan<'_, PathChar>,
    ) -> FFIResult<RawObjArc<RawObj<dyn IModule>>>,
) -> Result<ObjArc<DynObj<dyn IModule>>> {
    use std::os::unix::ffi::OsStrExt;

    let os_str = path.as_os_str();
    let bytes = OsStrExt::as_bytes(os_str);
    (f)(ptr, bytes.into()).into_rust().map(|a| a.into())
}

#[inline]
#[cfg(windows)]
unsafe fn load_impl(
    path: &Path,
    ptr: *const (),
    f: unsafe extern "C-unwind" fn(
        *const (),
        ConstSpan<'_, PathChar>,
    ) -> FFIResult<RawObjArc<RawObj<dyn IModule>>>,
) -> Result<ObjArc<DynObj<dyn IModule>>> {
    use std::os::windows::ffi::OsStrExt;

    let os_str = path.as_os_str();
    let buf: Vec<PathChar> = OsStrExt::encode_wide(os_str).collect();
    let bytes = buf.as_slice();
    (f)(ptr, bytes.into()).into_rust().map(|a| a.into())
}

// Manual implementation as we require a 'static lifetime.
/// VTable of a [`IModuleLoader`].
#[vtable(interface = "IModuleLoader")]
#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct IModuleLoaderVTable {
    /// Fetches an internal vtable for the loader.
    pub inner: unsafe extern "C-unwind" fn(*const ()) -> RawObj<dyn IBase + Send + Sync>,
    /// Removes all modules that aren't referenced by anyone from the cache,
    /// unloading them in the process.
    pub evict_module_cache: unsafe extern "C-unwind" fn(*const ()),
    /// Loads a new module from a path to the module root.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    pub load_module: unsafe extern "C-unwind" fn(
        *const (),
        ConstSpan<'_, PathChar>,
    ) -> FFIResult<RawObjArc<RawObj<dyn IModule>>>,
    /// Loads a new module from a path to the module library.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    pub load_module_raw: unsafe extern "C-unwind" fn(
        *const (),
        ConstSpan<'_, PathChar>,
    )
        -> FFIResult<RawObjArc<RawObj<dyn IModule>>>,
}

impl IModuleLoaderVTable {
    /// Constructs a new vtable for a given type.
    #[inline]
    pub const fn new_for<T>() -> Self
    where
        T: IModuleLoader + ObjectId + 'static,
    {
        Self::new_for_embedded::<T, dyn IModuleLoader>(0)
    }

    /// Constructs a new vtable for a given type and interface with a custom offset.
    #[inline]
    pub const fn new_for_embedded<T, Dyn>(offset: usize) -> Self
    where
        T: IModuleLoader + ObjectId + Unsize<Dyn> + 'static,
        Dyn: ObjInterface + ?Sized + 'static,
    {
        unsafe extern "C-unwind" fn inner<T: IModuleLoader>(
            ptr: *const (),
        ) -> RawObj<dyn IBase + Send + Sync> {
            let t = &*(ptr as *const T);
            let inner = t.as_inner();
            into_raw(inner)
        }

        unsafe extern "C-unwind" fn evict_module_cache<T: IModuleLoader>(ptr: *const ()) {
            let t = &*(ptr as *const T);
            t.evict_module_cache();
        }

        unsafe extern "C-unwind" fn load<T: IModuleLoader + 'static>(
            ptr: *const (),
            path: ConstSpan<'_, PathChar>,
        ) -> FFIResult<RawObjArc<RawObj<dyn IModule>>> {
            let t = &*(ptr as *const T);
            let path = module_path_slice_to_path(path.into());
            let path = match path {
                Cow::Borrowed(p) => p,
                Cow::Owned(ref p) => p.as_path(),
            };

            t.load_module(path).map(|a| a.into()).into()
        }

        unsafe extern "C-unwind" fn load_raw<T: IModuleLoader + 'static>(
            ptr: *const (),
            path: ConstSpan<'_, PathChar>,
        ) -> FFIResult<RawObjArc<RawObj<dyn IModule>>> {
            let t = &*(ptr as *const T);
            let path = module_path_slice_to_path(path.into());
            let path = match path {
                Cow::Borrowed(p) => p,
                Cow::Owned(ref p) => p.as_path(),
            };

            t.load_module_raw(path).map(|a| a.into()).into()
        }

        Self::new_embedded::<T, Dyn>(
            offset,
            inner::<T>,
            evict_module_cache::<T>,
            load::<T>,
            load_raw::<T>,
        )
    }
}

/// Interface of a module.
#[interface(
    uuid = "ccca2ad2-38e4-4c0d-9975-8f8e472ab03a",
    vtable = "IModuleVTable",
    generate()
)]
pub trait IModule: IBase + Send + Sync {
    /// Casts the [`IModule`] to an internal object.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ObjMetadata<dyn IBase + Send + Sync>",
        into = "fimo_ffi::ptr::metadata",
        from_expr = "unsafe { &*fimo_ffi::ptr::from_raw_parts(self as *const _ as _, res) }"
    )]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

    /// Fetches the path to the module root.
    #[inline]
    #[vtable_info(ignore)]
    fn module_path(&self) -> PathBuf {
        let path = self.module_path_slice();
        module_path_slice_to_path(path).into_owned()
    }

    /// Fetches the path to the module root.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ConstSpanPtr<PathChar>",
        into = "Into::into",
        from_expr = "unsafe { res.deref().into() }"
    )]
    fn module_path_slice(&self) -> &[PathChar];

    /// Fetches a reference to the modules [`ModuleInfo`].
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "*const ModuleInfo",
        from_expr = "unsafe { &*res }"
    )]
    fn module_info(&self) -> &ModuleInfo;

    /// Fetches a reference to the modules [`IModuleLoader`].
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "RawObj<dyn IModuleLoader>",
        into = "fimo_ffi::ptr::into_raw",
        from_expr = "unsafe { &*fimo_ffi::ptr::from_raw(res) }"
    )]
    fn module_loader(&self) -> &'static DynObj<dyn IModuleLoader>;

    /// Instantiates the module.
    ///
    /// A module may disallow multiple instantiations.
    ///
    /// # Note
    ///
    /// This function will result in an unique instance, or an error, each time it is called.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "FFIResult<RawObjArc<RawObj<dyn IModuleInstance>>>",
        into_expr = "let res = FFIResult::from_rust(res)?; FFIResult::Ok(res.into())",
        from_expr = "let res = res.into_rust()?; Ok(res.into())"
    )]
    fn new_instance(&self) -> Result<ObjArc<DynObj<dyn IModuleInstance>>>;
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

/// Interface of a module instance.
#[interface(
    uuid = "e0c7335e-4cfe-44fc-909b-2e02f3f139b1",
    vtable = "IModuleInstanceVTable",
    generate()
)]
pub trait IModuleInstance: IBase + Send + Sync {
    /// Casts the [`IModuleInstance`] to an internal object.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ObjMetadata<dyn IBase + Send + Sync>",
        into = "fimo_ffi::ptr::metadata",
        from_expr = "unsafe { &*fimo_ffi::ptr::from_raw_parts(self as *const _ as _, res) }"
    )]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

    /// Fetches the parent module.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "RawObjArc<RawObj<dyn IModule>>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn module(&self) -> ObjArc<DynObj<dyn IModule>>;

    /// Fetches a span of the available interfaces.
    ///
    /// The resulting descriptors can be used to instantiate the interfaces.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ConstSpanPtr<ModuleInterfaceDescriptor>",
        into = "Into::into",
        from_expr = "unsafe { res.deref().into() }"
    )]
    fn available_interfaces(&self) -> &[ModuleInterfaceDescriptor];

    /// Fetches the interface described by the interface descriptor.
    ///
    /// The interface is instantiated if it does not already exist.
    /// Multiple calls with the same interface will retrieve the same
    /// instance if is has not already been dropped.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "FFIResult<RawObjArc<RawObj<dyn IModuleInterface>>>",
        into_expr = "let res = FFIResult::from_rust(res)?; FFIResult::Ok(res.into())",
        from_expr = "let res = res.into_rust()?; Ok(res.into())"
    )]
    fn interface(
        &self,
        #[vtable_info(
            type = "*const ModuleInterfaceDescriptor",
            from_expr = "let p_1 = &*p_1;"
        )]
        i: &ModuleInterfaceDescriptor,
    ) -> Result<ObjArc<DynObj<dyn IModuleInterface>>>;

    /// Fetches the dependencies of an interface.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "FFIResult<ConstSpanPtr<ModuleInterfaceDescriptor>>",
        into_expr = "let res = FFIResult::from_rust(res)?; FFIResult::Ok(res.into())",
        from_expr = "let res = res.into_rust()?; unsafe { Ok(res.deref().into()) }"
    )]
    fn dependencies(
        &self,
        #[vtable_info(
            type = "*const ModuleInterfaceDescriptor",
            from_expr = "let p_1 = &*p_1;"
        )]
        i: &ModuleInterfaceDescriptor,
    ) -> Result<&[ModuleInterfaceDescriptor]>;

    /// Binds an interface to an interface of the module instance.
    ///
    /// This interface will be used the next time the interface is constructed.
    ///
    /// May return an error if the interface does not require the interface.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "FFIResult<u8>",
        into_expr = "let _ = FFIResult::from_rust(res)?; FFIResult::Ok(0)",
        from_expr = "let _ = res.into_rust()?; Ok(())"
    )]
    fn bind_interface(
        &self,
        #[vtable_info(
            type = "*const ModuleInterfaceDescriptor",
            from_expr = "let p_1 = &*p_1;"
        )]
        desc: &ModuleInterfaceDescriptor,
        #[vtable_info(
            type = "RawObjArc<RawObj<dyn IModuleInterface>>",
            from = "Into::into",
            into = "Into::into"
        )]
        interface: ObjArc<DynObj<dyn IModuleInterface>>,
    ) -> Result<()>;
}

/// Marker trait for interfaces.
pub trait FimoInterface: IModuleInterface + ObjInterface + DowncastSafeInterface {
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
            extensions: Self::EXTENSIONS
                .iter()
                .cloned()
                .map(|ext| ext.into())
                .collect(),
        }
    }
}

/// Interface of a module interface.
#[interface(
    uuid = "9b0e35ac-b20d-4c75-8b42-16d99a8cf182",
    vtable = "IModuleInterfaceVTable",
    generate()
)]
pub trait IModuleInterface: IBase + Send + Sync {
    /// Casts the [`IModuleInterface`] to an internal object.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ObjMetadata<dyn IBase + Send + Sync>",
        into = "fimo_ffi::ptr::metadata",
        from_expr = "unsafe { &*fimo_ffi::ptr::from_raw_parts(self as *const _ as _, res) }"
    )]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync>;

    /// Extracts the name of the implemented interface.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ConstStrPtr",
        into = "Into::into",
        from_expr = "unsafe { res.deref().into() }"
    )]
    fn name(&self) -> &str;

    /// Extracts the version of the implemented interface.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn version(&self) -> Version;

    /// Extracts the implemented extensions of the interface.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "ConstSpanPtr<ConstStrPtr>",
        into_expr = "std::mem::transmute::<ConstSpanPtr<_>, ConstSpanPtr<ConstStrPtr>>(res.into())",
        from_expr = "unsafe { 
            std::mem::transmute::<ConstSpanPtr<ConstStrPtr>, ConstSpanPtr<&str>>(res).deref().into() 
        }"
    )]
    fn extensions(&self) -> &[&str];

    /// Constructs the descriptor of the given interface.
    #[inline]
    #[vtable_info(ignore)]
    fn descriptor(&self) -> ModuleInterfaceDescriptor {
        let name = self.name().into();
        let version = self.version();
        let extensions = self.extensions().iter().map(|&ext| ext.into()).collect();

        ModuleInterfaceDescriptor::new(name, version, extensions)
    }

    /// Fetches an extension from the interface.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<RawObj<dyn IBase + Send + Sync>>",
        into_expr = "let res = Optional::from_rust(res)?; Optional::Some(fimo_ffi::ptr::into_raw(res))",
        from_expr = "let res = res.into_rust()?; unsafe { Some(&*fimo_ffi::ptr::from_raw(res)) }"
    )]
    fn extension(
        &self,
        #[vtable_info(type = "ConstStr<'_>", into = "Into::into", from = "Into::into")] name: &str,
    ) -> Option<&DynObj<dyn IBase + Send + Sync>>;

    /// Fetches the parent instance.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "RawObjArc<RawObj<dyn IModuleInstance>>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn instance(&self) -> ObjArc<DynObj<dyn IModuleInstance>>;
}

/// Tries to downcast the interface.
#[inline]
pub fn try_downcast<'a, T, U>(obj: &fimo_ffi::DynObj<U>) -> Result<&DynObj<T>>
where
    T: FimoInterface + Unsize<dyn IBase> + ?Sized + 'a,
    U: CastInto<dyn IModuleInterface + 'a> + ?Sized,
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
    T: FimoInterface + Unsize<dyn IBase> + ?Sized + 'a,
    U: CastInto<dyn IModuleInterface + 'a> + ?Sized,
{
    // the inner object always equals the original object, except for the different vtable.
    // because of that we can simply perform the casting ourselves and rebuild the arc.
    let inner = try_downcast::<T, _>(&*obj)? as *const _;
    let (_, alloc) = ObjArc::into_raw_parts(obj);
    unsafe { Ok(ObjArc::from_raw_parts(inner, alloc)) }
}
