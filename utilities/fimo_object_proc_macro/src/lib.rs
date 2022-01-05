use proc_macro::TokenStream;
use quote::quote;
use std::default::Default;
use syn::parse::{ParseStream, Parser};
use syn::{parse_macro_input, ItemStruct, Lit, LitStr, Type};

struct FimoVTableAttr {
    pub name: LitStr,
    pub marker: Type,
}

impl syn::parse::Parse for FimoVTableAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let id_path: syn::Path = syn::parse_quote!(id);
        let marker_path: syn::Path = syn::parse_quote!(marker);

        let metas = input.call(
            syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated,
        )?;
        let name_idx = match metas.iter().position(|v| v.path == id_path) {
            None => return Err(syn::Error::new(input.span(), "key `id` not found")),
            Some(i) => i,
        };
        let marker_idx = metas.iter().position(|v| v.path == marker_path);

        let name_meta = &metas[name_idx];
        let name = match &name_meta.lit {
            Lit::Str(s) => s.clone(),
            l => {
                return Err(syn::Error::new(l.span(), "expected a string literal"));
            }
        };

        let marker = match marker_idx {
            None => {
                syn::parse_quote!(fimo_object::vtable::DefaultMarker)
            }
            Some(marker_idx) => {
                let marker_meta = &metas[marker_idx];
                match &marker_meta.lit {
                    Lit::Str(l) => syn::parse_str(l.value().as_str())?,
                    l => {
                        return Err(syn::Error::new(l.span(), "expected a string literal"));
                    }
                }
            }
        };

        Ok(Self { name, marker })
    }
}

/// Makes a struct usable as a vtable.
///
/// The `fimo_vtable` attributes adds a set of predefined fields
/// to the struct, to make it's layout compatible with a fimo-object vtable.
/// Furthermore it implements the `VTable` trait for the given struct.
/// The attribute items are specified using the
/// [MetaListNameValueStr](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax)
/// syntax. The 'id' key is the unique id of the interface. The 'marker' key is an optional value
/// which specifies a path to a marker type to use when implementing the `VTable` trait.
///
/// # Struct Layout
///
/// The struct must be laid out according to the system`s C ABI, which can be achieved using
/// the `[#repr(C)]` attribute. VTables are only passed as a pointer or reference, as that allows
/// adding new fields to the end of the table, without breaking the ABI.
///
/// # Example
///
/// ```
/// use fimo_object_proc_macro::fimo_vtable;
///
/// // VTable with default Marker
/// #[repr(C)]
/// #[fimo_vtable(id = "default_marker_interface")]
/// #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
/// struct InterfaceDefMar;
///
/// // Marker that is `Send` and `Sync`
/// struct Marker;
///
/// #[repr(C)]
/// #[fimo_vtable(id = "custom_marker_interface", marker = "Marker")]
/// #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
/// struct InterfaceCusMar;
/// ```
#[proc_macro_attribute]
pub fn fimo_vtable(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_struct = parse_macro_input!(input as ItemStruct);
    let attr = parse_macro_input!(args as FimoVTableAttr);

    let ident = &item_struct.ident;

    let repr_attr = syn::Attribute::parse_outer
        .parse2(quote! { #[repr(C)] })
        .unwrap();

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

    // Convert unit structs to named structs.
    if matches!(item_struct.fields, syn::Fields::Unit) {
        item_struct.fields = syn::Fields::Named(syn::FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        })
    }

    let trait_impl = match item_struct.fields {
        syn::Fields::Named(ref mut fields) => add_named(fields, ident, attr.name, attr.marker),
        syn::Fields::Unnamed(ref mut fields) => add_unnamed(fields, ident, attr.name, attr.marker),
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
    use syn::parse::Parse;
    let mut vtable_fields = syn::FieldsNamed::parse
        .parse2(quote! {
            {
                /// Dropping procedure for the object.
                ///
                /// Consumes the pointer.
                pub __internal_drop_in_place: unsafe extern "C" fn(*mut ()),
                /// Size of the object.
                pub __internal_object_size: usize,
                /// Alignment of the object.
                pub __internal_object_alignment: usize,
                /// Unique id of the object type.
                pub __internal_object_id: fimo_object::ConstStr<'static>,
                /// Unique id of the interface type.
                pub __internal_interface_id: fimo_object::ConstStr<'static>
            }
        })
        .unwrap();

    vtable_fields.named.extend(fields.named.clone());
    fields.named = vtable_fields.named;

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
    use syn::parse::Parse;
    let mut vtable_fields = syn::FieldsUnnamed::parse
        .parse2(quote! {
            (
                /// Dropping procedure for the object.
                ///
                /// Consumes the pointer.
                pub unsafe extern "C" fn(*mut ()),
                /// Size of the object.
                pub usize,
                /// Alignment of the object.
                pub usize,
                /// Unique id of the object type.
                pub fimo_object::ConstStr<'static>,
                /// Unique id of the interface type.
                pub fimo_object::ConstStr<'static>
            )
        })
        .unwrap();

    vtable_fields.unnamed.extend(fields.unnamed.clone());
    fields.unnamed = vtable_fields.unnamed;

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
