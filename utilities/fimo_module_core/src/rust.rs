//! Definition of Rust modules.
use crate::{DynArc, DynArcBase, DynArcCaster, ModuleInfo, ModuleInterfaceDescriptor, ModulePtr};
use std::error::Error;
use std::marker::PhantomData;
use std::path::Path;

/// A type-erased module object.
///
/// # Safety
///
/// The underlying type must implement [`Send`] and [`Sync`].
pub struct ModuleObject<T: 'static> {
    _phantom: PhantomData<&'static T>,
    // makes `ModuleLoader` into a DST with size 0 and alignment 1.
    _inner: [()],
}

/// A type-erased module loader.
pub type ModuleLoader = ModuleObject<ModuleLoaderVTable>;

/// A type-erased module.
pub type Module = ModuleObject<ModuleVTable>;

/// A type-erased module module instance.
pub type ModuleInstance = ModuleObject<ModuleInstanceVTable>;

/// A type-erased module module interface.
pub type ModuleInterface = ModuleObject<ModuleInterfaceVTable>;

/// A [`DynArc`] for a [`Module`].
pub type ModuleArc = DynArc<Module, ModuleCaster>;

/// A [`DynArc`] for a [`ModuleInstance`].
pub type ModuleInstanceArc = DynArc<ModuleInstance, ModuleInstanceCaster>;

/// A [`DynArc`] for a [`ModuleInterface`].
pub type ModuleInterfaceArc = DynArc<ModuleInterface, ModuleInterfaceCaster>;

/// A [`Result`] alias.
pub type ModuleResult<T> = Result<T, Box<dyn Error>>;

impl<T: 'static> ModuleObject<T> {
    /// Splits the reference into a data- and vtable- pointer.
    #[inline]
    pub fn into_raw_parts(&self) -> (*const (), &'static T) {
        // safety: `&Self` has the same layout as `&[()]`
        let s: &[()] = unsafe { std::mem::transmute(self) };

        // safety: the values are properly initialized upon construction.
        let ptr = s.as_ptr();
        let vtable = unsafe { &*(s.len() as *const T) };

        (ptr, vtable)
    }

    /// Constructs a `*const FimoCore` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts(data: *const (), vtable: &'static T) -> *const Self {
        // `()` has size 0 and alignment 1, so it should be sound to use an
        // arbitrary ptr and length.
        let vtable_ptr = vtable as *const _ as usize;
        let s = std::ptr::slice_from_raw_parts(data, vtable_ptr);

        // safety: the types have the same layout
        unsafe { std::mem::transmute(s) }
    }
}

impl ModuleLoader {
    /// Fetches an internal [ModulePtr] to the loader.
    ///
    /// The ptr remains valid until the loader is dropped.
    #[inline]
    pub fn get_raw_ptr(&self) -> ModulePtr {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_raw_ptr)(ptr)
    }

    /// Extracts the type identifier of the raw loader.
    #[inline]
    pub fn get_raw_type_id(&self) -> &str {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_raw_type_id)(ptr) }
    }

    /// Removes all modules that aren't referenced by anyone from the cache,
    /// unloading them in the process.
    #[inline]
    pub fn evict_module_cache(&self) {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.evict_module_cache)(ptr)
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
    pub unsafe fn load_module<P: AsRef<Path>>(&'static self, path: P) -> ModuleResult<ModuleArc> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.load_module)(ptr, path.as_ref())
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
    ) -> ModuleResult<ModuleArc> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.load_module_raw)(ptr, path.as_ref())
    }
}

impl Module {
    /// Fetches an internal [ModulePtr] to the module.
    ///
    /// The ptr remains valid until the module is dropped.
    #[inline]
    pub fn get_raw_ptr(&self) -> ModulePtr {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_raw_ptr)(ptr)
    }

    /// Extracts the type identifier of the raw module.
    #[inline]
    pub fn get_raw_type_id(&self) -> &str {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_raw_type_id)(ptr) }
    }

    /// Fetches the path to the module root.
    #[inline]
    pub fn get_module_path(&self) -> &Path {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_module_path)(ptr) }
    }

    /// Fetches a reference to the modules `ModuleInfo`.
    #[inline]
    pub fn get_module_info(&self) -> &ModuleInfo {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_module_info)(ptr) }
    }

    /// Fetches a reference to the loader which loaded the module.
    #[inline]
    pub fn get_module_loader(&self) -> &'static ModuleLoader {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_module_loader)(ptr)
    }

    /// Instantiates the module.
    ///
    /// A module may disallow multiple instantiations.
    ///
    /// # Note
    ///
    /// This function will result in an unique instance, or an error, each time it is called.
    #[inline]
    pub fn create_instance(&self) -> ModuleResult<ModuleInstanceArc> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.create_instance)(ptr)
    }
}

