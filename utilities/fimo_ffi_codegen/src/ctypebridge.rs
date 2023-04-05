use proc_macro::TokenStream;
use quote::quote;

pub fn bridge_impl(input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::Item);

    let (ident, generics) = match item {
        syn::Item::Enum(x) => (x.ident, x.generics),
        syn::Item::Struct(x) => (x.ident, x.generics),
        syn::Item::Type(x) => (x.ident, x.generics),
        syn::Item::Union(x) => (x.ident, x.generics),
        x => {
            return syn::Error::new_spanned(x, "Item type not supported")
                .into_compile_error()
                .into()
        }
    };

    let (impl_gen, ty_gen, where_cl) = generics.split_for_impl();

    quote! {
        unsafe impl #impl_gen ::fimo_ffi::marshal::CTypeBridge for #ident #ty_gen #where_cl {
            type Type = Self;

            #[inline(always)]
            fn marshal(self) -> Self::Type {
                self
            }

            #[inline(always)]
            unsafe fn demarshal(x: Self::Type) -> Self {
                x
            }
        }
    }
    .into()
}
