use proc_macro::TokenStream;

mod export_module;

#[proc_macro_attribute]
pub fn export_module(args: TokenStream, item: TokenStream) -> TokenStream {
    export_module::export_module_impl(args.into(), item.into())
}
