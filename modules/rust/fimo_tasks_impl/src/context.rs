use crate::module_export::Module;
use fimo_std::{
    error::Error,
    module::{DynamicExport, PartialModule, SymbolItem},
};

#[derive(Debug)]
pub struct ContextImpl {}

impl DynamicExport<Module<'_>> for ContextImpl {
    type Item = fimo_tasks::symbols::fimo_tasks::Context;

    fn construct<'a>(
        module: PartialModule<'a, Module<'_>>,
    ) -> Result<&'a mut <Self::Item as SymbolItem>::Type, Error> {
        todo!()
    }

    fn destroy(symbol: &mut <Self::Item as SymbolItem>::Type) {
        todo!()
    }
}
