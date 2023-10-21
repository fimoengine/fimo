//! Implementation of an [`AppContext`].
use std::path::Path;

use crate::{InterfaceDescriptor, InterfaceQuery, ModuleInfo};
use fimo_ffi::provider::IProvider;
use fimo_ffi::ptr::IBase;
use fimo_ffi::{interface, DynObj, ObjBox, Version};

pub use private::{AppContext, LoaderManifest, ModuleDeclaration};

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "f029f6a2-82dd-43bf-ad84-a4cb755c11aa",
    )]

    /// Interface for the context.
    pub interface IContext: marker IBase + marker Send + marker Sync {
        /// Loads a module located at `path` to the current context.
        fn load_module(
            &mut self,
            path: &Path,
            features: &[fimo_ffi::String],
        ) -> crate::Result<&DynObj<dyn IModule + '_>>;

        /// Fetches all modules that are loaded into the context.
        fn loaded_modules(&self) -> fimo_ffi::Vec<&DynObj<dyn IModule + '_>>;

        /// Checks if an interface is present in the context.
        fn has_interface(&self, query: InterfaceQuery) -> bool;

        /// Fetches an interface that matches the query.
        fn get_interface(&self, query: InterfaceQuery) -> crate::Result<&DynObj<dyn IInterface + '_>>;
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "a5118167-62c5-4683-9ba4-b3b55e2a902f"
    )]

    /// Context access from an interface.
    pub interface IInterfaceContext: marker IBase + marker Send + marker Sync {
        /// Checks if an interface is present in the context.
        fn has_interface(&self, query: InterfaceQuery) -> bool;

        /// Fetches an interface that matches the query.
        fn get_interface(&self, query: InterfaceQuery) -> crate::Result<&DynObj<dyn IInterface + '_>>;
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "653631f1-8860-4cfc-bba5-4ec177e512ab"
    )]

    /// Interface of a module.
    pub interface IModule: marker IBase + marker Send + marker Sync {
        /// Fetches the path to the module root.
        fn module_path(&self) -> &Path;

        /// Fetches a reference to the modules [`ModuleInfo`].
        fn module_info(&self) -> &ModuleInfo;

        /// Fetches the features enabled in the module.
        fn features(&self) -> &[fimo_ffi::String];

        /// Fetches a slice of all available interfaces.
        ///
        /// The resulting descriptors can be used to instantiate the interfaces.
        fn interfaces(&self) -> &[InterfaceDescriptor];
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "fce578d4-8000-4d59-8694-43438df648fe"
    )]

    /// Interface exposed by all modules, required for instantiating it.
    pub interface IModuleBuilder: marker IBase + marker Send + marker Sync {
        /// Fetches the path to the module root.
        fn module_path(&self) -> &Path;

        /// Fetches a reference to the modules [`ModuleInfo`].
        fn module_info(&self) -> &ModuleInfo;

        /// Fetches the features enabled in the module.
        fn features(&self) -> &[fimo_ffi::String];

        /// Fetches all [`IInterfaceBuilder`] contained in the module.
        fn interfaces(&self) -> &[&DynObj<dyn IInterfaceBuilder + '_>];
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "bb66b2d3-edd9-425d-a003-164c96cb4f83"
    )]

    /// Builder for an interface from a module.
    pub interface IInterfaceBuilder: marker IBase + marker Send + marker Sync {
        /// Extracts the name of the implemented interface.
        fn name(&self) -> &str;

        /// Extracts the version of the implemented interface.
        fn version(&self) -> Version;

        /// Extracts the implemented extensions of the interface.
        fn extensions(&self) -> &[fimo_ffi::String];

        /// Constructs the descriptor of the given interface.
        #[interface_cfg(mapping = "exclude")]
        fn descriptor(&self) -> InterfaceDescriptor {
            let name = self.name().into();
            let version = self.version();
            let extensions = self.extensions();

            InterfaceDescriptor::new(name, version, extensions.into())
        }

        /// Queries the dependencies required to instantiate the interface.
        fn dependencies(&self) -> &[InterfaceQuery];

        /// Queries the optional dependencies that can be consumed by the interface.
        fn optional_dependencies(&self) -> &[InterfaceQuery];

        /// Builds an instance of the interface.
        #[interface_cfg(phantom_parameter = "&'a ()")]
        fn build<'a>(&self, context: &'a DynObj<dyn IInterfaceContext + 'a>) -> crate::Result<ObjBox<DynObj<dyn IInterface + 'a>>>;
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "016c372e-5f4f-4156-b0dd-0223879d91a7"
    )]

    /// Interface of a module interface.
    pub interface IInterface: marker IBase + marker Send + marker Sync + IProvider @ version("0.0") {
        /// Extracts the name of the implemented interface.
        fn name(&self) -> &str;

        /// Extracts the version of the implemented interface.
        fn version(&self) -> Version;

        /// Extracts the implemented extensions of the interface.
        fn extensions(&self) -> &[fimo_ffi::String];

        /// Constructs the descriptor of the given interface.
        #[interface_cfg(mapping = "exclude")]
        fn descriptor(&self) -> InterfaceDescriptor {
            let name = self.name().into();
            let version = self.version();
            let extensions = self.extensions();

            InterfaceDescriptor::new(name, version, extensions.into())
        }
    }
}

