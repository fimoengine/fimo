use darling::{FromField, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Add;
use syn::{parse_macro_input, AttributeArgs, Fields, ItemStruct, TraitBound, Type};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, FromMeta)]
enum VTableType {
    /// Not a vtable.
    None,
    /// Primary inner vtable of a vtable.
    Primary,
    /// Secondary inner vtable.
    Secondary,
}

impl Default for VTableType {
    fn default() -> Self {
        VTableType::None
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(super_vtable))]
struct SuperVTableField {
    // name of the field is always `Some(_)`
    ident: Option<syn::Ident>,
    // type of the field.
    ty: syn::Type,
    #[darling(default)]
    is: VTableType,
}

#[derive(Debug, FromMeta)]
struct VTableArgs {
    // single generic trait e.g. `Trait<'a, 'b>`
    interface: Punctuated<TraitBound, Add>,
    #[darling(default, map = "crate::interface::map_lifetime")]
    obj_bound: Option<syn::Result<syn::LifetimeDef>>,
}

pub fn vtable_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let mut input = parse_macro_input!(input as ItemStruct);

    let args = match VTableArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    // validate that the input to the macro is correct.
    let fields = match validate_input(&mut input, &args) {
        Ok(v) => v,
        Err(e) => {
            return e.into();
        }
    };

    let obj_bound = match args.obj_bound {
        None => syn::parse_quote!('inner),
        Some(Ok(bound)) => bound,
        Some(Err(e)) => return e.into_compile_error().into(),
    };
    let obj_bound_lifetime = &obj_bound.lifetime;

    // extract the fields and the `super_vtable` attributes.
    let field_names: Vec<syn::Ident> = fields
        .named
        .iter()
        .cloned()
        .map(|field| field.ident.unwrap())
        .collect();
    let field_types: Vec<syn::Type> = fields.named.iter().cloned().map(|field| field.ty).collect();
    let super_vtables: Vec<_> = fields
        .named
        .iter()
        .map(SuperVTableField::from_field)
        .map(Result::unwrap)
        .collect();

    // remove the `super_vtable` attribute from the fields
    let attr_path: syn::Path = syn::parse_quote!(super_vtable);
    for (idx, _) in super_vtables.iter().enumerate() {
        let field = &mut fields.named[idx];
        if let Some(idx) = field.attrs.iter().position(|attr| attr.path == attr_path) {
            field.attrs.remove(idx);
        }
    }

    // add a vtable head if there isn't one already.
    // if the first field is already a vtable we can use that one instead.
    let new_head_expr = if super_vtables.is_empty() || !is_head_vtable(&super_vtables[0]) {
        let mut field: syn::FieldsNamed = syn::parse_quote! {
            {
                /// Common head of the vtable.
                pub __internal_head: ::fimo_ffi::ptr::VTableHead,
            }
        };
        let field = field.named.pop().unwrap().into_value();
        fields.named.insert(0, field);

        quote! {
            __internal_head: ::fimo_ffi::ptr::VTableHead::new_embedded::<#obj_bound_lifetime, T, Dyn>(internal_offset),
        }
    } else {
        quote! {}
    };

    let struct_name = input.ident.clone();
    let interface = args.interface[0].clone();
    let interface_name = interface.path.clone();

    //let inner_lifetime = syn::Lifetime::new("'inner", Span::call_site());
    let lifetimes = interface
        .lifetimes
        .as_ref()
        .map(|l| l.lifetimes.clone())
        .unwrap_or_default();

    let mut vtable_casts: Vec<_> = Vec::new();
    // implement vtable upcasting.
    for super_vtable in super_vtables.iter() {
        if matches!(super_vtable.is, VTableType::Primary | VTableType::Secondary) {
            vtable_casts.push(inner_vtable_impl(super_vtable, &struct_name));
        }
    }

    // implement the upcasting to `IBaseVTable` if there are no primary vtables.
    if !super_vtables
        .iter()
        .any(|attr| attr.is == VTableType::Primary)
    {
        vtable_casts.push(base_upcast_impl(&struct_name));
    }

    let input = quote! {
        #input

        impl<#obj_bound, #lifetimes> #struct_name {
            /// Constructs a new instance of the vtable.
            #[allow(clippy::type_complexity)]
            #[allow(clippy::too_many_arguments)]
            pub const fn new<T>(#(#field_names: #field_types),*) -> Self
            where
                T: #interface_name + ::fimo_ffi::ptr::ObjectId + #obj_bound_lifetime
            {
                Self::new_embedded::<T, dyn #interface_name + #obj_bound_lifetime>(0, #(#field_names),*)
            }

            /// Constructs a new instance of the vtable when embedded into another vtable.
            #[allow(clippy::type_complexity)]
            #[allow(clippy::too_many_arguments)]
            pub const fn new_embedded<T, Dyn>(internal_offset: usize, #(#field_names: #field_types),*) -> Self
            where
                T: #interface_name + ::fimo_ffi::ptr::ObjectId + std::marker::Unsize<Dyn> + #obj_bound_lifetime,
                Dyn: ::fimo_ffi::ptr::ObjInterface + ?Sized + #obj_bound_lifetime
            {
                Self {
                    #new_head_expr
                    #(#field_names),*
                }
            }
        }

        unsafe impl ::fimo_ffi::ptr::ObjMetadataCompatible for #struct_name {}

        #(#vtable_casts)*
    };
    input.into()
}

fn validate_input<'a>(
    input: &'a mut ItemStruct,
    args: &VTableArgs,
) -> Result<&'a mut syn::FieldsNamed, proc_macro2::TokenStream> {
    if args.interface.len() != 1 {
        return Err(syn::Error::new(args.interface.span(), "expected one trait").to_compile_error());
    }

    if let Some(Err(e)) = &args.obj_bound {
        return Err(e.to_compile_error());
    }

    // add the `repr(C)` attribute to the type.
    let repr_attr: syn::Attribute = syn::parse_quote!(#[repr(C)]);
    input.attrs.push(repr_attr);

    // convert unit structs to empty structs.
    if input.fields == Fields::Unit {
        input.fields = Fields::Named(syn::parse_quote!({}));
    }

    // check that we are dealing with a struct with named fields.
    let fields = match input.fields {
        Fields::Named(ref mut fields) => fields,
        _ => {
            return Err(
                syn::Error::new(input.span(), "expected named fields or unit struct")
                    .to_compile_error(),
            );
        }
    };

    // check that the fields are compatible with a vtable.
    for field in &fields.named {
        let attr = match SuperVTableField::from_field(field) {
            Ok(v) => v,
            Err(e) => {
                return Err(e.write_errors());
            }
        };

        // we only allow embedded vtables by value.
        if attr.is != VTableType::None {
            match field.ty {
                Type::Path(_) => {}
                _ => {
                    return Err(
                        syn::Error::new(field.ty.span(), "expected struct").to_compile_error()
                    );
                }
            }
        }
    }

    Ok(fields)
}