impl ModuleInstance {
    /// Fetches an internal [ModulePtr] to the instance.
    ///
    /// The ptr remains valid until the instance is dropped.
    #[inline]
    pub fn get_raw_ptr(&self) -> ModulePtr {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_raw_ptr)(ptr)
    }

    /// Extracts the type identifier of the raw instance.
    #[inline]
    pub fn get_raw_type_id(&self) -> &str {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_raw_type_id)(ptr) }
    }

    /// Fetches the parent module.
    #[inline]
    pub fn get_module(&self) -> ModuleArc {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_module)(ptr)
    }

    /// Fetches a slice of the available interfaces.
    ///
    /// The resulting descriptors can be used to instantiate the interfaces.
    #[inline]
    pub fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_available_interfaces)(ptr) }
    }

    /// Fetches the interface described by the interface descriptor.
    ///
    /// The interface is instantiated if it does not already exist.
    /// Multiple calls with the same interface will retrieve the same
    /// instance if is has not already been dropped.
    #[inline]
    pub fn get_interface<D: AsRef<ModuleInterfaceDescriptor>>(
        &self,
        desc: D,
    ) -> ModuleResult<ModuleInterfaceArc> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_interface)(ptr, desc.as_ref())
    }

    /// Fetches the dependencies of an interface.
    #[inline]
    pub fn get_interface_dependencies<D: AsRef<ModuleInterfaceDescriptor>>(
        &self,
        desc: D,
    ) -> ModuleResult<&[ModuleInterfaceDescriptor]> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_interface_dependencies)(ptr, desc.as_ref()).map(|dep| unsafe { &*dep })
    }

    /// Provides an core interface to the module instance.
    ///
    /// This interface may be used to fetch other loaded interfaces.
    ///
    /// May return an error if the instance does not require the interface.
    #[inline]
    pub fn set_core_dependency<D: AsRef<ModuleInterfaceDescriptor>>(
        &self,
        desc: D,
        interface: ModuleInterfaceArc,
    ) -> ModuleResult<()> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.set_core_dependency)(ptr, desc.as_ref(), interface)
    }
}

impl ModuleInterface {
    /// Fetches an internal [ModulePtr] to the interface.
    ///
    /// The ptr remains valid until the interface is dropped.
    #[inline]
    pub fn get_raw_ptr(&self) -> ModulePtr {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_raw_ptr)(ptr)
    }

    /// Extracts the type identifier of the raw interface.
    #[inline]
    pub fn get_raw_type_id(&self) -> &str {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.get_raw_type_id)(ptr) }
    }

    /// Fetches the parent instance.
    #[inline]
    pub fn get_instance(&self) -> ModuleInstanceArc {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_instance)(ptr)
    }
}

impl<T: 'static> std::fmt::Debug for ModuleObject<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(ModuleObject)")
    }
}

unsafe impl<T: 'static> Send for ModuleObject<T> {}
unsafe impl<T: 'static> Sync for ModuleObject<T> {}

/// VTable of a module loader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleLoaderVTable {
    get_raw_ptr: fn(*const ()) -> ModulePtr,
    get_raw_type_id: fn(*const ()) -> *const str,
    evict_module_cache: fn(*const ()),
    load_module: fn(*const (), *const Path) -> ModuleResult<ModuleArc>,
    load_module_raw: fn(*const (), *const Path) -> ModuleResult<ModuleArc>,
}

impl ModuleLoaderVTable {
    /// Constructs a new `ModuleLoaderVTable`.
    #[inline]
    pub const fn new(
        get_raw_ptr: fn(*const ()) -> ModulePtr,
        get_raw_type_id: fn(*const ()) -> *const str,
        evict_module_cache: fn(*const ()),
        load_module: fn(*const (), *const Path) -> ModuleResult<ModuleArc>,
        load_module_raw: fn(*const (), *const Path) -> ModuleResult<ModuleArc>,
    ) -> Self {
        Self {
            get_raw_ptr,
            get_raw_type_id,
            evict_module_cache,
            load_module,
            load_module_raw,
        }
    }
}

