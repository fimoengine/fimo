use darling::{FromAttributes, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{FnArg, TraitItem};
use uuid::Uuid;

#[derive(Debug, FromMeta)]
struct InterfaceArgs {
    vtable: syn::Path,
    #[darling(default)]
    no_dyn_impl: bool,
    #[darling(default, map = "crate::object::uuid_from_string")]
    uuid: Option<darling::Result<Uuid>>,
    #[darling(default)]
    generate: Option<darling::util::PathList>,
    #[darling(default, map = "map_lifetime")]
    obj_bound: Option<syn::Result<syn::LifetimeDef>>,
}

pub(crate) fn map_lifetime(bound: Option<String>) -> Option<syn::Result<syn::LifetimeDef>> {
    bound.map(|bound| syn::parse_str(&*bound))
}

pub fn interface_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let mut input = syn::parse_macro_input!(input as syn::ItemTrait);

    let InterfaceArgs {
        vtable,
        no_dyn_impl,
        uuid,
        generate,
        obj_bound,
    } = match InterfaceArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let uuid = match uuid.unwrap_or(Ok(Uuid::from_bytes([0; 16]))) {
        Ok(v) => *v.as_bytes(),
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let obj_bound = match obj_bound {
        None => syn::parse_quote!('inner),
        Some(Ok(bound)) => bound,
        Some(Err(e)) => return e.into_compile_error().into(),
    };

    let is_downcast_safe = input.generics.params.iter().count() == 0;

    let trait_ident = input.ident.clone();
    let mut generics = input.generics.clone();
    generics.where_clause = None;
    for generic in &mut generics.params {
        if let syn::GenericParam::Lifetime(bound) = generic {
            bound.bounds = syn::punctuated::Punctuated::new();
        } else {
            return syn::Error::new(generic.span(), "only generic lifetimes are supported")
                .to_compile_error()
                .into();
        }
    }

    let mut extended_generics = input.generics.clone();
    extended_generics.params.push(syn::parse_quote!(#obj_bound));

    let obj_bound_lifetime = &obj_bound.lifetime;
    let mut dyn_trait_idents: Vec<proc_macro2::TokenStream> = Vec::new();
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident #generics + #obj_bound_lifetime));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident #generics + Send + #obj_bound_lifetime));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident #generics + Sync + #obj_bound_lifetime));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident #generics + Unpin + #obj_bound_lifetime));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident #generics + Send + Sync + #obj_bound_lifetime));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident #generics + Send + Unpin + #obj_bound_lifetime));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident #generics + Sync + Unpin + #obj_bound_lifetime));
    dyn_trait_idents.push(
        syn::parse_quote!(dyn #trait_ident #generics + Send + Sync + Unpin + #obj_bound_lifetime),
    );

    let dyn_trait_base = dyn_trait_idents[0].clone();

    let impls = if let Some(vtables) = generate {
        self::generate_impls(&mut input, &vtable, &vtables, no_dyn_impl, &obj_bound)
    } else {
        quote!()
    };

    let downcast_safe_impls = if is_downcast_safe {
        quote! {
            unsafe impl #extended_generics ::fimo_ffi::ptr::DowncastSafe for #dyn_trait_base { }
        }
    } else {
        quote!()
    };

    let output = quote! {
        #input

        impl #extended_generics ::fimo_ffi::ptr::ObjInterfaceBase for #dyn_trait_base {
            type VTable = #vtable;
            const INTERFACE_ID: ::fimo_ffi::ptr::Uuid = ::fimo_ffi::ptr::Uuid::from_bytes([#(#uuid),*]);
        }

        #(
            impl #extended_generics ::fimo_ffi::ptr::ObjInterface for #dyn_trait_idents {
                type Base = #dyn_trait_base;
            }
        )*

        #downcast_safe_impls

        #impls
    };
    output.into()
}

#[derive(Default, Debug, FromAttributes)]
#[darling(attributes(vtable_info))]
struct FnVTableInfo {
    #[darling(default, rename = "unsafe")]
    r#unsafe: bool,
    #[darling(default)]
    ignore: bool,
    #[darling(default, map = "parse_lifetimes")]
    lifetimes: Option<syn::Result<syn::BoundLifetimes>>,
    #[darling(default, map = "parse_abi")]
    abi: Option<syn::Result<syn::Abi>>,
    #[darling(default, map = "parse_type")]
    return_type: Option<syn::Result<syn::Type>>,
    #[darling(default)]
    into: Option<syn::Path>,
    #[darling(default)]
    into_expr: syn::punctuated::Punctuated<syn::Expr, syn::token::Semi>,
    #[darling(default)]
    from: Option<syn::Path>,
    #[darling(default)]
    from_expr: syn::punctuated::Punctuated<syn::Expr, syn::token::Semi>,
}

#[derive(Default, FromAttributes)]
#[darling(attributes(vtable_info))]
struct ParamVTableInfo {
    #[darling(default, rename = "type", map = "parse_type")]
    r#type: Option<syn::Result<syn::Type>>,
    #[darling(default)]
    into: Option<syn::Path>,
    #[darling(default)]
    into_expr: syn::punctuated::Punctuated<syn::Expr, syn::token::Semi>,
    #[darling(default)]
    from: Option<syn::Path>,
    #[darling(default)]
    from_expr: syn::punctuated::Punctuated<syn::Expr, syn::token::Semi>,
}

fn parse_type(ty: String) -> Option<syn::Result<syn::Type>> {
    let ty = syn::parse_str(&ty);
    Some(ty)
}

fn parse_abi(ty: String) -> Option<syn::Result<syn::Abi>> {
    let ty = syn::parse_str(&ty);
    Some(ty)
}

fn parse_lifetimes(ty: String) -> Option<syn::Result<syn::BoundLifetimes>> {
    let ty = syn::parse_str(&ty);
    Some(ty)
}

fn generate_impls(
    trait_item: &mut syn::ItemTrait,
    vtable_name: &syn::Path,
    vtables: &darling::util::PathList,
    no_dyn_impl: bool,
    obj_bound: &syn::LifetimeDef,
) -> proc_macro2::TokenStream {
    let mut items = Vec::new();

    // extract and remove the attributes from all methods and parameters.
    for item in trait_item.items.iter_mut() {
        match item {
            TraitItem::Method(method) => {
                let attr = match extract_fn_attribute(&mut method.attrs) {
                    Ok(attr) => attr,
                    Err(e) => return e.write_errors(),
                };

                let mut param_attrs = Vec::new();
                for param in method.sig.inputs.iter_mut() {
                    let attr = match param {
                        FnArg::Receiver(p) => extract_param_attribute(&mut p.attrs),
                        FnArg::Typed(p) => extract_param_attribute(&mut p.attrs),
                    };

                    match attr {
                        Ok(attr) => param_attrs.push(attr),
                        Err(e) => return e.write_errors(),
                    }
                }

                items.push((attr, param_attrs, method.sig.clone()))
            }
            i => {
                let error = syn::Error::new(i.span(), "only methods are allowed in interfaces");
                return error.into_compile_error();
            }
        }
    }

    // construct the type of the functions
    let mut fn_types = Vec::new();
    for (attr, params, sig) in &items {
        if attr.ignore {
            fn_types.push(None);
            continue;
        }

        let mut inputs: syn::punctuated::Punctuated<syn::BareFnArg, _> = Default::default();
        for (input, attr) in sig.inputs.iter().zip(params) {
            let ty = if let Some(ty) = &attr.r#type {
                match ty {
                    Ok(t) => t.clone(),
                    Err(e) => return e.clone().into_compile_error(),
                }
            } else {
                match input {
                    FnArg::Receiver(i) => {
                        if i.mutability.is_some() {
                            syn::parse_quote!(*mut ())
                        } else {
                            syn::parse_quote!(*const ())
                        }
                    }
                    FnArg::Typed(i) => *i.ty.clone(),
                }
            };

            let attrs = match input {
                FnArg::Receiver(i) => i.attrs.clone(),
                FnArg::Typed(i) => i.attrs.clone(),
            };

            let arg = syn::BareFnArg {
                attrs,
                name: None,
                ty,
            };
            inputs.push(arg)
        }

        let lifetimes = if let Some(lifetimes) = attr.lifetimes.clone() {
            match lifetimes {
                Ok(lifetimes) => Some(lifetimes),
                Err(e) => return e.into_compile_error(),
            }
        } else if sig.generics.lifetimes().count() == 0 {
            None
        } else {
            let mut lifetimes = syn::BoundLifetimes::default();

            for lifetime in sig.generics.lifetimes() {
                lifetimes.lifetimes.push(lifetime.clone())
            }

            Some(lifetimes)
        };

        let unsafety: Option<syn::token::Unsafe> = if attr.r#unsafe {
            Some(Default::default())
        } else {
            sig.unsafety
        };

        let abi: Option<syn::Abi> = if let Some(abi) = attr.abi.clone() {
            match abi {
                Ok(abi) => Some(abi),
                Err(e) => return e.into_compile_error(),
            }
        } else {
            sig.abi.clone()
        };

        let output = if let Some(return_type) = attr.return_type.clone() {
            match return_type {
                Ok(return_type) => syn::ReturnType::Type(Default::default(), Box::new(return_type)),
                Err(e) => return e.into_compile_error(),
            }
        } else {
            sig.output.clone()
        };

        let fn_type = syn::TypeBareFn {
            lifetimes,
            unsafety,
            abi,
            fn_token: Default::default(),
            paren_token: Default::default(),
            inputs,
            variadic: sig.variadic.clone(),
            output,
        };
        fn_types.push(Some(fn_type));
    }

    let vtable_def = vtable_definition(
        trait_item,
        vtable_name,
        vtables,
        &items,
        &fn_types,
        obj_bound,
    );

    let dyn_obj_impl = if no_dyn_impl {
        quote!()
    } else {
        dyn_obj_implementation(trait_item, vtable_name, &items, &fn_types, obj_bound)
    };

    quote! {
        #dyn_obj_impl
        #vtable_def
    }
}

fn vtable_definition(
    trait_item: &syn::ItemTrait,
    vtable_name: &syn::Path,
    vtables: &darling::util::PathList,
    items: &[(FnVTableInfo, Vec<ParamVTableInfo>, syn::Signature)],
    fn_types: &[Option<syn::TypeBareFn>],
    obj_bound: &syn::LifetimeDef,
) -> proc_macro2::TokenStream {
    let obj_bound_lifetime = &obj_bound.lifetime;
    let vis = trait_item.vis.clone();
    let trait_ident = trait_item.ident.clone();
    let doc = format!("VTable for a [`{}`]", trait_item.ident);

    let (impl_generics, ty_generics, _) = trait_item.generics.split_for_impl();
    let interface: String = if trait_item.generics.lt_token.is_some() {
        quote!(for #impl_generics #trait_ident #ty_generics).to_string()
    } else {
        quote!(#trait_ident).to_string()
    };

    let mut vtable_members = Vec::new();

    // append all vtables to the member list.
    for (idx, vtable) in vtables.iter().enumerate() {
        let name = quote::format_ident!("__inner_{}", idx);
        let vtable_name = match vtable.get_ident() {
            None => "trait implementation".into(),
            Some(ident) => format!("[`{}`]", ident),
        };
        let doc = format!("VTable for a {}.", vtable_name);
        let vtable_type = if idx == 0 { "primary" } else { "secondary" };
        vtable_members.push(quote! {
            #[doc = #doc]
            #[super_vtable(is = #vtable_type)]
            pub #name: #vtable,
        })
    }

    // append all functions to the member list.
    for (idx, func) in fn_types.iter().enumerate() {
        let func = match func {
            Some(f) => f,
            None => continue,
        };

        let sig = &items[idx].2;
        let name = sig.ident.clone();
        let doc = format!(
            "Fn pointer implementing [`{}::{}`].",
            trait_item.ident, name
        );
        vtable_members.push(quote! {
            #[doc = #doc]
            pub #name: #func,
        });
    }

    let mut new_generics = trait_item.generics.clone();
    let mut new_emb_generics = new_generics.clone();

    new_generics.params.push(syn::parse_quote!(#obj_bound));
    new_generics.params.push(syn::parse_quote!(T));
    new_generics.where_clause = match new_generics.where_clause {
        None => Some(syn::parse_quote! {
            where
                T: #trait_ident #ty_generics + ::fimo_ffi::ptr::ObjectId + #obj_bound_lifetime
        }),
        Some(mut c) => {
            c.predicates.push(syn::parse_quote!(T: #trait_ident #ty_generics + ::fimo_ffi::ptr::ObjectId + #obj_bound_lifetime));
            Some(c)
        }
    };

    new_emb_generics.params.push(syn::parse_quote!(#obj_bound));
    new_emb_generics.params.push(syn::parse_quote!(T));
    new_emb_generics.params.push(syn::parse_quote!(Dyn));
    new_emb_generics.where_clause = match new_emb_generics.where_clause {
        None => Some(syn::parse_quote! {
            where
                T: #trait_ident #ty_generics + ::fimo_ffi::ptr::ObjectId + std::marker::Unsize<Dyn> + #obj_bound_lifetime,
                Dyn: ::fimo_ffi::ptr::ObjInterface + ?Sized + #obj_bound_lifetime
        }),
        Some(mut c) => {
            c.predicates.push(syn::parse_quote!(T: #trait_ident #ty_generics + ::fimo_ffi::ptr::ObjectId + std::marker::Unsize<Dyn> + #obj_bound_lifetime));
            c.predicates
                .push(syn::parse_quote!(Dyn: ::fimo_ffi::ptr::ObjInterface + ?Sized + #obj_bound_lifetime));
            Some(c)
        }
    };

    let (new_generics, _, new_where) = new_generics.split_for_impl();
    let (new_emb_generics, _, new_emb_where) = new_emb_generics.split_for_impl();

    let mut self_new_args: Vec<syn::Ident> = vec![syn::parse_quote!(offset)];

    let vtables = if vtables.is_empty() {
        quote! {}
    } else {
        let mut consts: Vec<_> = Vec::new();
        let mut tables: Vec<_> = Vec::new();
        for (idx, vtable) in vtables.iter().enumerate() {
            let name = quote::format_ident!("__inner_{}", idx);
            let const_ptr_name = quote::format_ident!("INNER_{}_PTR", idx);
            let const_offset_name = quote::format_ident!("INNER_{}_OFFSET", idx);

            consts.push(quote! {
                const #const_ptr_name: *const #vtable = unsafe { std::ptr::addr_of!((*UNINIT_PTR).#name) };
                const #const_offset_name: usize =
                    unsafe { (#const_ptr_name as *const u8).offset_from(UNINIT_PTR as *const u8) as usize };
            });

            tables.push(quote! {
                let #name = #vtable::new_for_embedded::<#obj_bound_lifetime, T, Dyn>(offset + #const_offset_name);
            });

            self_new_args.push(name);
        }

        quote! {
            const UNINIT: std::mem::MaybeUninit<#vtable_name> = std::mem::MaybeUninit::uninit();
            const UNINIT_PTR: *const #vtable_name = UNINIT.as_ptr();

            #(#consts)*

            #(#tables)*
        }
    };

    let mut fn_generics = trait_item.generics.clone();
    fn_generics
        .params
        .push(syn::parse_quote!(T: #trait_ident #ty_generics + ::fimo_ffi::ptr::ObjectId));

    let mut function_impls: Vec<_> = Vec::new();
    for (fn_ty, (attr, param_attrs, sig)) in fn_types.iter().zip(items) {
        let fn_ty = match fn_ty {
            Some(ty) => ty,
            None => continue,
        };

        let unsafety = fn_ty.unsafety;
        let abi = &fn_ty.abi;
        let output = &fn_ty.output;
        let name = &sig.ident;
        let mut inputs = sig.inputs.clone();
        let mut input_names = Vec::new();
        for (idx, input) in inputs.iter_mut().enumerate() {
            let input_name = quote::format_ident!("p_{}", idx);

            match input {
                FnArg::Receiver(r) => {
                    *input = FnArg::Typed(syn::PatType {
                        attrs: r.attrs.clone(),
                        pat: Box::new(syn::parse_quote!(#input_name)),
                        colon_token: Default::default(),
                        ty: Box::new(fn_ty.inputs[idx].ty.clone()),
                    })
                }
                FnArg::Typed(t) => {
                    t.pat = Box::new(syn::parse_quote!(#input_name));
                    t.ty = Box::new(fn_ty.inputs[idx].ty.clone());
                }
            }

            input_names.push(input_name);
        }

        let mut map_inputs = Vec::new();
        if fn_ty.inputs[0].ty == syn::parse_quote!(*mut ()) {
            map_inputs.push(quote!(let p_0 = unsafe { &mut *(p_0 as *mut T) };));
        } else if fn_ty.inputs[0].ty == syn::parse_quote!(*const ()) {
            map_inputs.push(quote!(let p_0 = unsafe { &*(p_0 as *const T) };));
        };

        for (param, attr) in param_attrs.iter().enumerate() {
            if let Some(from) = &attr.from {
                let input_name = quote::format_ident!("p_{}", param);
                map_inputs.push(quote!(let #input_name = unsafe { #from(#input_name) };));
            }

            for from_expr in &attr.from_expr {
                map_inputs.push(quote!(#from_expr;));
            }
        }

        let map_output = if let Some(into) = &attr.into {
            quote!(let res = unsafe { #into(res) };)
        } else {
            quote!(let res = res;)
        };

        let into_expr = if attr.into_expr.is_empty() {
            quote!(res)
        } else {
            let expr = attr.into_expr.iter();
            quote!(#(#expr);*)
        };

        let mut fn_generics = fn_generics.clone();
        if let Some(lifetimes) = &fn_ty.lifetimes {
            for lifetime in &lifetimes.lifetimes {
                fn_generics
                    .params
                    .push(syn::GenericParam::Lifetime(lifetime.clone()))
            }
        }

        let input_names_without_self = &input_names[1..];
        let (fn_generics, _, fn_where) = fn_generics.split_for_impl();

        function_impls.push(quote! {
            #[allow(clippy::type_complexity)]
            #[allow(clippy::too_many_arguments)]
            #unsafety #abi fn #name #fn_generics (#inputs) #output #fn_where {
                #(#map_inputs)*

                let res = p_0.#name(#(#input_names_without_self),*);
                #map_output
                #into_expr
            }

            let #name = #name::<T>;
        });
        self_new_args.push(name.clone());
    }

    let obj_bound_string = format!("{}", quote::ToTokens::to_token_stream(obj_bound));
    quote! {
        #[doc = #doc]
        #[::fimo_ffi::vtable(interface = #interface, obj_bound = #obj_bound_string)]
        #vis struct #vtable_name {
            #(#vtable_members)*
        }

        impl #vtable_name {
            /// Constructs a new vtable for a given type.
            #[inline]
            pub const fn new_for #new_generics () -> Self #new_where
            {
                Self::new_for_embedded::<T, dyn #trait_ident #ty_generics + #obj_bound_lifetime>(0)
            }

            /// Constructs a new vtable for a given type and interface with a custom offset.
            #[inline]
            pub const fn new_for_embedded #new_emb_generics (offset: usize) -> Self #new_emb_where
            {
                #vtables
                #(#function_impls)*

                Self::new_embedded::<T, Dyn>(#(#self_new_args),*)
            }
        }
    }
}

fn dyn_obj_implementation(
    trait_item: &syn::ItemTrait,
    vtable_name: &syn::Path,
    items: &[(FnVTableInfo, Vec<ParamVTableInfo>, syn::Signature)],
    fn_types: &[Option<syn::TypeBareFn>],
    obj_bound: &syn::LifetimeDef,
) -> proc_macro2::TokenStream {
    let obj_bound_lifetime = &obj_bound.lifetime;
    let trait_ident = &trait_item.ident;
    let supertraits = &trait_item.supertraits;
    let (_, trait_generics, _) = trait_item.generics.split_for_impl();

    // map the generics to `<..., 'inner>`
    let mut generics = trait_item.generics.clone();
    generics.params.push(syn::parse_quote!(#obj_bound));
    generics.params.push(syn::parse_quote!(T));

    // where T: Trait<...> + ?Sized + CastInto<dyn Trait<...> + bound>
    // where DynObj<T>: Super1 + ... + SuperN
    let where_clause = generics.make_where_clause();
    where_clause
        .predicates
        .push(syn::parse_quote!(T: #trait_ident #trait_generics + ?Sized));
    where_clause.predicates.push(
        syn::parse_quote!(T: ::fimo_ffi::ptr::CastInto<dyn #trait_ident #trait_generics + #obj_bound_lifetime>),
    );
    where_clause
        .predicates
        .push(syn::parse_quote!(::fimo_ffi::ptr::DynObj<T>: #supertraits));

    let mut impls: Vec<syn::ImplItemMethod> = Vec::new();
    for (idx, fn_ty) in fn_types.iter().enumerate() {
        if fn_ty.is_none() {
            continue;
        }

        let attr = &items[idx].0;
        let param_attrs = &items[idx].1;
        let mut sig = items[idx].2.clone();
        let mut param_idents = Vec::new();
        let mut param_mappings: Vec<proc_macro2::TokenStream> = Vec::new();
        for (idx, param) in sig.inputs.iter_mut().enumerate() {
            let param_ident = quote::format_ident!("p_{}", idx);
            param_idents.push(param_ident.clone());

            match param {
                FnArg::Receiver(param) => {
                    if param.mutability.is_some() {
                        param_mappings.push(syn::parse_quote!(let #param_ident: *mut Self = self;));
                    } else {
                        param_mappings
                            .push(syn::parse_quote!(let #param_ident: *const Self = self;))
                    }
                    param_mappings.push(syn::parse_quote!(let #param_ident = #param_ident as _;))
                }
                FnArg::Typed(param) => {
                    let old_ident = &param.pat;
                    param_mappings.push(syn::parse_quote!(let #param_ident = #old_ident;))
                }
            };

            if let Some(into) = &param_attrs[idx].into {
                param_mappings
                    .push(syn::parse_quote!(let #param_ident = unsafe { #into(#param_ident) };))
            }

            for into_expr in param_attrs[idx].into_expr.iter() {
                param_mappings.push(syn::parse_quote!(#into_expr;))
            }
        }

        let fn_ident = &sig.ident;
        let result_mapping = if let Some(from) = &attr.from {
            quote!(let res = unsafe { #from(res) };)
        } else {
            quote!(let res = res;)
        };

        let from_expr = &attr.from_expr;
        let from_expr = if from_expr.is_empty() {
            quote!(res)
        } else {
            let from_expr = from_expr.iter();
            quote!(#(#from_expr);*)
        };

        impls.push(syn::parse_quote! {
            #[inline]
            #sig {
                #(#param_mappings)*

                let __vtable: & #vtable_name = ::fimo_ffi::ptr::metadata(self).super_vtable();
                let res = unsafe { (__vtable.#fn_ident)(#(#param_idents),*) };

                #result_mapping
                #from_expr
            }
        })
    }

    let (impl_gen, _, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_gen #trait_ident #trait_generics for ::fimo_ffi::ptr::DynObj<T> #where_clause {
            #(#impls)*
        }
    }
}

fn extract_fn_attribute(attrs: &mut Vec<syn::Attribute>) -> darling::Result<FnVTableInfo> {
    let attr = FnVTableInfo::from_attributes(attrs);
    attrs.retain(|attr| *attr.path.get_ident().unwrap() != "vtable_info");
    attr
}

fn extract_param_attribute(attrs: &mut Vec<syn::Attribute>) -> darling::Result<ParamVTableInfo> {
    let attr = ParamVTableInfo::from_attributes(attrs);
    attrs.retain(|attr| *attr.path.get_ident().unwrap() != "vtable_info");
    attr
}