fn is_head_vtable(field: &SuperVTableField) -> bool {
    matches!(field.is, VTableType::Primary | VTableType::Secondary)
        && matches!(field.ty, syn::Type::Path(_))
}

fn inner_vtable_impl(
    attr: &SuperVTableField,
    struct_name: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    let vtable_ty = attr.ty.clone();
    let vtable_name = attr.ident.clone().unwrap();

    // for primary vtables we allow transitive upcasting of vtables.
    if attr.is == VTableType::Primary {
        quote! {
            impl<T> ::fimo_ffi::ptr::InnerVTable<T> for #struct_name
            where
                #vtable_ty: ::fimo_ffi::ptr::InnerVTable<T>,
                T: ::fimo_ffi::ptr::ObjMetadataCompatible,
            {
                #[inline]
                fn inner(&self) -> &T {
                    <#vtable_ty as ::fimo_ffi::ptr::InnerVTable<T>>::inner(&self.#vtable_name)
                }
            }
        }
    } else {
        quote! {
            impl ::fimo_ffi::ptr::InnerVTable<#vtable_ty> for #struct_name {
                #[inline]
                fn inner(&self) -> &#vtable_ty {
                    &self.#vtable_name
                }
            }
        }
    }
}

fn base_upcast_impl(struct_name: &proc_macro2::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl ::fimo_ffi::ptr::InnerVTable<::fimo_ffi::ptr::IBaseVTable> for #struct_name {
            #[inline]
            fn inner(&self) -> &::fimo_ffi::ptr::IBaseVTable {
                // safety: the safety is guaranteed by the invariants of a `ObjMetadataCompatible`.
                unsafe { std::mem::transmute(self) }
            }
        }
    }
}
