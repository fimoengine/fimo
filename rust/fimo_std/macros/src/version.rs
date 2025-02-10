use proc_macro2::TokenStream;
use quote::quote;
use semver::Version;
use syn::{Error, LitStr, parse_macro_input};

pub fn version_impl(item: TokenStream) -> proc_macro::TokenStream {
    let item = item.into();
    let lit = parse_macro_input!(item as LitStr);

    let version = match Version::parse(&lit.value()) {
        Ok(v) => v,
        Err(err) => {
            return Error::new_spanned(lit, err).to_compile_error().into();
        }
    };

    let major = version.major;
    let minor = version.minor;
    let patch = version.patch;
    let pre = if version.pre.is_empty() {
        quote!(None)
    } else {
        let pre = version.pre.as_str();
        quote!(Some(#pre))
    };
    let build = if version.build.is_empty() {
        quote!(None)
    } else {
        let build = version.build.as_str();
        quote!(Some(#build))
    };

    let item = quote! {
        ::fimo_std::version::Version::<'static>::__private_new(#major, #minor, #patch, #pre, #build)
    };
    item.into()
}