/// VTable of a module.
#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleVTable {
    get_raw_ptr: fn(*const ()) -> ModulePtr,
    get_raw_type_id: fn(*const ()) -> *const str,
    get_module_path: fn(*const ()) -> *const Path,
    get_module_info: fn(*const ()) -> *const ModuleInfo,
    get_module_loader: fn(*const ()) -> &'static ModuleLoader,
    create_instance: fn(*const ()) -> ModuleResult<ModuleInstanceArc>,
}

impl ModuleVTable {
    /// Constructs a new `ModuleVTable`.
    #[inline]
    pub const fn new(
        get_raw_ptr: fn(*const ()) -> ModulePtr,
        get_raw_type_id: fn(*const ()) -> *const str,
        get_module_path: fn(*const ()) -> *const Path,
        get_module_info: fn(*const ()) -> *const ModuleInfo,
        get_module_loader: fn(*const ()) -> &'static ModuleLoader,
        create_instance: fn(*const ()) -> ModuleResult<ModuleInstanceArc>,
    ) -> Self {
        Self {
            get_raw_ptr,
            get_raw_type_id,
            get_module_path,
            get_module_info,
            get_module_loader,
            create_instance,
        }
    }
}

/// VTable of a module instance.
#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleInstanceVTable {
    get_raw_ptr: fn(*const ()) -> ModulePtr,
    get_raw_type_id: fn(*const ()) -> *const str,
    get_module: fn(*const ()) -> ModuleArc,
    get_available_interfaces: fn(*const ()) -> *const [ModuleInterfaceDescriptor],
    get_interface:
        fn(*const (), *const ModuleInterfaceDescriptor) -> ModuleResult<ModuleInterfaceArc>,
    get_interface_dependencies: fn(
        *const (),
        *const ModuleInterfaceDescriptor,
    ) -> ModuleResult<*const [ModuleInterfaceDescriptor]>,
    set_core_dependency:
        fn(*const (), *const ModuleInterfaceDescriptor, ModuleInterfaceArc) -> ModuleResult<()>,
}

impl ModuleInstanceVTable {
    /// Constructs a new `ModuleInstanceVTable`.
    #[inline]
    pub const fn new(
        get_raw_ptr: fn(*const ()) -> ModulePtr,
        get_raw_type_id: fn(*const ()) -> *const str,
        get_module: fn(*const ()) -> ModuleArc,
        get_available_interfaces: fn(*const ()) -> *const [ModuleInterfaceDescriptor],
        get_interface: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
        ) -> ModuleResult<ModuleInterfaceArc>,
        get_interface_dependencies: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
        ) -> ModuleResult<*const [ModuleInterfaceDescriptor]>,
        set_core_dependency: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
            ModuleInterfaceArc,
        ) -> ModuleResult<()>,
    ) -> Self {
        Self {
            get_raw_ptr,
            get_raw_type_id,
            get_module,
            get_available_interfaces,
            get_interface,
            get_interface_dependencies,
            set_core_dependency,
        }
    }
}

/// VTable of a module interface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleInterfaceVTable {
    get_raw_ptr: fn(*const ()) -> ModulePtr,
    get_raw_type_id: fn(*const ()) -> *const str,
    get_instance: fn(*const ()) -> ModuleInstanceArc,
}

/// [`DynArc`] caster.
#[derive(PartialEq, Copy, Clone, Debug)]
pub struct ModuleObjCaster<T: 'static> {
    vtable: &'static T,
}

impl<T: 'static> ModuleObjCaster<T> {
    /// Constructs a new `ModuleObjCaster`.
    pub fn new(vtable: &'static T) -> Self {
        Self { vtable }
    }
}

/// [`DynArc`] caster for a [`Module`].
pub type ModuleCaster = ModuleObjCaster<ModuleVTable>;

/// [`DynArc`] caster for a [`ModuleInstance`].
pub type ModuleInstanceCaster = ModuleObjCaster<ModuleInstanceVTable>;

/// [`DynArc`] caster for a [`ModuleInterface`].
pub type ModuleInterfaceCaster = ModuleObjCaster<ModuleInterfaceVTable>;

impl DynArcCaster<Module> for ModuleCaster {
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const Module {
        let ptr = base as *const _ as *const ();
        Module::from_raw_parts(ptr, self.vtable)
    }
}

impl DynArcCaster<ModuleInstance> for ModuleInstanceCaster {
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const ModuleInstance {
        let ptr = base as *const _ as *const ();
        ModuleInstance::from_raw_parts(ptr, self.vtable)
    }
}

impl DynArcCaster<ModuleInterface> for ModuleInterfaceCaster {
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const ModuleInterface {
        let ptr = base as *const _ as *const ();
        ModuleInterface::from_raw_parts(ptr, self.vtable)
    }
}
