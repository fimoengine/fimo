//! Exports the `fimo_core` module.

use crate::Logger;
use fimo_ffi::ptr::{IBase, IBaseExt};
use fimo_ffi::{DynObj, ObjArc, ObjectId, Version};
use fimo_logging_int::{IFimoLogging, ILogger};
use fimo_module::{
    FimoInterface, IModule, IModuleInstance, IModuleInterface, IModuleLoader, ModuleInfo,
};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::Path;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_logging";

/// Struct implementing the `fimo-logging` interface.
// TODO: Change uuid.
#[derive(ObjectId)]
#[fetch_vtable(
    uuid = "8e68e497-4dd1-481c-afe2-db7c063ae9f4",
    interfaces(IModuleInterface, IFimoLogging)
)]
pub struct LoggingInterface {
    logger: Logger,
    parent: ObjArc<DynObj<dyn IModuleInstance>>,
}

impl Debug for LoggingInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(LoggingInterface)")
    }
}

impl Deref for LoggingInterface {
    type Target = Logger;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.logger
    }
}

impl DerefMut for LoggingInterface {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.logger
    }
}

impl IModuleInterface for LoggingInterface {
    #[inline]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        fimo_ffi::ptr::coerce_obj::<_, dyn IFimoLogging + Send + Sync>(self).cast_super()
    }

    #[inline]
    fn name(&self) -> &str {
        <dyn IFimoLogging>::NAME
    }

    #[inline]
    fn version(&self) -> Version {
        <dyn IFimoLogging>::VERSION
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        <dyn IFimoLogging>::EXTENSIONS
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

impl IFimoLogging for LoggingInterface {
    fn logger(&self) -> &DynObj<dyn ILogger> {
        fimo_ffi::ptr::coerce_obj(&self.logger)
    }
}

fn module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: <dyn IFimoLogging>::VERSION.into(),
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
            .empty(<dyn IFimoLogging>::new_descriptor(), |instance| {
                let logging = LoggingInterface {
                    logger: Default::default(),
                    parent: ObjArc::coerce_obj(instance),
                };
                let logging = ObjArc::new(logging);
                Ok(ObjArc::coerce_obj(logging))
            })
            .build();
        Ok(instance)
    });
    Ok(ObjArc::coerce_obj(module))
}
