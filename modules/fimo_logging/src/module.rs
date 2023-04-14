//! Exports the `fimo_core` module.

use crate::Logger;
use fimo_ffi::provider::IProvider;
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, ObjBox, Object, Version};
use fimo_logging_int::{IFimoLogging, ILogger};
use fimo_module::context::{IInterface, IInterfaceContext};
use fimo_module::module::{Interface, ModuleBuilderBuilder};
use fimo_module::QueryBuilder;
use std::fmt::{Debug, Formatter};
use std::path::Path;

/// Struct implementing the `fimo-logging` interface.
#[derive(Object, StableTypeId)]
#[name("LoggingInterface")]
#[uuid("85cbdb52-3ffb-4dff-b893-e90bfb1e6ac1")]
#[interfaces(IInterface, IFimoLogging)]
pub struct LoggingInterface<'a> {
    logger: Logger,
    _context: &'a DynObj<dyn IInterfaceContext + 'a>,
}

impl Debug for LoggingInterface<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingInterface")
            .field("logger", &self.logger)
            .field("_context", &(self._context as *const _))
            .finish()
    }
}

impl IProvider for LoggingInterface<'_> {
    fn provide<'a>(&'a self, demand: &mut fimo_ffi::provider::Demand<'a>) {
        demand.provide_obj::<dyn IFimoLogging + 'a>(fimo_ffi::ptr::coerce_obj(self));
    }
}

impl IInterface for LoggingInterface<'_> {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> Version {
        Self::VERSION
    }

    fn extensions(&self) -> &[fimo_ffi::String] {
        &[]
    }
}

impl IFimoLogging for LoggingInterface<'_> {
    fn logger(&self) -> &DynObj<dyn ILogger> {
        fimo_ffi::ptr::coerce_obj(&self.logger)
    }
}

impl Interface for LoggingInterface<'_> {
    type Result<'a> = LoggingInterface<'a>;
    const NAME: &'static str = QueryBuilder.name::<dyn IFimoLogging>();
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
        Ok(ObjBox::new(LoggingInterface {
            logger: Default::default(),
            _context: context,
        }))
    }
}

fimo_module::module!(|path, features| {
    Ok(
        ModuleBuilderBuilder::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .with_interface::<LoggingInterface<'_>>()
            .build(path, features),
    )
});
