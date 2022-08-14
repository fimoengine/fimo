//! Exports the `fimo_core` module.

use crate::Core;
use fimo_core_int::modules::IModuleRegistry;
use fimo_core_int::settings::ISettingsRegistry;
use fimo_core_int::IFimoCore;
use fimo_ffi::ptr::{IBase, IBaseExt};
use fimo_ffi::{DynObj, ObjArc, ObjectId, Version};
use fimo_module::{
    FimoInterface, IModule, IModuleInstance, IModuleInterface, IModuleLoader, ModuleInfo,
};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::Path;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

/// Struct implementing the `fimo-core` interface.
#[derive(ObjectId)]
#[fetch_vtable(
    uuid = "8e68e497-4dd1-481c-afe2-db7c063ae9f4",
    interfaces(IModuleInterface, IFimoCore)
)]
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
