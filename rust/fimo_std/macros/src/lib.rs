use proc_macro::TokenStream;

mod export_module;
mod version;

#[proc_macro_attribute]
pub fn export_module(args: TokenStream, item: TokenStream) -> TokenStream {
    export_module::export_module_impl(args.into(), item.into())
}

/// Parses a version string.
#[proc_macro]
pub fn version(item: TokenStream) -> TokenStream {
    version::version_impl(item.into())
}
