//! Exports the `fimo_core` module.

use crate::Core;
use fimo_core_int::modules::IModuleRegistry;
use fimo_core_int::settings::ISettingsRegistry;
use fimo_core_int::{IFimoCore, _IFimoCore};
use fimo_ffi::provider::IProvider;
use fimo_ffi::ptr::{IBase, IBaseExt};
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, ObjArc, ObjBox, Object, ReleaseType, Version};
use fimo_module::context::{IInterface, InterfaceContext};
use fimo_module::module_::{Interface, ModuleBuilderBuilder};
use fimo_module::{
    FimoInterface, IModule, IModuleInstance, IModuleInterface, IModuleLoader, ModuleInfo,
};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::Path;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

/// Struct implementing the `fimo-core` interface.
#[derive(Object, StableTypeId)]
#[name("CoreInterface")]
#[uuid("8e68e497-4dd1-481c-afe2-db7c063ae9f4")]
#[interfaces(IModuleInterface, IFimoCore)]
pub struct CoreInterface {
    core: Core,
    parent: ObjArc<DynObj<dyn IModuleInstance>>,
}

impl Debug for CoreInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(CoreInterface)")
    }
}

impl Deref for CoreInterface {
    type Target = Core;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl DerefMut for CoreInterface {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

impl IModuleInterface for CoreInterface {
    #[inline]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        fimo_ffi::ptr::coerce_obj::<_, dyn IFimoCore + Send + Sync>(self).cast_super()
    }

    #[inline]
    fn name(&self) -> &str {
        <dyn IFimoCore as FimoInterface>::NAME
    }

    #[inline]
    fn version(&self) -> Version {
        <dyn IFimoCore as FimoInterface>::VERSION
    }

    #[inline]
    fn extensions(&self) -> fimo_ffi::Vec<fimo_ffi::String> {
        <dyn IFimoCore as FimoInterface>::EXTENSIONS
            .iter()
            .map(|&s| s.into())
            .collect()
    }

    #[inline]
    fn extension(&self, _name: &str) -> Option<&DynObj<dyn IBase + Send + Sync>> {
        None
    }

    #[inline]
    fn instance(&self) -> ObjArc<DynObj<dyn IModuleInstance>> {
        self.parent.clone()
    }
}

impl IFimoCore for CoreInterface {
    #[inline]
    fn modules(&self) -> &DynObj<dyn IModuleRegistry + '_> {
        fimo_ffi::ptr::coerce_obj(self.core.as_module_registry())
    }

    #[inline]
    fn settings(&self) -> &DynObj<dyn ISettingsRegistry + '_> {
        fimo_ffi::ptr::coerce_obj(self.core.as_settings_registry())
    }
}

fn module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: <dyn IFimoCore>::VERSION.into(),
    }
}

fimo_module::rust_module!(load_module);

fn load_module(
    loader: &'static DynObj<dyn IModuleLoader>,
    path: &Path,
) -> fimo_module::Result<ObjArc<DynObj<dyn IModule>>> {
    let module = fimo_module::module::Module::new(module_info(), path, loader, |module| {
        let builder = fimo_module::module::InstanceBuilder::new(module);
        let instance = builder
            .empty(<dyn IFimoCore>::new_descriptor(), |instance| {
                let core = CoreInterface {
                    core: Default::default(),
                    parent: ObjArc::coerce_obj(instance),
                };
                let core = ObjArc::new(core);
                Ok(ObjArc::coerce_obj(core))
            })
            .build();
        Ok(instance)
    });
    Ok(ObjArc::coerce_obj(module))
}

/// Struct implementing the `fimo-core` interface.
#[derive(Debug, Object, StableTypeId)]
#[name("CoreInterface")]
#[uuid("d8ea3d1e-3286-4c4b-ac51-12dca7daa624")]
#[interfaces(IInterface, _IFimoCore)]
pub struct CoreInterface_ {
    core: Core,
    _context: InterfaceContext,
}

impl IProvider for CoreInterface_ {
    fn provide<'a>(&'a self, demand: &mut fimo_ffi::provider::Demand<'a>) {
        demand.provide_ref::<DynObj<dyn _IFimoCore>>(fimo_ffi::ptr::coerce_obj(self));
    }
}

impl IInterface for CoreInterface_ {
    fn name(&self) -> &str {
        CoreInterface_::NAME
    }

    fn version(&self) -> Version {
        CoreInterface_::VERSION
    }

    fn extensions(&self) -> &[fimo_ffi::String] {
        &[]
    }
}

impl _IFimoCore for CoreInterface_ {
    #[inline]
    fn modules(&self) -> &DynObj<dyn IModuleRegistry + '_> {
        fimo_ffi::ptr::coerce_obj(self.core.as_module_registry())
    }

    #[inline]
    fn settings(&self) -> &DynObj<dyn ISettingsRegistry + '_> {
        fimo_ffi::ptr::coerce_obj(self.core.as_settings_registry())
    }
}

impl Interface for CoreInterface_ {
    const NAME: &'static str = "fimo::interfaces::core";

    const VERSION: Version = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);

    fn extensions(_feature: Option<&str>) -> Vec<String> {
        vec![]
    }

    fn dependencies(_feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        vec![]
    }

    fn optional_dependencies(_feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        vec![]
    }

    fn construct(
        _module_root: &Path,
        context: InterfaceContext,
    ) -> fimo_module::Result<ObjBox<Self>> {
        Ok(ObjBox::new(Self {
            core: Default::default(),
            _context: context,
        }))
    }
}

fimo_module::module!(|path, features| {
    Ok(ModuleBuilderBuilder::new(MODULE_NAME, "version")
        .with_interface::<CoreInterface_>()
        .build(path, features))
});
