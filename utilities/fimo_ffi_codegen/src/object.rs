use proc_macro::TokenStream;
use quote::quote;
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, DeriveInput};
use uuid::Uuid;

pub(crate) fn uuid_from_string(uuid: String) -> Option<darling::Result<Uuid>> {
    match Uuid::from_str(uuid.as_str()) {
        Ok(v) => Some(Ok(v)),
        Err(e) => Some(Err(darling::Error::custom(format!("uuid {}", e)))),
    }
}

#[allow(unused)]
struct InterfaceListAttribute {
    paren_token: syn::token::Paren,
    interfaces: syn::punctuated::Punctuated<syn::Path, syn::Token!(,)>,
}

impl InterfaceListAttribute {
    fn from_derive_input(input: &syn::DeriveInput) -> syn::Result<Option<Self>> {
        let parser = |input: syn::parse::ParseStream| {
            let content;
            let paren_token = syn::parenthesized!(content in input);
            let interfaces = content.call(syn::punctuated::Punctuated::parse_terminated)?;
            Ok((paren_token, interfaces))
        };

        for attr in &input.attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let Some(ident) = attr.path.get_ident() {
                if ident == "interfaces" {
                    let (paren_token, interfaces) =
                        syn::parse::Parser::parse2(parser, attr.tokens.clone())?;

                    return Ok(Some(Self {
                        paren_token,
                        interfaces,
                    }));
                }
            }
        }

        Ok(None)
    }
}

pub fn object_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let interfaces = match InterfaceListAttribute::from_derive_input(&input) {
        Ok(x) => x.map(|x| x.interfaces).unwrap_or_default(),
        Err(e) => return e.to_compile_error().into(),
    };

    // Check that only generic lifetimes are used, if any. This is required because of
    // the need to assign a unique uuid to every type. To every lifetime we add the bound
    // `'inner` transforming the possible generic argument `<'a, 'b: 'a>` to
    // `<'a: 'inner, b: 'a + 'inner>`.
    let mut generics = input.generics;
    for generic in &mut generics.params {
        if matches!(generic, syn::GenericParam::Type(_)) {
            return syn::Error::new(generic.span(), "only generic lifetimes are supported")
                .to_compile_error()
                .into();
        }
    }

    // Generic parameters without any `'inner` lifetime.
    let generics_no_inner = generics.clone();

    // For the implementation we also need the generic arguments without any bounds, e.g.
    // `impl<'a: 'inner, 'b: 'a + 'inner> Trait for Type<'a, 'b>`.
    let mut boundless_generics = generics.clone();
    boundless_generics.where_clause = None;
    for generic in &mut boundless_generics.params {
        if let syn::GenericParam::Lifetime(bound) = generic {
            bound.bounds = syn::punctuated::Punctuated::new();
        } else {
            return syn::Error::new(generic.span(), "only generic lifetimes are supported")
                .to_compile_error()
                .into();
        }
    }

    // The vtable for each interface is stored inside a static variable which can't contain
    // any generic arguments. So we need to replace `<'a, 'b>` to `<'_, '_>`.
    let mut elided_generics = boundless_generics.clone();
    for generic in &mut elided_generics.params {
        if let syn::GenericParam::Lifetime(bound) = generic {
            bound.lifetime = parse_quote!('_);
        } else {
            return syn::Error::new(generic.span(), "only generic lifetimes are supported")
                .to_compile_error()
                .into();
        }
    }

    // Type<'a, 'b>
    let ident = input.ident;
    let ty: syn::Type = parse_quote!(#ident #boundless_generics);

    // Type<'_, '_>
    let elided_ty: syn::Type = parse_quote!(#ident #elided_generics);

    // `<'a: 'inner, 'b: 'a + 'inner>` -> `<'a: 'inner, 'b: 'a + 'inner, 'inner>`
    generics.params.push(parse_quote!('inner));

    let mut impls: Vec<_> = Vec::new();
    // Implement the `DowncastSafe` trait for the type.
    impls.push(quote! {
        unsafe impl #generics_no_inner ::fimo_ffi::ptr::DowncastSafe for #ty where #ty: 'static { }
    });

    // Implement the `FetchVTable<dyn IBase + '_>` trait for the generic type.
    impls.push(quote! {
        impl #generics ::fimo_ffi::ptr::FetchVTable<dyn ::fimo_ffi::ptr::IBase + 'inner> for #ty
        where #ty: 'inner
        {
            fn fetch_interface() -> &'static ::fimo_ffi::ptr::IBaseVTable {
                static VTABLE: ::fimo_ffi::ptr::IBaseVTable = ::fimo_ffi::ptr::IBaseVTable::new_for::<#elided_ty>();
                &VTABLE
            }
        }
    });

    // Implement the remaining `FetchVTable<dyn _ + '_>` traits for the generic type.
    for interface in interfaces.iter() {
        impls.push(quote! {
            impl #generics ::fimo_ffi::ptr::FetchVTable<dyn #interface + 'inner> for #ty
            where #ty: 'inner
            {
                fn fetch_interface() -> &'static <<(dyn #interface + 'inner) as ::fimo_ffi::ptr::ObjInterface>::Base as ::fimo_ffi::ptr::ObjInterfaceBase>::VTable {
                    static VTABLE: <<(dyn #interface) as ::fimo_ffi::ptr::ObjInterface>::Base as ::fimo_ffi::ptr::ObjInterfaceBase>::VTable
                        = <<(dyn #interface) as ::fimo_ffi::ptr::ObjInterface>::Base as ::fimo_ffi::ptr::ObjInterfaceBase>::VTable::new_for::<#elided_ty>();
                    &VTABLE
                }
            }
        });
    }

    let output = quote! { #(#impls)* };
    output.into()
}
