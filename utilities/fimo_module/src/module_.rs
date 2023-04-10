//! Implementation of a [`ModuleBuilderBuilder`].
use crate::{
    context::{IInterface, IInterfaceBuilder, IModuleBuilder, InterfaceContext},
    InterfaceQuery, ModuleInfo,
};
use fimo_ffi::{
    ptr::{FetchVTable, ObjInterface},
    DynObj, ObjBox, Object, Version,
};
use std::{
    collections::HashSet,
    fmt::Debug,
    marker::Unsize,
    path::{Path, PathBuf},
};

/// Builder for a [`IModuleBuilder`].
pub struct ModuleBuilderBuilder {
    info: ModuleInfo,
    features: HashSet<String>,
    #[allow(clippy::type_complexity)]
    builders: Vec<Box<dyn FnOnce(&Path, &[String]) -> InterfaceBuilder>>,
}

impl ModuleBuilderBuilder {
    /// Constructs a new instance.
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            info: ModuleInfo {
                name: name.into(),
                version: version.into(),
            },
            features: Default::default(),
            builders: Default::default(),
        }
    }

    /// Defines a new feature for the module.
    pub fn with_feature(mut self, feature: &str) -> Self {
        self.features.insert(feature.into());
        self
    }

    /// Defines a new interface that will be instantiated with the module.
    pub fn with_interface<T>(mut self) -> Self
    where
        T: Interface,
    {
        let builder = Box::new(|path: &'_ _, feat: &'_ _| InterfaceBuilder::new::<T>(path, feat));
        self.builders.push(builder);
        self
    }

    /// Builds the [`ModuleBuilder`].
    pub fn build(
        mut self,
        path: &Path,
        features: &[fimo_ffi::String],
    ) -> ObjBox<DynObj<dyn IModuleBuilder>> {
        let path = path.to_path_buf();
        self.features.retain(|f| features.iter().any(|x| &**x == f));
        let enabled_features = self.features.into_iter().collect::<Vec<_>>();

        let mut builders = vec![];
        for builder in self.builders {
            builders.push(builder(&path, &enabled_features));
        }

        let mut interfaces = vec![];
        for builder in &builders {
            let builder = fimo_ffi::ptr::coerce_obj::<_, dyn IInterfaceBuilder>(builder);

            // Safety: We can extend the lifetime because we know that `builders` will outlive `interfaces`.
            unsafe {
                interfaces.push(&*(builder as *const _));
            }
        }

        let features = enabled_features.into_iter().map(Into::into).collect();

        let builder = ModuleBuilder {
            path,
            info: self.info,
            features,
            interfaces,
            _builders: builders,
        };
        let builder = ObjBox::new(builder);
        ObjBox::coerce_obj(builder)
    }
}

impl Debug for ModuleBuilderBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleBuilderBuilder")
            .field("info", &self.info)
            .field("features", &self.features)
            .finish_non_exhaustive()
    }
}

#[derive(Object)]
#[interfaces(IModuleBuilder)]
struct ModuleBuilder {
    path: PathBuf,
    info: ModuleInfo,
    features: Vec<fimo_ffi::String>,
    interfaces: Vec<&'static DynObj<dyn IInterfaceBuilder>>,
    _builders: Vec<InterfaceBuilder>,
}

impl IModuleBuilder for ModuleBuilder {
    fn module_path(&self) -> &std::path::Path {
        &self.path
    }

    fn module_info(&self) -> &ModuleInfo {
        &self.info
    }

    fn features(&self) -> &[fimo_ffi::String] {
        &self.features
    }

    fn interfaces(&self) -> &[&fimo_ffi::DynObj<dyn IInterfaceBuilder>] {
        &self.interfaces
    }
}

impl Debug for ModuleBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleBuilder")
            .field("path", &self.path)
            .field("info", &self.info)
            .field("features", &self.features)
            .finish_non_exhaustive()
    }
}

#[derive(Object, PartialEq, PartialOrd, Eq, Ord)]
#[interfaces(IInterfaceBuilder)]
struct InterfaceBuilder {
    path: PathBuf,
    name: &'static str,
    version: Version,
    extensions: Vec<fimo_ffi::String>,
    dependencies: Vec<InterfaceQuery>,
    optional_dependencies: Vec<InterfaceQuery>,
    #[allow(clippy::type_complexity)]
    build: fn(&Path, InterfaceContext) -> crate::Result<ObjBox<DynObj<dyn IInterface>>>,
}

impl InterfaceBuilder {
    fn new<T>(path: &Path, features: &[String]) -> Self
    where
        T: Interface,
    {
        let mut extensions = HashSet::new();
        let mut dependencies = HashSet::new();
        let mut optional_dependencies = HashSet::new();

        extensions.extend(<T as Interface>::extensions(None));
        dependencies.extend(<T as Interface>::dependencies(None));
        optional_dependencies.extend(<T as Interface>::optional_dependencies(None));
        for feature in features {
            extensions.extend(<T as Interface>::extensions(Some(feature)));
            dependencies.extend(<T as Interface>::dependencies(Some(feature)));
            optional_dependencies.extend(<T as Interface>::optional_dependencies(Some(feature)));
        }

        let extensions = extensions.into_iter().map(fimo_ffi::String::from).collect();
        let dependencies = dependencies.into_iter().collect();
        let optional_dependencies = optional_dependencies.into_iter().collect();

        fn construct<T>(
            module_root: &Path,
            context: InterfaceContext,
        ) -> crate::Result<ObjBox<DynObj<dyn IInterface>>>
        where
            T: Interface,
        {
            let x = T::construct(module_root, context)?;
            let x = ObjBox::coerce_obj(x);
            crate::Result::Ok(x)
        }

        Self {
            path: path.to_path_buf(),
            name: T::NAME,
            version: T::VERSION,
            extensions,
            dependencies,
            optional_dependencies,
            build: construct::<T>,
        }
    }
}

impl IInterfaceBuilder for InterfaceBuilder {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> fimo_ffi::Version {
        self.version
    }

    fn extensions(&self) -> &[fimo_ffi::String] {
        &self.extensions
    }

    fn dependencies(&self) -> &[InterfaceQuery] {
        &self.dependencies
    }

    fn optional_dependencies(&self) -> &[InterfaceQuery] {
        &self.optional_dependencies
    }

    fn build<'a>(
        &self,
        context: InterfaceContext,
    ) -> crate::Result<ObjBox<DynObj<dyn IInterface>>> {
        (self.build)(&self.path, context)
    }
}

impl Debug for InterfaceBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterfaceBuilder")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("extensions", &self.extensions)
            .field("dependencies", &self.dependencies)
            .field("optional_dependencies", &self.optional_dependencies)
            .finish_non_exhaustive()
    }
}

/// Helper trait for constructing an interface.
pub trait Interface:
    IInterface + FetchVTable<<dyn IInterface as ObjInterface>::Base> + Unsize<dyn IInterface> + 'static
{
    /// Name of the interface.
    const NAME: &'static str;
    /// Implemented version.
    const VERSION: Version;

    /// Extensions implemented with each feature.
    fn extensions(feature: Option<&str>) -> Vec<String>;

    /// Dependencies required by each feature.
    fn dependencies(feature: Option<&str>) -> Vec<InterfaceQuery>;

    /// Optional dependencies consumable by each feature.
    fn optional_dependencies(feature: Option<&str>) -> Vec<InterfaceQuery>;

    /// Constructs the interface.
    fn construct(module_root: &Path, context: InterfaceContext) -> crate::Result<ObjBox<Self>>;
}