mod private {
    use super::{
        IContext, IInterface, IInterfaceBuilder, IInterfaceContext, IModule, IModuleBuilder,
    };
    use crate::{InterfaceDescriptor, InterfaceQuery};
    use fimo_ffi::{
        error::{wrap_error, Error, ErrorKind},
        marshal::CTypeBridge,
        provider::IProvider,
        Version,
    };
    use fimo_ffi::{DynObj, ObjArc, ObjBox, Object};
    use libloading::Library;
    use petgraph::stable_graph::{NodeIndex, StableDiGraph};
    use petgraph::Direction;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::HashMap,
        fmt::{Debug, Formatter},
        fs::File,
        io::BufReader,
        marker::PhantomData,
        path::{Path, PathBuf},
        ptr::NonNull,
        sync::OnceLock,
    };

    /// Exports a module to enable its loading with the Rust loader.
    #[macro_export]
    macro_rules! module {
        ($load:expr) => {
            #[no_mangle]
            #[doc(hidden)]
            pub static MODULE_DECLARATION: $crate::context::ModuleDeclaration =
                $crate::context::ModuleDeclaration {
                    fimo_version: $crate::context::ModuleDeclaration::FIMO_VERSION_COMPAT,
                    load_fn: __private_load_module,
                };

            unsafe extern "C-unwind" fn __private_load_module<'a>(
                path: <&std::path::Path as $crate::fimo_ffi::marshal::CTypeBridge>::Type,
                features: <&[$crate::fimo_ffi::String] as $crate::fimo_ffi::marshal::CTypeBridge>::Type,
                _: std::marker::PhantomData<&'a ()>
            ) -> <$crate::Result<$crate::fimo_ffi::ObjBox<$crate::fimo_ffi::DynObj<dyn $crate::context::IModuleBuilder + 'a>>> as $crate::fimo_ffi::marshal::CTypeBridge>::Type {
                let path: &std::path::Path = $crate::fimo_ffi::marshal::CTypeBridge::demarshal(path);
                let features: &[$crate::fimo_ffi::String] = $crate::fimo_ffi::marshal::CTypeBridge::demarshal(features);
                let res: $crate::Result<$crate::fimo_ffi::ObjBox<$crate::fimo_ffi::DynObj<dyn $crate::context::IModuleBuilder>>> = $load(path, features);
                $crate::fimo_ffi::marshal::CTypeBridge::marshal(res)
            }
        };
    }

    #[derive(Object)]
    #[interfaces(IInterfaceContext)]
    struct InterfaceContext {
        interfaces: InterfaceMap<ObjArc<Interface>>,
    }

    impl InterfaceContext {
        fn new() -> Self {
            Self {
                interfaces: InterfaceMap::new(),
            }
        }

        fn provide(&mut self, interface: ObjArc<Interface>) {
            self.interfaces.insert(interface.descriptor(), interface);
        }
    }

    impl IInterfaceContext for InterfaceContext {
        fn has_interface(&self, query: InterfaceQuery) -> bool {
            self.interfaces.contains(query)
        }

        fn get_interface(
            &self,
            query: InterfaceQuery,
        ) -> crate::Result<&DynObj<dyn IInterface + '_>> {
            if let Some((_, x)) = self.interfaces.get(query.clone()) {
                let interface_ptr = ObjArc::as_ptr(x);
                let interface_ptr = fimo_ffi::ptr::coerce_obj_raw(interface_ptr);

                // Safety: We use the dependency graph to guarantee that the
                // module does not outlive the reference.
                unsafe { crate::Result::Ok(&*interface_ptr) }
            } else {
                let error = Error::new(
                    ErrorKind::NotFound,
                    format!("The required interface {query:?} does not exist"),
                );
                crate::Result::Err(error)
            }
        }
    }

    #[derive(Object)]
    #[interfaces(IInterface)]
    struct Interface {
        interface: OnceLock<ObjBox<DynObj<dyn IInterface>>>,
        context: InterfaceContext,
        builder: NonNull<DynObj<dyn IInterfaceBuilder>>,
    }

    unsafe impl Send for Interface where DynObj<dyn IInterfaceBuilder>: Send {}
    unsafe impl Sync for Interface where DynObj<dyn IInterfaceBuilder>: Sync {}

    impl Interface {
        fn new(context: InterfaceContext, builder: &DynObj<dyn IInterfaceBuilder + '_>) -> Self {
            // Safety: We can artificially prolong the lifetime of the builder, because
            // we know that the builder will be dropped before the module containing it.
            let builder = unsafe {
                std::mem::transmute::<
                    &DynObj<dyn IInterfaceBuilder + '_>,
                    &DynObj<dyn IInterfaceBuilder>,
                >(builder)
            };
            let builder = NonNull::from(builder);

            Self {
                interface: OnceLock::new(),
                context,
                builder,
            }
        }

        fn init_interface(&self) -> &DynObj<dyn IInterface + '_> {
            self.interface.get_or_init(|| {
                let context = fimo_ffi::ptr::coerce_obj(&self.context);
                let x = self.builder().build(context).unwrap();

                unsafe {
                    std::mem::transmute::<
                        ObjBox<DynObj<dyn IInterface + '_>>,
                        ObjBox<DynObj<dyn IInterface>>,
                    >(x)
                }
            })
        }

        fn get_interface(&self) -> Option<&DynObj<dyn IInterface + '_>> {
            self.interface.get().map(|x| &**x)
        }

        fn builder(&self) -> &DynObj<dyn IInterfaceBuilder + '_> {
            // Safety: We know that the builder is still alive at this point.
            unsafe { self.builder.as_ref() }
        }
    }

    impl Debug for Interface {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Interface").finish_non_exhaustive()
        }
    }

    impl IProvider for Interface {
        fn provide<'a>(&'a self, demand: &mut fimo_ffi::provider::Demand<'a>) {
            self.init_interface().provide(demand);
        }
    }

    impl IInterface for Interface {
        fn name(&self) -> &str {
            if let Some(x) = self.get_interface() {
                x.name()
            } else {
                self.builder().name()
            }
        }

        fn version(&self) -> Version {
            if let Some(x) = self.get_interface() {
                x.version()
            } else {
                self.builder().version()
            }
        }

        fn extensions(&self) -> &[fimo_ffi::String] {
            if let Some(x) = self.get_interface() {
                x.extensions()
            } else {
                self.builder().extensions()
            }
        }
    }

    #[derive(Object)]
    #[interfaces(IModule)]
    struct Module {
        interfaces: Vec<InterfaceDescriptor>,
        builder: ObjBox<DynObj<dyn IModuleBuilder>>,
        _lib: ObjBox<Library>,
    }

    impl Module {
        fn new(builder: ObjBox<DynObj<dyn IModuleBuilder + '_>>, lib: ObjBox<Library>) -> Self {
            // Safety: We know that `builder` stems from `lib`, so
            // it is valid as long as `lib` remains alive. Since we
            // don't let anyonce access the builder it is safe to extend
            // the lifetime.
            let builder = unsafe {
                std::mem::transmute::<
                    ObjBox<DynObj<dyn IModuleBuilder + '_>>,
                    ObjBox<DynObj<dyn IModuleBuilder>>,
                >(builder)
            };

            let mut interfaces = vec![];
            for &x in builder.interfaces() {
                interfaces.push(x.descriptor());
            }

            Self {
                interfaces,
                builder,
                _lib: lib,
            }
        }
    }

    impl Debug for Module {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Module")
                .field("path", &self.module_path())
                .field("module_info", &self.module_info())
                .field("features", &self.features())
                .field("interfaces", &self.interfaces)
                .finish_non_exhaustive()
        }
    }

    impl IModule for Module {
        fn module_path(&self) -> &Path {
            self.builder.module_path()
        }

        fn module_info(&self) -> &crate::ModuleInfo {
            self.builder.module_info()
        }

        fn features(&self) -> &[fimo_ffi::String] {
            self.builder.features()
        }

        fn interfaces(&self) -> &[InterfaceDescriptor] {
            &self.interfaces
        }
    }

    /// Rust module manifest.
    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "schema")]
    pub enum LoaderManifest {
        /// Version `0` manifest schema.
        #[serde(rename = "0")]
        V0 {
            /// Path to the library.
            library_path: PathBuf,
        },
    }

    /// Declaration of a module.
    #[derive(Copy, Clone)]
    pub struct ModuleDeclaration {
        /// Used Rust version.
        pub fimo_version: Version,
        /// Load function.
        #[allow(clippy::type_complexity)]
        #[allow(improper_ctypes_definitions)]
        pub load_fn: for<'a> unsafe extern "C-unwind" fn(
            <&Path as CTypeBridge>::Type,
            <&[fimo_ffi::String] as CTypeBridge>::Type,
            PhantomData<&'a ()>,
        ) -> <crate::Result<
            ObjBox<DynObj<dyn IModuleBuilder + 'a>>,
        > as CTypeBridge>::Type,
    }

    impl ModuleDeclaration {
        /// Path from a module root to the manifest.
        pub const MODULE_MANIFEST_PATH: &'static str = "module.json";

        /// Name of the module declaration.
        pub const MODULE_DECLARATION_NAME: &'static str = "MODULE_DECLARATION";

        /// Redeclaration of [`fimo_ffi::FIMO_VERSION_COMPAT`].
        pub const FIMO_VERSION_COMPAT: Version = fimo_ffi::FIMO_VERSION_COMPAT;

        const MODULE_DECLARATION_NAME_WITH_NULL: &'static [u8] = b"MODULE_DECLARATION\0";

        unsafe fn load(
            &self,
            path: &Path,
            features: &[fimo_ffi::String],
        ) -> crate::Result<ObjBox<DynObj<dyn IModuleBuilder + '_>>> {
            let path = CTypeBridge::marshal(path);
            let features = CTypeBridge::marshal(features);
            let x = (self.load_fn)(path, features, PhantomData);
            CTypeBridge::demarshal(x)
        }
    }

    impl Debug for ModuleDeclaration {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ModuleDeclaration")
                .field("fimo_version", &self.fimo_version)
                .finish()
        }
    }

    #[derive(Debug)]
    struct Loader;

    impl Loader {
        #[allow(clippy::type_complexity)]
        fn load_module(&self, path: &Path, features: &[fimo_ffi::String]) -> crate::Result<Module> {
            let manifest_path = path.join(ModuleDeclaration::MODULE_MANIFEST_PATH);
            let file = File::open(manifest_path)
                .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?;
            let buf_reader = BufReader::new(file);
            let manifest: LoaderManifest = serde_json::from_reader(buf_reader)
                .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?;

            unsafe {
                match manifest {
                    LoaderManifest::V0 { library_path, .. } => {
                        let library_path = path.join(library_path);
                        self.load_module_raw(&library_path, features)
                    }
                }
            }
        }

        unsafe fn load_module_raw(
            &self,
            path: &Path,
            features: &[fimo_ffi::String],
        ) -> crate::Result<Module> {
            let lib = libloading::Library::new(path)
                .map_err(|e| Error::new(ErrorKind::Unknown, wrap_error(e)))?;
            let lib = ObjBox::new(lib);

            let module_declaration = match lib.get::<*const ModuleDeclaration>(
                ModuleDeclaration::MODULE_DECLARATION_NAME_WITH_NULL,
            ) {
                Ok(s) => s,
                Err(e) => return Err(Error::new(ErrorKind::Internal, wrap_error(e))),
            };

            let module_declaration = **module_declaration;
            if !module_declaration
                .fimo_version
                .is_compatible(&ModuleDeclaration::FIMO_VERSION_COMPAT)
            {
                return Err(Error::new(
                    ErrorKind::FailedPrecondition,
                    format!(
                        "Version {} is not compatible with {}",
                        module_declaration.fimo_version,
                        ModuleDeclaration::FIMO_VERSION_COMPAT
                    ),
                ));
            }

            let builder = module_declaration.load(path, features)?;
            let module = Module::new(builder, lib);
            crate::Result::Ok(module)
        }
    }

    #[derive(Debug, Clone)]
    enum ResourceData {
        Root,
        Inteface(ObjArc<Interface>),
        Module(ObjArc<Module>),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct ResourceId(usize);

    impl ResourceId {
        const ROOT_ID: Self = Self(0);

        fn new(resource: &ResourceData) -> Self {
            match &resource {
                ResourceData::Inteface(x) => Self(ObjArc::as_ptr(x) as usize),
                ResourceData::Module(x) => Self(ObjArc::as_ptr(x) as usize),
                ResourceData::Root => Self::ROOT_ID,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct Resource {
        id: ResourceId,
        data: ResourceData,
    }

    impl Resource {
        fn new_interface(x: Interface) -> Self {
            let data = ResourceData::Inteface(ObjArc::new(x));
            Self {
                id: ResourceId::new(&data),
                data,
            }
        }

        fn new_module(x: Module) -> Self {
            let data = ResourceData::Module(ObjArc::new(x));
            Self {
                id: ResourceId::new(&data),
                data,
            }
        }
    }

    #[derive(Debug)]
    struct InterfaceMap<T> {
        map: HashMap<InterfaceDescriptor, (InterfaceDescriptor, T)>,
    }

    impl<T> InterfaceMap<T> {
        fn new() -> Self {
            Self {
                map: Default::default(),
            }
        }

        fn contains(&self, query: InterfaceQuery) -> bool {
            self.get(query).is_some()
        }

        fn get(&self, query: InterfaceQuery) -> Option<(&InterfaceDescriptor, &T)> {
            let min_query_version = match query.version {
                crate::VersionQuery::Exact(x) => Version::new_short(x.major, 0, 0),
                crate::VersionQuery::Minimum(x) => Version::new_short(x.major, 0, 0),
                crate::VersionQuery::Range { min, .. } => Version::new_short(min.major, 0, 0),
            };
            let min_interface =
                InterfaceDescriptor::new(query.name.clone(), min_query_version, fimo_ffi::vec![]);

            let entry = self.map.get(&min_interface)?;
            if query.query_matches(&entry.0) {
                Some((&entry.0, &entry.1))
            } else {
                None
            }
        }

        fn insert(
            &mut self,
            interface: InterfaceDescriptor,
            value: T,
        ) -> Option<(InterfaceDescriptor, T)> {
            let min_interface = InterfaceDescriptor::new(
                interface.name.clone(),
                Version::new_short(interface.version.major, 0, 0),
                fimo_ffi::vec![],
            );

            self.map.insert(min_interface, (interface, value))
        }

        fn remove(&mut self, query: InterfaceQuery) -> Option<(InterfaceDescriptor, T)> {
            let min_query_version = match query.version {
                crate::VersionQuery::Exact(x) => Version::new_short(x.major, 0, 0),
                crate::VersionQuery::Minimum(x) => Version::new_short(x.major, 0, 0),
                crate::VersionQuery::Range { min, .. } => Version::new_short(min.major, 0, 0),
            };
            let min_interface =
                InterfaceDescriptor::new(query.name.clone(), min_query_version, fimo_ffi::vec![]);

            let entry = self.map.get_mut(&min_interface)?;
            if query.query_matches(&entry.0) {
                self.map.remove(&min_interface)
            } else {
                None
            }
        }
    }

    /// Context for all modules and interfaces
    /// contained in an application.
    #[derive(Debug)]
    pub struct AppContext {
        context: Context,
    }

    impl AppContext {
        /// Constructs a new instance.
        pub fn new() -> Self {
            Self {
                context: Context::new(),
            }
        }

        /// Enters the `AppContext`.
        pub fn enter<T>(&mut self, f: impl FnOnce(&mut DynObj<dyn IContext>) -> T) -> T {
            let context = fimo_ffi::ptr::coerce_obj_mut(&mut self.context);
            let res = f(context);
            self.context.shutdown();
            res
        }
    }

    impl Default for AppContext {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Debug, Object)]
    #[interfaces(IContext)]

    struct Context {
        loader: Loader,
        resources: ResourceMap,
    }

    impl Context {
        fn new() -> Self {
            Self {
                loader: Loader,
                resources: ResourceMap::new(),
            }
        }

        fn shutdown(&mut self) {
            self.resources.cleanup_resources();
        }
    }

    impl IContext for Context {
        fn load_module(
            &mut self,
            path: &Path,
            features: &[fimo_ffi::String],
        ) -> crate::Result<&DynObj<dyn IModule>> {
            if let Some((_, module)) = self.resources.get_module_by_path(path) {
                let module_features = module.features();
                if !features.iter().all(|f| features.contains(f)) {
                    let error = Error::new(
                        ErrorKind::Unavailable,
                        format!(
                            "The module at path {path:?} does not enables \
                        all required features {features:?}, enabled {module_features:?}"
                        ),
                    );
                    return crate::Result::Err(error);
                }

                let module_ptr = ObjArc::as_ptr(&module);
                let module_ptr = fimo_ffi::ptr::coerce_obj_raw(module_ptr);

                // Safety: We use the dependency graph to guarantee that the
                // module does not outlive the reference.
                unsafe { crate::Result::Ok(&*module_ptr) }
            } else {
                let module = self.loader.load_module(path, features)?;
                let module_id = self.resources.add_module(module)?;
                let (_, module) = self.resources.get_module(module_id)?;

                let mut inserted = vec![];
                for &builder in module.builder.interfaces() {
                    let mut dependencies = HashMap::new();
                    for dependency in builder.dependencies() {
                        if let Some(x) = self.resources.get_interface_by_query(dependency.clone()) {
                            dependencies.insert(x.0, x.1);
                        } else {
                            for i in inserted.into_iter().rev() {
                                self.resources.remove_interface(i).unwrap();
                            }
                            self.resources.remove_module(module_id).unwrap();

                            let error = Error::new(
                                ErrorKind::NotFound,
                                format!(
                                    "Can not sattisfy the query {dependency:?} for interface {:?}",
                                    builder.descriptor()
                                ),
                            );
                            return crate::Result::Err(error);
                        };
                    }

                    for dependency in builder.optional_dependencies() {
                        if let Some(x) = self.resources.get_interface_by_query(dependency.clone()) {
                            dependencies.insert(x.0, x.1);
                        }
                    }

                    let mut context = InterfaceContext::new();
                    for dep in dependencies.values() {
                        context.provide(dep.clone());
                    }

                    let interface = Interface::new(context, builder);
                    let interface_id = match self.resources.add_interface(interface, module_id) {
                        Ok(x) => x,
                        Err(e) => {
                            for i in inserted.into_iter().rev() {
                                self.resources.remove_interface(i).unwrap();
                            }
                            self.resources.remove_module(module_id).unwrap();
                            return crate::Result::Err(e);
                        }
                    };
                    inserted.push(interface_id);

                    for dep in dependencies.into_keys() {
                        match self.resources.add_interface_dependency(dep, interface_id) {
                            Ok(_) => {}
                            Err(e) => {
                                for i in inserted.into_iter().rev() {
                                    self.resources.remove_interface(i).unwrap();
                                }
                                self.resources.remove_module(module_id).unwrap();
                                return crate::Result::Err(e);
                            }
                        }
                    }
                }

                assert!(self.resources.is_acyclic());

                let module_ptr = ObjArc::as_ptr(&module);
                let module_ptr = fimo_ffi::ptr::coerce_obj_raw(module_ptr);

                // Safety: We use the dependency graph to guarantee that the
                // module does not outlive the reference.
                unsafe { crate::Result::Ok(&*module_ptr) }
            }
        }

        fn loaded_modules(&self) -> fimo_ffi::Vec<&DynObj<dyn super::IModule>> {
            let modules = self.resources.get_modules();
            modules
                .into_iter()
                .map(|module| {
                    let module_ptr = ObjArc::as_ptr(&module);
                    let module_ptr = fimo_ffi::ptr::coerce_obj_raw(module_ptr);

                    // Safety: We use the dependency graph to guarantee that the
                    // module does not outlive the reference.
                    unsafe { &*module_ptr }
                })
                .collect()
        }

        fn has_interface(&self, query: InterfaceQuery) -> bool {
            self.resources.get_interface_by_query(query).is_some()
        }

        fn get_interface(
            &self,
            query: InterfaceQuery,
        ) -> crate::Result<&DynObj<dyn super::IInterface>> {
            if let Some((_, x)) = self.resources.get_interface_by_query(query.clone()) {
                let interface_ptr = ObjArc::as_ptr(&x);
                let interface_ptr = fimo_ffi::ptr::coerce_obj_raw(interface_ptr);

                // Safety: We use the dependency graph to guarantee that the
                // module does not outlive the reference.
                unsafe { crate::Result::Ok(&*interface_ptr) }
            } else {
                let error = Error::new(
                    ErrorKind::NotFound,
                    format!("The required interface {query:?} does not exist"),
                );
                crate::Result::Err(error)
            }
        }
    }

    #[derive(Debug)]
    struct ResourceMap {
        modules: HashMap<PathBuf, Resource>,
        interfaces: InterfaceMap<Resource>,
        dependency_map: StableDiGraph<Resource, (), usize>,
        resources: HashMap<ResourceId, NodeIndex<usize>>,
    }

    impl ResourceMap {
        fn new() -> Self {
            let mut map = ResourceMap {
                modules: Default::default(),
                interfaces: InterfaceMap::new(),
                dependency_map: StableDiGraph::with_capacity(1, 1),
                resources: Default::default(),
            };

            let node = map.dependency_map.add_node(Resource {
                id: ResourceId::ROOT_ID,
                data: ResourceData::Root,
            });
            map.resources.insert(ResourceId::ROOT_ID, node);

            map
        }

        fn add_interface(
            &mut self,
            interface: Interface,
            module_id: ResourceId,
        ) -> crate::Result<ResourceId> {
            let (node, _) = self.get_module(module_id)?;
            let descriptor = interface.descriptor();
            let interface = Resource::new_interface(interface);
            let interface_id = interface.id;

            let interface_node = self.dependency_map.add_node(interface.clone());
            self.dependency_map.add_edge(node, interface_node, ());
            self.resources.insert(interface_id, interface_node);
            self.interfaces.insert(descriptor, interface);
            crate::Result::Ok(interface_id)
        }

        fn add_interface_dependency(
            &mut self,
            dependency: ResourceId,
            dependent: ResourceId,
        ) -> crate::Result<()> {
            let (dependency_node, _) = self.get_interface(dependency)?;
            let (dependent_node, _) = self.get_interface(dependent)?;

            self.dependency_map
                .add_edge(dependency_node, dependent_node, ());
            crate::Result::Ok(())
        }

        fn add_module(&mut self, module: Module) -> crate::Result<ResourceId> {
            let root = self.resources.get(&ResourceId::ROOT_ID).cloned().unwrap();
            let module_path = module.module_path().to_path_buf();
            let module = Resource::new_module(module);
            let module_id = module.id;

            let module_node = self.dependency_map.add_node(module.clone());
            self.dependency_map.add_edge(root, module_node, ());
            self.resources.insert(module_id, module_node);
            self.modules.insert(module_path, module);
            crate::Result::Ok(module_id)
        }

        fn remove_interface(&mut self, interface_id: ResourceId) -> crate::Result<()> {
            let (node, interface) = self.get_interface(interface_id)?;

            if self
                .dependency_map
                .edges_directed(node, Direction::Outgoing)
                .next()
                .is_some()
            {
                let error = Error::new(
                    ErrorKind::FailedPrecondition,
                    format!("The interface {interface:?} can not be removed, as it is being used"),
                );
                return crate::Result::Err(error);
            }

            self.dependency_map.remove_node(node);
            self.resources.remove(&interface_id);
            self.interfaces.remove(interface.descriptor().into());
            crate::Result::Ok(())
        }

        fn remove_module(&mut self, module_id: ResourceId) -> crate::Result<()> {
            let (node, module) = self.get_module(module_id)?;
            if self
                .dependency_map
                .edges_directed(node, Direction::Outgoing)
                .next()
                .is_some()
            {
                let error = Error::new(
                    ErrorKind::FailedPrecondition,
                    format!(
                        "The module {module:?} can not be removed, as it still owns some interfaces"
                    ),
                );
                return crate::Result::Err(error);
            }

            self.dependency_map.remove_node(node);
            self.resources.remove(&module_id);
            self.modules.remove(module.module_path());
            crate::Result::Ok(())
        }

        fn get_interface(
            &self,
            interface_id: ResourceId,
        ) -> crate::Result<(NodeIndex<usize>, ObjArc<Interface>)> {
            let node = if let Some(node) = self.resources.get(&interface_id) {
                *node
            } else {
                let error = Error::new(
                    ErrorKind::NotFound,
                    format!("The interface {interface_id:?} does not exist"),
                );
                return crate::Result::Err(error);
            };

            let resource = self.dependency_map.node_weight(node).unwrap();
            match &resource.data {
                ResourceData::Inteface(x) => crate::Result::Ok((node, x.clone())),
                _ => {
                    let error = Error::new(
                        ErrorKind::FailedPrecondition,
                        format!("The resource {interface_id:?} is not an interface"),
                    );
                    crate::Result::Err(error)
                }
            }
        }

        fn get_interface_by_query(
            &self,
            query: InterfaceQuery,
        ) -> Option<(ResourceId, ObjArc<Interface>)> {
            let (_, interface) = self.interfaces.get(query)?;
            match &interface.data {
                ResourceData::Inteface(x) => Some((interface.id, x.clone())),
                _ => unreachable!(),
            }
        }

        fn get_module(
            &self,
            module_id: ResourceId,
        ) -> crate::Result<(NodeIndex<usize>, ObjArc<Module>)> {
            let node = if let Some(node) = self.resources.get(&module_id) {
                *node
            } else {
                let error = Error::new(
                    ErrorKind::NotFound,
                    format!("The module {module_id:?} does not exist"),
                );
                return crate::Result::Err(error);
            };

            let resource = self.dependency_map.node_weight(node).unwrap();
            match &resource.data {
                ResourceData::Module(x) => crate::Result::Ok((node, x.clone())),
                _ => {
                    let error = Error::new(
                        ErrorKind::FailedPrecondition,
                        format!("The resource {module_id:?} is not a module"),
                    );
                    crate::Result::Err(error)
                }
            }
        }

        fn get_module_by_path(&self, path: &Path) -> Option<(ResourceId, ObjArc<Module>)> {
            let module = self.modules.get(path)?;
            match &module.data {
                ResourceData::Module(x) => Some((module.id, x.clone())),
                _ => unreachable!(),
            }
        }

        fn get_modules(&self) -> Vec<ObjArc<Module>> {
            self.modules
                .values()
                .map(|m| match &m.data {
                    ResourceData::Module(m) => m.clone(),
                    _ => unreachable!(),
                })
                .collect()
        }

        fn is_acyclic(&self) -> bool {
            !petgraph::algo::is_cyclic_directed(&self.dependency_map)
        }

        fn cleanup_resources(&mut self) {
            let mut dirty = true;
            let mut modules = vec![];

            while dirty {
                dirty = false;
                let externals = self
                    .dependency_map
                    .externals(Direction::Outgoing)
                    .collect::<Vec<_>>();

                for external in externals {
                    let resource = self.dependency_map.node_weight(external).unwrap().clone();
                    if resource.id == ResourceId::ROOT_ID {
                        continue;
                    }

                    match &resource.data {
                        ResourceData::Inteface(_) => {
                            dirty = true;
                            self.remove_interface(resource.id).unwrap();
                        }
                        ResourceData::Module(_) => {
                            dirty = true;
                            self.remove_module(resource.id).unwrap();
                            modules.push(resource);
                        }
                        _ => {}
                    }
                }
            }

            drop(modules)
        }
    }
}
