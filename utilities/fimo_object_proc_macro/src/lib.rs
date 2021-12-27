use proc_macro::TokenStream;
use quote::quote;
use std::default::Default;
use syn::parse::Parser;
use syn::{parse_macro_input, ItemStruct, LitStr, Type};

#[proc_macro_attribute]
pub fn fimo_vtable(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_struct = parse_macro_input!(input as ItemStruct);
    let interface_identifier = parse_macro_input!(args as LitStr);

    let ident = &item_struct.ident;

    let repr_attr = syn::Attribute::parse_outer
        .parse2(quote! { #[repr(C)] })
        .unwrap();
    let markers_attr = syn::Attribute::parse_outer
        .parse2(quote! { #[fimo_vtable_markers] })
        .unwrap()
        .first()
        .unwrap()
        .clone();

    // require that the `#[repr(C)]` attribute is present.
    if !item_struct
        .attrs
        .iter()
        .any(|attr| attr == repr_attr.first().unwrap())
    {
        let err = syn::Error::new(
            item_struct.ident.span(),
            "the `#repr[C]` attribute must be added to the struct definition",
        )
        .to_compile_error();

        return quote! {
            #item_struct
            #err
        }
        .into();
    }

    let markers = match item_struct
        .attrs
        .iter()
        .position(|attr| attr.path == markers_attr.path)
    {
        None => syn::parse_quote!(fimo_object::vtable::DefaultMarker),
        Some(idx) => {
            let attr = item_struct.attrs.remove(idx);
            match attr.parse_args::<Type>() {
                Err(e) => {
                    let err = e.to_compile_error();
                    return quote! {
                        #item_struct
                        #err
                    }
                    .into();
                }
                Ok(markers) => markers,
            }
        }
    };

    // Convert unit structs to named structs.
    if matches!(item_struct.fields, syn::Fields::Unit) {
        item_struct.fields = syn::Fields::Named(syn::FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        })
    }

    let trait_impl = match item_struct.fields {
        syn::Fields::Named(ref mut fields) => {
            add_named(fields, ident, interface_identifier, markers)
        }
        syn::Fields::Unnamed(ref mut fields) => {
            add_unnamed(fields, ident, interface_identifier, markers)
        }
        _ => {
            panic!();
        }
    };

    quote! {
        #item_struct
        #trait_impl
    }
    .into()
}

fn add_named(
    fields: &mut syn::FieldsNamed,
    ident: &proc_macro2::Ident,
    inter_ident: LitStr,
    markers: Type,
) -> proc_macro2::TokenStream {
    fields.named.insert(
        0,
        syn::Field::parse_named
            .parse2(quote! {
                /// Dropping procedure for the object.
                ///
                /// Consumes the pointer.
                pub __internal_drop_in_place: unsafe extern "C" fn(*mut ())
            })
            .unwrap(),
    );
    fields.named.insert(
        1,
        syn::Field::parse_named
            .parse2(quote! {
                /// Size of the object.
                pub __internal_object_size: usize
            })
            .unwrap(),
    );
    fields.named.insert(
        2,
        syn::Field::parse_named
            .parse2(quote! {
                /// Alignment of the object.
                pub __internal_object_alignment: usize
            })
            .unwrap(),
    );
    fields.named.insert(
        3,
        syn::Field::parse_named
            .parse2(quote! {
                /// Unique id of the object type.
                pub __internal_object_id: fimo_object::ConstStr<'static>
            })
            .unwrap(),
    );
    fields.named.insert(
        4,
        syn::Field::parse_named
            .parse2(quote! {
                /// Unique id of the interface type.
                pub __internal_interface_id: fimo_object::ConstStr<'static>
            })
            .unwrap(),
    );

    quote! {
        impl fimo_object::vtable::VTable for #ident {
            type Markers = #markers;
            const INTERFACE_ID: &'static str = #inter_ident;

            unsafe fn drop_in_place(&self, obj: *mut ()) {
                (self.__internal_drop_in_place)(obj)
            }

            fn size_of(&self) -> usize {
                self.__internal_object_size
            }

            fn align_of(&self) -> usize {
                self.__internal_object_alignment
            }

            fn object_id(&self) -> fimo_object::ConstStr<'static> {
                self.__internal_object_id
            }

            fn interface_id(&self) -> fimo_object::ConstStr<'static> {
                self.__internal_interface_id
            }
        }
    }
}

fn add_unnamed(
    fields: &mut syn::FieldsUnnamed,
    ident: &proc_macro2::Ident,
    inter_ident: LitStr,
    markers: Type,
) -> proc_macro2::TokenStream {
    fields.unnamed.insert(
        0,
        syn::Field::parse_unnamed
            .parse2(quote! {
                /// Dropping procedure for the object.
                ///
                /// Consumes the pointer.
                pub unsafe extern "C" fn(*mut ())
            })
            .unwrap(),
    );
    fields.unnamed.insert(
        1,
        syn::Field::parse_unnamed
            .parse2(quote! {
                /// Size of the object.
                pub usize
            })
            .unwrap(),
    );
    fields.unnamed.insert(
        2,
        syn::Field::parse_unnamed
            .parse2(quote! {
                /// Alignment of the object.
                pub usize
            })
            .unwrap(),
    );
    fields.unnamed.insert(
        3,
        syn::Field::parse_unnamed
            .parse2(quote! {
                /// Unique id of the object type.
                pub fimo_object::ConstStr<'static>
            })
            .unwrap(),
    );
    fields.unnamed.insert(
        4,
        syn::Field::parse_unnamed
            .parse2(quote! {
                /// Unique id of the interface type.
                pub fimo_object::ConstStr<'static>
            })
            .unwrap(),
    );

    quote! {
        impl fimo_object::vtable::VTable for #ident {
            type Markers = #markers;
            const INTERFACE_ID: &'static str = #inter_ident;

            unsafe fn drop_in_place(&self, obj: *mut ()) {
                (self.0)(obj)
            }

            fn size_of(&self) -> usize {
                self.1
            }

            fn align_of(&self) -> usize {
                self.2
            }

            fn object_id(&self) -> fimo_object::ConstStr<'static> {
                self.3
            }

            fn interface_id(&self) -> fimo_object::ConstStr<'static> {
                self.4
            }
        }
    }
}
