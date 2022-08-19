use proc_macro::TokenStream;
use quote::quote;

#[allow(unused)]
struct UUIDAttribute {
    paren_token: syn::token::Paren,
    uuid: syn::LitStr,
}

impl UUIDAttribute {
    fn from_derive_input(input: &syn::DeriveInput) -> syn::Result<Option<Self>> {
        let parser = |input: syn::parse::ParseStream| {
            let content;
            let paren_token = syn::parenthesized!(content in input);
            let uuid: syn::LitStr = content.parse()?;
            Ok((paren_token, uuid))
        };

        for attr in &input.attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let Some(ident) = attr.path.get_ident() {
                if ident == "uuid" {
                    let (paren_token, uuid) =
                        syn::parse::Parser::parse2(parser, attr.tokens.clone())?;

                    return Ok(Some(Self { paren_token, uuid }));
                }
            }
        }

        Ok(None)
    }
}

#[allow(unused)]
struct NameAttribute {
    paren_token: syn::token::Paren,
    name: syn::LitStr,
}

impl NameAttribute {
    fn from_derive_input(input: &syn::DeriveInput) -> syn::Result<Option<Self>> {
        let parser = |input: syn::parse::ParseStream| {
            let content;
            let paren_token = syn::parenthesized!(content in input);
            let name: syn::LitStr = content.parse()?;
            Ok((paren_token, name))
        };

        for attr in &input.attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let Some(ident) = attr.path.get_ident() {
                if ident == "name" {
                    let (paren_token, name) =
                        syn::parse::Parser::parse2(parser, attr.tokens.clone())?;

                    return Ok(Some(Self { paren_token, name }));
                }
            }
        }

        Ok(None)
    }
}

#[allow(unused)]
struct GenerationAttribute {
    paren_token: syn::token::Paren,
    generation: syn::LitInt,
}

impl GenerationAttribute {
    fn from_derive_input(input: &syn::DeriveInput) -> syn::Result<Option<Self>> {
        let parser = |input: syn::parse::ParseStream| {
            let content;
            let paren_token = syn::parenthesized!(content in input);
            let generation: syn::LitInt = content.parse()?;
            generation.base10_parse::<u32>()?;
            Ok((paren_token, generation))
        };

        for attr in &input.attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let Some(ident) = attr.path.get_ident() {
                if ident == "generation" {
                    let (paren_token, generation) =
                        syn::parse::Parser::parse2(parser, attr.tokens.clone())?;

                    return Ok(Some(Self {
                        paren_token,
                        generation,
                    }));
                }
            }
        }

        Ok(None)
    }
}

struct IgnoredAttribute;

impl IgnoredAttribute {
    fn from_attributes(attrs: &[syn::Attribute]) -> Option<Self> {
        for attr in attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let Some(ident) = attr.path.get_ident() {
                if ident == "ignored" {
                    return Some(Self);
                }
            }
        }

        None
    }
}

pub fn stable_id(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let uuid = match UUIDAttribute::from_derive_input(&input) {
        Ok(x) => x,
        Err(e) => return e.to_compile_error().into(),
    };

    let name = match NameAttribute::from_derive_input(&input) {
        Ok(x) => x,
        Err(e) => return e.to_compile_error().into(),
    };

    let generation = match GenerationAttribute::from_derive_input(&input) {
        Ok(x) => x,
        Err(e) => return e.to_compile_error().into(),
    };

    let uuid = match uuid {
        Some(uuid) => uuid.uuid,
        None => return quote!(std::compile_error!("Expected #[uuid] attribute")).into(),
    };

    let name = name.map(|n| {
        let name = n.name;
        quote!(const NAME: &'static str = #name;)
    });

    let generation = generation
        .map(|gen| gen.generation.base10_parse::<u32>().unwrap())
        .unwrap_or(0);

    let variant: u32;
    let mut members = Vec::new();
    if let syn::Data::Struct(data) = &input.data {
        match &data.fields {
            syn::Fields::Named(x) => {
                variant = 0;
                for f in &x.named {
                    if IgnoredAttribute::from_attributes(&f.attrs).is_none() {
                        members.push(f.ty.clone())
                    }
                }
            }
            syn::Fields::Unnamed(x) => {
                variant = 1;
                for f in &x.unnamed {
                    if IgnoredAttribute::from_attributes(&f.attrs).is_none() {
                        members.push(f.ty.clone())
                    }
                }
            }
            syn::Fields::Unit => variant = 2,
        };
    } else {
        return quote!(std::compile_error!("Expected struct")).into();
    }

    let variant = ((variant as usize) << 32) | (generation as usize);

    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();
    where_clause
        .predicates
        .push(syn::parse_quote!(Self: 'static));

    let ident = input.ident;
    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_gen ::fimo_ffi::type_id::TypeInfo for #ident #ty_gen #where_clause {
            const ID: ::fimo_ffi::ptr::Uuid = ::fimo_ffi::ptr::uuid!(#uuid);
            const VARIANT: usize = #variant;
            #name
            const MEMBER_IDS: &'static [::fimo_ffi::type_id::StableTypeId] = &[
                #(::fimo_ffi::type_id::StableTypeId::of::<#members>()),*
            ];
        }
    }
    .into()
}
