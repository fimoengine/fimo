#![allow(dead_code, clippy::todo)]

use crate::module_export::TasksModule;
use fimo_std::{
    error::Error,
    module::{DynamicExport, PartialModule, SymbolItem},
};

#[derive(Debug)]
pub struct ContextImpl {}

impl DynamicExport<TasksModule<'_>> for ContextImpl {
    type Item = fimo_tasks::symbols::fimo_tasks::Context;

    fn construct<'a>(
        _module: PartialModule<'a, TasksModule<'_>>,
    ) -> Result<&'a mut <Self::Item as SymbolItem>::Type, Error> {
        todo!()
    }

    fn destroy(_symbol: &mut <Self::Item as SymbolItem>::Type) {
        todo!()
    }
}
