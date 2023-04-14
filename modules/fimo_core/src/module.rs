//! Exports the `fimo_core` module.

use crate::Core;
use fimo_core_int::settings::ISettingsRegistry;
use fimo_core_int::IFimoCore;
use fimo_ffi::provider::IProvider;
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, ObjBox, Object, Version};
use fimo_module::context::{IInterface, IInterfaceContext};
use fimo_module::module::{Interface, ModuleBuilderBuilder};
use fimo_module::QueryBuilder;
use std::fmt::{Debug, Formatter};
use std::path::Path;

/// Struct implementing the `fimo-core` interface.
#[derive(Object, StableTypeId)]
#[name("CoreInterface")]
#[uuid("d8ea3d1e-3286-4c4b-ac51-12dca7daa624")]
#[interfaces(IInterface, IFimoCore)]
pub struct CoreInterface<'a> {
    core: Core,
    _context: &'a DynObj<dyn IInterfaceContext + 'a>,
}

impl Debug for CoreInterface<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreInterface_")
            .field("core", &self.core)
            .field("_context", &(self._context as *const _))
            .finish()
    }
}

impl IProvider for CoreInterface<'_> {
    fn provide<'a>(&'a self, demand: &mut fimo_ffi::provider::Demand<'a>) {
        demand.provide_obj::<dyn IFimoCore + 'a>(fimo_ffi::ptr::coerce_obj(self));
    }
}

impl IInterface for CoreInterface<'_> {
    fn name(&self) -> &str {
        CoreInterface::NAME
    }

    fn version(&self) -> Version {
        CoreInterface::VERSION
    }

    fn extensions(&self) -> &[fimo_ffi::String] {
        &[]
    }
}

impl IFimoCore for CoreInterface<'_> {
    #[inline]
    fn settings(&self) -> &DynObj<dyn ISettingsRegistry + '_> {
        fimo_ffi::ptr::coerce_obj(self.core.as_settings_registry())
    }
}

impl Interface for CoreInterface<'_> {
    type Result<'a> = CoreInterface<'a>;
    const NAME: &'static str = QueryBuilder.name::<dyn IFimoCore>();
    const VERSION: Version = Version::new_short(0, 1, 0);

    fn extensions(_feature: Option<&str>) -> Vec<String> {
        vec![]
    }

    fn dependencies(_feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        vec![]
    }

    fn optional_dependencies(_feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        vec![]
    }

    fn construct<'a>(
        _module_root: &Path,
        context: &'a DynObj<dyn IInterfaceContext + 'a>,
    ) -> fimo_module::Result<ObjBox<Self::Result<'a>>> {
        Ok(ObjBox::new(CoreInterface {
            core: Default::default(),
            _context: context,
        }))
    }
}

fimo_module::module!(|path, features| {
    Ok(
        ModuleBuilderBuilder::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .with_interface::<CoreInterface<'_>>()
            .build(path, features),
    )
});
