use darling::{FromAttributes, FromMeta, ToTokens};
use proc_macro::TokenStream;
use quote::quote;
use std::sync::LazyLock;
use syn::spanned::Spanned;
use uuid::Uuid;

#[derive(Default, Debug, FromAttributes)]
#[darling(attributes(interface_cfg))]
struct InterfaceArgs {
    #[darling(default)]
    version: u32,
    #[darling(default)]
    vtable: Option<syn::Path>,
    #[darling(default)]
    no_dyn_impl: bool,
    #[darling(default)]
    abi: darling::util::SpannedValue<AbiMapping>,
    #[darling(default)]
    marshal: MarshalingStrategy,
    #[darling(default, map = "crate::object::uuid_from_string")]
    uuid: Option<darling::Result<Uuid>>,
}

#[derive(Default, FromAttributes)]
#[darling(attributes(interface_cfg))]
struct InterfaceFnArgs {
    #[darling(default)]
    since_minor: u32,
    #[darling(default)]
    abi: darling::util::SpannedValue<AbiMapping>,
    #[darling(default)]
    mapping: VTableMapping,
    #[darling(default)]
    marshal: MarshalingStrategy,
    #[darling(default)]
    phantom_parameter: Option<syn::Type>,
}

#[derive(Default, FromAttributes)]
#[darling(attributes(interface_cfg))]
struct InterfaceFnParamArgs {
    #[darling(default)]
    marshal: MarshalingStrategy,
}

#[derive(Debug, Default, FromMeta)]
enum MarshalingStrategy {
    /// Marshal with the default marshaler.
    #[default]
    Auto,
}

#[derive(Debug, Default, PartialEq, FromMeta)]
enum VTableMapping {
    /// Include the definition in the vtable.
    #[default]
    Include,
    // Exclude the definition from the vtable.
    Exclude,
    // Include the definition in the vtable, but mark it as optional.
    Optional {
        #[darling(default)]
        replace: Option<syn::Path>,
    },
}

#[derive(Debug, Default)]
enum AbiMapping {
    /// Inherit the abi from a previous cfg.
    #[default]
    Inherit,
    Explicit {
        abi: syn::Abi,
    },
}

impl FromMeta for AbiMapping {
    fn from_list(items: &[syn::NestedMeta]) -> darling::Result<Self> {
        let abi = <AbiMapping_ as FromMeta>::from_list(items)?;
        Self::check(abi)
    }

    fn from_string(lit: &str) -> darling::Result<Self> {
        let abi = <AbiMapping_ as FromMeta>::from_string(lit)?;
        Self::check(abi)
    }
}

#[derive(Debug, Default, FromMeta)]
enum AbiMapping_ {
    /// Inherit the abi from a previous cfg.
    #[default]
    Inherit,
    Explicit {
        abi: String,
    },
}

impl AbiMapping {
    fn map_abi(abi: &str) -> darling::Result<syn::Abi> {
        let extern_abi = format!("extern {abi:?}");
        syn::parse_str(&extern_abi).map_err(Into::into)
    }

    fn check(m: AbiMapping_) -> darling::Result<Self> {
        match m {
            AbiMapping_::Inherit => Ok(AbiMapping::Inherit),
            AbiMapping_::Explicit { abi } => Ok(AbiMapping::Explicit {
                abi: Self::map_abi(&abi)?,
            }),
        }
    }
}

mod kw {
    syn::custom_keyword!(interface);
    syn::custom_keyword!(marker);
    syn::custom_keyword!(frozen);
    syn::custom_keyword!(version);
    syn::custom_keyword!(major);
    syn::custom_keyword!(minor);
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct ItemInterface {
    pub interface_args: Option<syn::Attribute>,
    pub attrs: Vec<syn::Attribute>,
    pub vis: syn::Visibility,
    pub unsafety: Option<syn::Token![unsafe]>,
    pub frozen_token: Option<kw::frozen>,
    pub interface_token: kw::interface,
    pub ident: syn::Ident,
    pub colon_token: Option<syn::Token![:]>,
    pub extends: syn::punctuated::Punctuated<InterfaceTypeParamBound, syn::Token![+]>,
    pub brace_token: syn::token::Brace,
    pub items: Vec<syn::TraitItemMethod>,
}

impl ItemInterface {
    fn get_input_args(&self) -> darling::Result<InterfaceArgs> {
        if let Some(arg) = &self.interface_args {
            InterfaceArgs::from_attributes(std::slice::from_ref(arg))
        } else {
            Ok(InterfaceArgs::default())
        }
    }

    fn has_static_bound(&self) -> bool {
        self.extends.iter().any(|b| match b {
            InterfaceTypeParamBound::Lifetime(lt) => *lt == syn::parse_quote!('static),
            _ => false,
        })
    }

    fn get_constraints(&self) -> syn::Result<Vec<proc_macro2::TokenStream>> {
        let interface_ident = &self.ident;

        let mut constraints = Vec::new();
        constraints.push(quote! {
            const _: () = ::fimo_ffi::ptr::__assert_ibase::<dyn #interface_ident>();
        });

        for bound in &self.extends {
            if let InterfaceTypeParamBound::InterfaceBound(interface) = bound {
                let t = syn::TraitBound {
                    paren_token: interface.paren_token,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: interface.lifetimes.clone(),
                    path: interface.path.clone(),
                };

                let (major, minor) = interface.version.get_version()?;
                let minor = minor.unwrap_or(0);

                constraints.push(quote! {
                    const _: () = ::fimo_ffi::ptr::__assert_ibase::<dyn #t>();
                });

                let major_msg = format!(
                    "Major version mismatch for interface `{}`",
                    t.to_token_stream()
                );
                let minor_msg = format!(
                    "Minor version mismatch for interface `{}`",
                    t.to_token_stream()
                );

                if interface.frozen_token.is_some() {
                    let frozen_msg = format!(
                        "Interface `{}` must be marked as frozen",
                        t.to_token_stream()
                    );

                    constraints.push(quote! {
                        const _: () = ::std::assert!(<dyn #t as ::fimo_ffi::ptr::ObjInterfaceBase>::IS_FROZEN, #frozen_msg);
                        const _: () = ::std::assert!(<dyn #t as ::fimo_ffi::ptr::ObjInterfaceBase>::INTERFACE_VERSION_MAJOR == #major, #major_msg);
                        const _: () = ::std::assert!(<dyn #t as ::fimo_ffi::ptr::ObjInterfaceBase>::INTERFACE_VERSION_MINOR == #minor, #minor_msg);
                    });
                } else {
                    constraints.push(quote! {
                        const _: () = ::std::assert!(<dyn #t as ::fimo_ffi::ptr::ObjInterfaceBase>::INTERFACE_VERSION_MAJOR == #major, #major_msg);
                        const _: () = ::std::assert!(<dyn #t as ::fimo_ffi::ptr::ObjInterfaceBase>::INTERFACE_VERSION_MINOR >= #minor, #minor_msg);
                    });
                }
            }
        }

        Ok(constraints)
    }

    fn get_interfaces(&self) -> Vec<InterfaceBound> {
        let mut bounds = Vec::new();
        for i in &self.extends {
            if let InterfaceTypeParamBound::InterfaceBound(i) = i {
                bounds.push(i.clone())
            }
        }

        bounds
    }

    fn construct_trait(&self) -> syn::ItemTrait {
        let trait_token = syn::token::Trait {
            span: self.interface_token.span(),
        };

        let generics = syn::Generics {
            lt_token: None,
            params: syn::punctuated::Punctuated::new(),
            gt_token: None,
            where_clause: None,
        };

        let mut supertraits = syn::punctuated::Punctuated::new();
        for pair in self.extends.pairs() {
            let bound = pair.value();
            match bound {
                InterfaceTypeParamBound::MarkerBound(b) => {
                    supertraits.push_value(syn::TypeParamBound::Trait(syn::TraitBound {
                        paren_token: None,
                        modifier: syn::TraitBoundModifier::None,
                        lifetimes: None,
                        path: b.path.clone(),
                    }))
                }
                InterfaceTypeParamBound::InterfaceBound(b) => {
                    supertraits.push_value(syn::TypeParamBound::Trait(syn::TraitBound {
                        paren_token: b.paren_token,
                        modifier: syn::TraitBoundModifier::None,
                        lifetimes: b.lifetimes.clone(),
                        path: b.path.clone(),
                    }))
                }
                InterfaceTypeParamBound::Lifetime(lt) => {
                    supertraits.push_value(syn::TypeParamBound::Lifetime(lt.clone()))
                }
            }

            if let Some(&punct) = pair.punct() {
                supertraits.push_punct(*punct)
            }
        }

        let items = self
            .items
            .iter()
            .cloned()
            .map(syn::TraitItem::Method)
            .collect();

        syn::ItemTrait {
            attrs: self.attrs.clone(),
            vis: self.vis.clone(),
            unsafety: self.unsafety,
            auto_token: None,
            trait_token,
            ident: self.ident.clone(),
            generics,
            colon_token: self.colon_token,
            supertraits,
            brace_token: self.brace_token,
            items,
        }
    }
}

impl syn::parse::Parse for ItemInterface {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let interface_args = input.call(syn::Attribute::parse_inner)?;
        if interface_args.len() > 1 {
            return Err(syn::Error::new_spanned(
                interface_args[1].clone(),
                "expected interface",
            ));
        }

        let interface_args = if interface_args.is_empty() {
            None
        } else {
            Some(interface_args[0].clone())
        };

        let outer_attrs = input.call(syn::Attribute::parse_outer)?;
        let vis: syn::Visibility = input.parse()?;
        let unsafety: Option<syn::Token![unsafe]> = input.parse()?;
        let frozen_token: Option<kw::frozen> = input.parse()?;
        let interface_token: kw::interface = input.parse()?;
        let ident: syn::Ident = input.parse()?;

        let colon_token: Option<syn::Token![:]> = input.parse()?;

        let mut extends = syn::punctuated::Punctuated::new();
        if colon_token.is_some() {
            loop {
                if input.peek(syn::token::Brace) {
                    break;
                }
                extends.push_value(input.parse()?);
                if input.peek(syn::token::Brace) {
                    break;
                }
                extends.push_punct(input.parse()?);
            }
        }

        let content;
        let brace_token = syn::braced!(content in input);

        let inner_attrs = content.call(syn::Attribute::parse_inner)?;
        let mut attrs = outer_attrs;
        attrs.extend(inner_attrs);

        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(content.parse()?);
        }

        Ok(Self {
            interface_args,
            attrs,
            vis,
            unsafety,
            frozen_token,
            interface_token,
            ident,
            colon_token,
            extends,
            brace_token,
            items,
        })
    }
}

#[derive(Debug, Clone)]
enum InterfaceTypeParamBound {
    MarkerBound(MarkerBound),
    InterfaceBound(InterfaceBound),
    Lifetime(syn::Lifetime),
}

impl syn::parse::Parse for InterfaceTypeParamBound {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Lifetime) {
            return input.parse().map(InterfaceTypeParamBound::Lifetime);
        }

        if input.peek(kw::marker) {
            return input.parse().map(InterfaceTypeParamBound::MarkerBound);
        }

        if input.peek(syn::token::Paren) {
            let content;
            let paren_token = syn::parenthesized!(content in input);
            let mut bound: InterfaceBound = content.parse()?;
            bound.paren_token = Some(paren_token);
            return Ok(InterfaceTypeParamBound::InterfaceBound(bound));
        }

        input.parse().map(InterfaceTypeParamBound::InterfaceBound)
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct MarkerBound {
    pub marker_token: kw::marker,
    pub path: syn::Path,
}

impl syn::parse::Parse for MarkerBound {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let marker_token: kw::marker = input.parse()?;

        let mut path: syn::Path = input.parse()?;
        if path.segments.last().unwrap().arguments.is_empty()
            && (input.peek(syn::token::Paren)
                || input.peek(syn::Token![::]) && input.peek3(syn::token::Paren))
        {
            input.parse::<Option<syn::Token![::]>>()?;
            let args: syn::ParenthesizedGenericArguments = input.parse()?;
            let parenthesized = syn::PathArguments::Parenthesized(args);
            path.segments.last_mut().unwrap().arguments = parenthesized;
        }

        if !(path.is_ident("Send")
            || path.is_ident("Sync")
            || path.is_ident("Unpin")
            || path.is_ident("IBase"))
        {
            return Err(syn::Error::new_spanned(
                path,
                "unknown bound, expected one of `Send`, `Sync`, `Unpin`, `IBase`",
            ));
        }

        Ok(Self { marker_token, path })
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct InterfaceBound {
    pub paren_token: Option<syn::token::Paren>,
    pub lifetimes: Option<syn::BoundLifetimes>,
    pub path: syn::Path,
    pub at_token: syn::Token![@],
    pub frozen_token: Option<kw::frozen>,
    pub version: InterfaceVersion,
}

impl syn::parse::Parse for InterfaceBound {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lifetimes: Option<syn::BoundLifetimes> = input.parse()?;

        let mut path: syn::Path = input.parse()?;
        if path.segments.last().unwrap().arguments.is_empty()
            && (input.peek(syn::token::Paren)
                || input.peek(syn::Token![::]) && input.peek3(syn::token::Paren))
        {
            input.parse::<Option<syn::Token![::]>>()?;
            let args: syn::ParenthesizedGenericArguments = input.parse()?;
            let parenthesized = syn::PathArguments::Parenthesized(args);
            path.segments.last_mut().unwrap().arguments = parenthesized;
        }

        let at_token: syn::Token![@] = input.parse()?;
        let frozen_token: Option<kw::frozen> = input.parse()?;
        let version: InterfaceVersion = input.parse()?;

        Ok(Self {
            paren_token: None,
            lifetimes,
            path,
            at_token,
            frozen_token,
            version,
        })
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct InterfaceVersion {
    pub paren_token: syn::token::Paren,
    pub version_token: kw::version,
    pub version: syn::LitStr,
}

static VERSION_VALIDATOR: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(\d+)(\.(\d+))?$").unwrap());

impl InterfaceVersion {
    fn get_version(&self) -> syn::Result<(u32, Option<u32>)> {
        let val = self.version.value();
        let captures = VERSION_VALIDATOR.captures(&val).unwrap();

        let major: u32 = captures[1].parse().map_err(|_| {
            syn::Error::new_spanned(&self.version, "could not parse the major version to a u32")
        })?;

        let minor = if let Some(x) = captures.get(3) {
            Some(x.as_str().parse().map_err(|_| {
                syn::Error::new_spanned(&self.version, "could not parse the minor version to a u32")
            })?)
        } else {
            None
        };

        Ok((major, minor))
    }
}

impl syn::parse::Parse for InterfaceVersion {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let version_token: kw::version = input.parse()?;

        let content;
        let paren_token = syn::parenthesized!(content in input);
        let version: syn::LitStr = content.parse()?;

        if !VERSION_VALIDATOR.is_match(&version.value()) {
            return Err(syn::Error::new_spanned(
                version,
                "expected \"a\" or \"a.b\"",
            ));
        }

        Ok(Self {
            paren_token,
            version_token,
            version,
        })
    }
}

struct InterfaceContext {
    version_major: u32,
    version_minor: u32,
    vtable: syn::Path,
    mappings: Vec<InterfaceFnContext>,
}

struct InterfaceFnContext {
    version: u32,
    optional: Option<Option<syn::Path>>,
    abi: syn::Abi,
    marshaler: syn::Path,
    generics: syn::Generics,
    receiver: syn::Path,
    receiver_lifetime: syn::Lifetime,
    inputs: Vec<syn::TypePath>,
    phantom_parameter: Option<proc_macro2::TokenStream>,
    output: syn::TypePath,
    marshal: Vec<syn::Expr>,
    demarshal: Vec<syn::Expr>,
    vtable_ty: syn::Type,
    method: syn::TraitItemMethod,
}

impl InterfaceContext {
    fn new(
        item: &mut syn::ItemTrait,
        version_major: u32,
        vtable: &syn::Path,
        abi: darling::util::SpannedValue<AbiMapping>,
        marshal: MarshalingStrategy,
    ) -> darling::Result<Self> {
        let item_ident = item.ident.clone();

        // Construct the mappings.
        let mut mappings: Vec<InterfaceFnContext> = Vec::new();
        for item in item.items.iter_mut() {
            match item {
                syn::TraitItem::Method(m) => {
                    let m = Self::parse_method(&item_ident, m, &abi, &marshal)?;
                    if let Some(m) = m {
                        if let Some(prev) = mappings.last() {
                            if prev.version > m.version {
                                let error = syn::Error::new(
                                    m.method.span(),
                                    "method has lower version requirements than the previous method",
                                );
                                return Err(error.into());
                            }
                        }

                        mappings.push(m);
                    }
                }
                _ => unreachable!(),
            }
        }

        let version_minor = mappings.last().map(|m| m.version).unwrap_or(0);

        Ok(Self {
            version_major,
            version_minor,
            vtable: vtable.clone(),
            mappings,
        })
    }

    fn parse_method(
        trait_ident: &syn::Ident,
        method: &mut syn::TraitItemMethod,
        abi: &darling::util::SpannedValue<AbiMapping>,
        marshal: &MarshalingStrategy,
    ) -> darling::Result<Option<InterfaceFnContext>> {
        // Consume attribute in method definition.
        let method_args = InterfaceFnArgs::from_attributes(&method.attrs)?;
        method
            .attrs
            .retain(|attr| !attr.path.is_ident("interface_cfg"));

        if method_args.mapping == VTableMapping::Exclude {
            if method.default.is_none() {
                let error = syn::Error::new(
                    method.span(),
                    "excluded methods require a default implementation",
                );
                return Err(error.into());
            }

            return Ok(None);
        }

        if matches!(
            method_args.mapping,
            VTableMapping::Optional { replace: None }
        ) && method.default.is_none()
        {
            let error = syn::Error::new(
                method.span(),
                "optional methods require a default implementation or a replacement",
            );
            return Err(error.into());
        }

        if method_args.since_minor > 0 && method.default.is_none() {
            let error = syn::Error::new(
                method.span(),
                "methods added after minor version 0 require a default implementation",
            );
            return Err(error.into());
        }

        // Inherit abi from trait or use the one defined with the method.
        let (abi, abi_span) = match method_args.abi.as_ref() {
            AbiMapping::Inherit => match abi.as_ref() {
                AbiMapping::Inherit => (
                    method
                        .sig
                        .abi
                        .clone()
                        .unwrap_or_else(|| syn::parse_quote!(extern "Rust")),
                    abi.span(),
                ),
                AbiMapping::Explicit { abi } => (abi.clone(), abi.span()),
            },
            AbiMapping::Explicit { abi } => (abi.clone(), abi.span()),
        };

        // Inherit marshaler from trait or use the one defined with the method.
        let marshaler = match &method_args.marshal {
            MarshalingStrategy::Auto => match &marshal {
                MarshalingStrategy::Auto => Self::default_marshaler(&abi_span, &abi)?,
            },
        };

        // Don't allow generic type parameters.
        if let Some(ty) = method.sig.generics.type_params().next() {
            let error = syn::Error::new(ty.span(), "generic type parameters are not allowed");
            return Err(error.into());
        }

        // Don't allow generic constant parameters.
        if let Some(ty) = method.sig.generics.const_params().next() {
            let error = syn::Error::new(ty.span(), "generic const parameters are not allowed");
            return Err(error.into());
        }

        // Don't allow where clauses.
        if let Some(where_clause) = &method.sig.generics.where_clause {
            let error = syn::Error::new(where_clause.span(), "where clauses are not allowed");
            return Err(error.into());
        }

        if method.sig.inputs.is_empty() {
            let error = syn::Error::new(
                method.sig.inputs.span(),
                "method must accept at least one `self`",
            );
            return Err(error.into());
        }

        // Parse the first input of the method.
        let (receiver, rec_lifetime, rec_lt_overwrite) =
            Self::parse_receiver(trait_ident, method.sig.inputs.first().unwrap())?;

        // Parse the remaining inputs.
        let mut inputs = Vec::new();
        let mut marshal_exprs = Vec::new();
        let mut demarshal_exprs = Vec::new();
        for input in method.sig.inputs.iter_mut().skip(1) {
            let (ty, marshal_expr, demarshal_expr) = Self::parse_input(input, &marshaler)?;
            inputs.push(ty);
            marshal_exprs.push(marshal_expr);
            demarshal_exprs.push(demarshal_expr);
        }

        // Add an optional PhantomData parameter.
        let phantom_parameter = method_args
            .phantom_parameter
            .as_ref()
            .map(|p| quote!(__private_phantom: std::marker::PhantomData<#p>,));

        // Parse and adapt the return type.
        let return_type = Self::parse_return_type(&method.sig.output, &rec_lifetime, &marshaler)?;

        // Add the new lifetime in case we replaced an empty one.
        let mut method_generics = method.sig.generics.clone();
        if rec_lt_overwrite {
            method_generics
                .params
                .push(syn::parse_quote!(#rec_lifetime));
        }

        let gen = if method_generics.params.is_empty() {
            quote! {}
        } else {
            let (m_impl_gen, _, _) = method_generics.split_for_impl();
            quote!(for #m_impl_gen )
        };

        let (vtable_ty, optional) = match method_args.mapping {
            VTableMapping::Include => (
                syn::parse_quote_spanned!(method.span() => #gen unsafe #abi fn (#receiver, #(#inputs),* #phantom_parameter) -> #return_type),
                None,
            ),
            VTableMapping::Exclude => unreachable!(),
            VTableMapping::Optional { replace } => (
                syn::parse_quote_spanned!(method.span() => Option<#gen unsafe #abi fn (#receiver, #(#inputs),* #phantom_parameter) -> #return_type>),
                Some(replace),
            ),
        };

        Ok(Some(InterfaceFnContext {
            version: method_args.since_minor,
            optional,
            abi,
            marshaler,
            generics: method_generics,
            receiver,
            receiver_lifetime: rec_lifetime,
            inputs,
            phantom_parameter,
            output: return_type,
            marshal: marshal_exprs,
            demarshal: demarshal_exprs,
            vtable_ty,
            method: method.clone(),
        }))
    }

    fn parse_receiver(
        trait_ident: &syn::Ident,
        arg: &syn::FnArg,
    ) -> darling::Result<(syn::Path, syn::Lifetime, bool)> {
        match arg {
            syn::FnArg::Receiver(r) => {
                // Check that we are borrowing `Self` and remember the lifetime used.
                let (lifetime, overwrite) = if let Some((_, lifetime)) = &r.reference {
                    match lifetime.clone() {
                        Some(l) if l != syn::parse_quote!('_) => (l, false),
                        _ => (syn::parse_quote!('__private_this), true),
                    }
                } else {
                    let error = syn::Error::new(r.span(), "`self` is not allowed, try borrowing");
                    return Err(error.into());
                };

                let this_ty = if r.mutability.is_none() {
                    syn::parse_quote!(::fimo_ffi::ptr::ThisPtr<#lifetime, dyn #trait_ident + #lifetime>)
                } else {
                    syn::parse_quote!(::fimo_ffi::ptr::MutThisPtr<#lifetime, dyn #trait_ident + #lifetime>)
                };

                Ok((this_ty, lifetime, overwrite))
            }
            _ => {
                let error =
                    syn::Error::new(arg.span(), "first argument must be `&self` or `&mut self`");
                Err(error.into())
            }
        }
    }

    fn parse_input(
        arg: &mut syn::FnArg,
        marshaler: &syn::Path,
    ) -> darling::Result<(syn::TypePath, syn::Expr, syn::Expr)> {
        match arg {
            syn::FnArg::Receiver(_) => {
                let error = syn::Error::new(
                    arg.span(),
                    "`self`, `&self` and `&mut self` not allowed in this position",
                );
                Err(error.into())
            }
            syn::FnArg::Typed(t) => {
                // Consume attribute from input.
                let args = InterfaceFnParamArgs::from_attributes(&t.attrs)?;
                t.attrs.retain(|attr| !attr.path.is_ident("interface_cfg"));

                // Inherit marshaler from trait or use the one defined with the method.
                let marshaler = match &args.marshal {
                    MarshalingStrategy::Auto => &marshaler,
                };

                let ident = t.pat.clone();
                let ty = t.ty.clone();
                let vtable_ty = syn::parse_quote!(<#ty as #marshaler>::Type);
                let marshal = syn::parse_quote!(let #ident = #marshaler::marshal( #ident ));
                let demarshal = syn::parse_quote!(let #ident = #marshaler::demarshal( #ident ));

                Ok((vtable_ty, marshal, demarshal))
            }
        }
    }

    fn parse_return_type(
        rt: &syn::ReturnType,
        lt: &syn::Lifetime,
        marshaler: &syn::Path,
    ) -> darling::Result<syn::TypePath> {
        match rt {
            syn::ReturnType::Default => {
                Ok(syn::parse_quote_spanned!(rt.span() => <() as #marshaler>::Type ))
            }
            syn::ReturnType::Type(_, t) => {
                let mut t = t.clone();
                Self::adapt_return_type(&mut t, lt)?;
                Ok(syn::parse_quote_spanned!(rt.span() => <#t as #marshaler>::Type ))
            }
        }
    }

    fn adapt_return_type(ty: &mut syn::Type, lt: &syn::Lifetime) -> darling::Result<()> {
        match ty {
            syn::Type::Array(x) => Self::adapt_return_type(&mut x.elem, lt),
            syn::Type::BareFn(_) => Ok(()),
            syn::Type::Group(x) => Self::adapt_return_type(&mut x.elem, lt),
            syn::Type::ImplTrait(x) => {
                let error = syn::Error::new(x.span(), "`impl T` return type is not allowed");
                Err(error.into())
            }
            syn::Type::Infer(x) => {
                let error = syn::Error::new(x.span(), "`_` return type is not allowed");
                Err(error.into())
            }
            syn::Type::Macro(x) => {
                let error = syn::Error::new(x.span(), "macro return type is not allowed");
                Err(error.into())
            }
            syn::Type::Never(_) => Ok(()),
            syn::Type::Paren(x) => Self::adapt_return_type(&mut x.elem, lt),
            syn::Type::Path(x) => {
                if let Some(qself) = &mut x.qself {
                    Self::adapt_return_type(&mut qself.ty, lt)?;
                }

                x.path = Self::adapt_path(&x.path, lt)?;
                Ok(())
            }
            syn::Type::Ptr(x) => Self::adapt_return_type(&mut x.elem, lt),
            syn::Type::Reference(x) => {
                if let Some(x) = &mut x.lifetime {
                    if *x == syn::parse_quote!('_) {
                        *x = lt.clone();
                    }
                } else {
                    x.lifetime = Some(lt.clone());
                }

                Self::adapt_return_type(&mut x.elem, lt)
            }
            syn::Type::Slice(x) => Self::adapt_return_type(&mut x.elem, lt),
            syn::Type::TraitObject(x) => {
                for b in x.bounds.iter_mut() {
                    match b {
                        syn::TypeParamBound::Trait(t) => {
                            Self::adapt_path(&t.path, lt)?;
                        }
                        syn::TypeParamBound::Lifetime(l) => {
                            if *l == syn::parse_quote!('_) {
                                *l = lt.clone();
                            }
                        }
                    }
                }

                Ok(())
            }
            syn::Type::Tuple(x) => {
                for t in x.elems.iter_mut() {
                    Self::adapt_return_type(t, lt)?;
                }

                Ok(())
            }
            syn::Type::Verbatim(x) => {
                let error = syn::Error::new(x.span(), "unknown return type is not allowed");
                Err(error.into())
            }
            _ => {
                let error = syn::Error::new(ty.span(), "macro return type is not allowed");
                Err(error.into())
            }
        }
    }

    fn adapt_path(path: &syn::Path, lt: &syn::Lifetime) -> darling::Result<syn::Path> {
        let mut p = path.clone();
        for segment in p.segments.iter_mut() {
            match &mut segment.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(x) => {
                    for arg in x.args.iter_mut() {
                        match arg {
                            syn::GenericArgument::Lifetime(l) => {
                                if *l == syn::parse_quote!('_) {
                                    *l = lt.clone();
                                }
                            }
                            syn::GenericArgument::Type(t) => Self::adapt_return_type(t, lt)?,
                            syn::GenericArgument::Binding(b) => {
                                Self::adapt_return_type(&mut b.ty, lt)?
                            }
                            syn::GenericArgument::Constraint(c) => {
                                for bound in c.bounds.iter_mut() {
                                    match bound {
                                        syn::TypeParamBound::Trait(t) => {
                                            Self::adapt_path(&t.path, lt)?;
                                        }
                                        syn::TypeParamBound::Lifetime(l) => {
                                            if *l == syn::parse_quote!('_) {
                                                *l = lt.clone();
                                            }
                                        }
                                    }
                                }
                            }
                            syn::GenericArgument::Const(c) => {
                                let error =
                                    syn::Error::new(c.span(), "const expressions are not allowed");
                                return Err(error.into());
                            }
                        }
                    }
                }
                syn::PathArguments::Parenthesized(x) => {
                    for input in x.inputs.iter_mut() {
                        Self::adapt_return_type(input, lt)?;
                    }

                    match &mut x.output {
                        syn::ReturnType::Default => {}
                        syn::ReturnType::Type(_, t) => Self::adapt_return_type(t, lt)?,
                    }
                }
            }
        }

        Ok(p)
    }

    fn default_marshaler(span: &proc_macro2::Span, abi: &syn::Abi) -> darling::Result<syn::Path> {
        match abi
            .name
            .as_ref()
            .map(|n| n.value())
            .unwrap_or_else(String::new)
            .as_str()
        {
            "" | "Rust" => Ok(syn::parse_quote!(::fimo_ffi::marshal::RustTypeBridge)),
            "C" | "C-unwind" => Ok(syn::parse_quote!(::fimo_ffi::marshal::CTypeBridge)),
            _ => Err(
                syn::Error::new(*span, "no default marshaler found for the requested abi").into(),
            ),
        }
    }
}

impl InterfaceContext {
    fn generate_vtable(
        &self,
        trait_item: &syn::ItemTrait,
        static_self_bound: bool,
        interfaces: &[InterfaceBound],
    ) -> proc_macro2::TokenStream {
        let trait_vis = trait_item.vis.clone();
        let trait_ident = trait_item.ident.clone();

        // Documentation of the VTable type.
        let doc = format!("VTable for a [`{}`]", trait_ident);

        // Name of the vtable.
        let vtable_ident = &self.vtable;

        // Name of the vtable head and data sections.
        let vtable_head_ident = quote::format_ident!("{}Head", vtable_ident.get_ident().unwrap());
        let vtable_data_ident = quote::format_ident!("{}Data", vtable_ident.get_ident().unwrap());

        // VTable definition
        let vtable = quote! {
            #[doc = #doc]
            #[repr(transparent)]
            #[allow(clippy::type_complexity)]
            #trait_vis struct #vtable_ident {
                #[doc = r"Internal vtable implementation."]
                pub inner: ::fimo_ffi::ptr::GenericVTable<#vtable_head_ident, #vtable_data_ident>,
            }

            impl #vtable_ident {
                #[doc = r"Constructs a new vtable."]
                #[inline]
                pub const fn new(head: #vtable_head_ident, data: #vtable_data_ident) -> Self {
                    Self {
                        inner: ::fimo_ffi::ptr::GenericVTable::new(head, data)
                    }
                }

                #[doc = r"Fetches a reference to the head section."]
                #[inline]
                pub const fn head(&self) -> &#vtable_head_ident {
                    self.inner.head()
                }

                #[doc = r"Fetches a reference to the data section."]
                #[inline]
                pub const fn data(&self) -> &#vtable_data_ident {
                    self.inner.data()
                }
            }

            unsafe impl ::fimo_ffi::ptr::ObjMetadataCompatible for #vtable_ident {}
        };

        // Head section definition.
        let vtable_head = {
            let mut head_impls = Vec::new();
            if trait_ident != "IBase" {
                head_impls.push(quote! {
                    impl<'__private_this> const ::fimo_ffi::ptr::IntoInterface<#vtable_ident> for dyn #trait_ident + '__private_this {
                        #[inline]
                        fn into_vtable(vtable: &Self::VTable) -> &#vtable_ident {
                            vtable
                        }
                    }
                });
            }

            if interfaces.is_empty() {
                head_impls.push(quote! {
                    impl<'__private_this> const ::fimo_ffi::ptr::IntoInterface<::fimo_ffi::ptr::IBaseVTable> for dyn #trait_ident + '__private_this {
                        #[inline]
                        fn into_vtable(vtable: &Self::VTable) -> &::fimo_ffi::ptr::IBaseVTable {
                            unsafe { std::mem::transmute(vtable) }
                        }
                    }
                });
            }

            let mut frozen_optimization = true;
            let mut interface_fields = Vec::new();
            let mut interface_offsets = Vec::new();
            let mut interface_new_impl = Vec::new();
            let mut interface_new_impl_offset_args = Vec::new();
            let mut interface_new_impl_table_args = Vec::new();

            if !interfaces.is_empty() {
                interface_new_impl.push(quote! {
                    const UNINIT: ::std::mem::MaybeUninit<#vtable_head_ident> = ::std::mem::MaybeUninit::uninit();
                    const UNINIT_PTR: *const #vtable_head_ident = UNINIT.as_ptr();
                });
            }

            for (idx, i) in interfaces.iter().enumerate() {
                let i_name = quote::format_ident!("__inner_{idx}");
                let i_doc = format!("VTable for [`{}`]", i.path.to_token_stream());
                let i_path = &i.path;
                let i_ty: syn::Type =
                    syn::parse_quote!(<dyn #i_path as ::fimo_ffi::ptr::ObjInterfaceBase>::VTable);

                interface_fields.push(quote! {
                    #[doc = #i_doc]
                    pub #i_name: #i_ty,
                });

                let const_ptr_name = quote::format_ident!("INNER_{}_PTR", idx);
                let const_offset_name = quote::format_ident!("INNER_{}_OFFSET", idx);
                interface_new_impl.push(quote! {
                    const #const_ptr_name: *const #i_ty = unsafe { ::std::ptr::addr_of!((*UNINIT_PTR).#i_name) };
                    const #const_offset_name: usize =
                        unsafe { (#const_ptr_name as *const u8).offset_from(UNINIT_PTR as *const u8) as usize };
                    let #i_name: #i_ty = #i_ty::new_for_embedded::<T>(__internal_this_offset + #const_offset_name);
                });
                interface_new_impl_table_args.push(quote!(#i_name: #i_name,));

                if !frozen_optimization {
                    let i_offset_name = quote::format_ident!("__inner_{idx}_offset");
                    let i_offset_doc =
                        format!("Offset to the VTable for [`{}`]", i.path.to_token_stream());

                    interface_offsets.push(quote! {
                        #[doc = #i_offset_doc]
                        pub #i_offset_name: usize,
                    });
                    interface_new_impl_offset_args
                        .push(quote!(#i_offset_name: #const_offset_name,));

                    if idx == 0 {
                        head_impls.push(quote! {
                            impl<'__private_this, T> const ::fimo_ffi::ptr::IntoInterface<T> for dyn #trait_ident + '__private_this
                            where
                                T: ::fimo_ffi::ptr::ObjMetadataCompatible,
                                dyn #i_path: ~const ::fimo_ffi::ptr::IntoInterface<T>,
                            {
                                #[inline]
                                fn into_vtable(vtable: &Self::VTable) -> &T {
                                    let head = vtable.head();
                                    let offset = head.#i_offset_name;
                                    let head_ptr: *const u8 = head as *const _ as *const u8;
                                    unsafe {
                                        let inner_ptr: *const #i_ty = head_ptr.add(offset) as *const #i_ty;
                                        <dyn #i_path>::into_vtable(&*inner_ptr)
                                    }
                                }
                            }
                        });
                    } else {
                        head_impls.push(quote! {
                            impl<'__private_this> const ::fimo_ffi::ptr::IntoInterface<#i_ty> for dyn #trait_ident + '__private_this {
                                #[inline]
                                fn into_vtable(vtable: &Self::VTable) -> &#i_ty {
                                    let head = vtable.head();
                                    let offset = head.#i_offset_name;
                                    let head_ptr: *const u8 = head as *const _ as *const u8;
                                    unsafe {
                                        let inner_ptr: *const #i_ty = head_ptr.add(offset) as *const #i_ty;
                                        &*inner_ptr
                                    }
                                }
                            }
                        });
                    }
                } else if idx == 0 {
                    head_impls.push(quote! {
                        impl<'__private_this, T> const ::fimo_ffi::ptr::IntoInterface<T> for dyn #trait_ident + '__private_this
                        where
                            T: ::fimo_ffi::ptr::ObjMetadataCompatible,
                            dyn #i_path: ~const ::fimo_ffi::ptr::IntoInterface<T>,
                        {
                            #[inline]
                            fn into_vtable(vtable: &Self::VTable) -> &T {
                                let head = vtable.head();
                                <dyn #i_path>::into_vtable(&head.#i_name)
                            }
                        }
                    });
                } else {
                    head_impls.push(quote! {
                        impl<'__private_this> const ::fimo_ffi::ptr::IntoInterface<#i_ty> for dyn #trait_ident + '__private_this {
                            #[inline]
                            fn into_vtable(vtable: &Self::VTable) -> &#i_ty {
                                let head = vtable.head();
                                &head.#i_name
                            }
                        }
                    });
                }

                frozen_optimization = frozen_optimization && i.frozen_token.is_some();
            }

            if !frozen_optimization {
                interface_offsets.insert(
                    0,
                    quote! {
                        #[doc = r"Offset to the data segment of the vtable."]
                        pub __internal_data_offset: usize,
                    },
                );

                interface_new_impl.insert(0, quote! {
                    #[repr(C)]
                    #[allow(missing_debug_implementations)]
                    struct __VT {
                        pub head: #vtable_head_ident,
                        pub data: #vtable_data_ident,
                    }

                    const VT_UNINIT: ::std::mem::MaybeUninit<__VT> = ::std::mem::MaybeUninit::uninit();
                    const VT_UNINIT_PTR: *const __VT = VT_UNINIT.as_ptr();
                    const VT_DATA_PTR: *const #vtable_data_ident = unsafe { ::std::ptr::addr_of!((*VT_UNINIT_PTR).data) };
                    const VT_DATA_OFFSET: usize =
                        unsafe { (VT_DATA_PTR as *const u8).offset_from(VT_UNINIT_PTR as *const u8) as usize };
                });

                interface_new_impl_offset_args
                    .insert(0, quote!(__internal_data_offset: VT_DATA_OFFSET,));

                head_impls.insert(
                    0,
                    quote! {
                        unsafe impl const ::fimo_ffi::ptr::DynamicDataOffset for #vtable_head_ident {
                            #[inline]
                            fn data_offset(&self) -> usize {
                                self.__internal_data_offset
                            }
                        }
                    },
                );
            }

            let doc = format!(
                "Head section of the {} vtable",
                vtable_ident.get_ident().unwrap()
            );
            let dyn_trait_ident = if static_self_bound {
                quote!(dyn #trait_ident + 'static)
            } else {
                quote!(dyn #trait_ident + '__private_inner)
            };

            quote! {
                #[repr(C)]
                #[doc = #doc]
                #[allow(clippy::type_complexity)]
                #trait_vis struct #vtable_head_ident {
                    #[doc = r"Common head of the vtable."]
                    pub __internal_head: ::fimo_ffi::ptr::VTableHead,
                    #(#interface_offsets)*
                    #(#interface_fields)*
                }

                impl #vtable_head_ident {
                    #[doc = r"Minor version of the vtable."]
                    pub const VERSION_MINOR: u32 = 0;

                    #[doc = r"Constructs a new instance of the vtable head section."]
                    pub const fn new<'__private_inner, T>() -> Self
                    where
                        T: #trait_ident + ::fimo_ffi::ptr::ObjectId + '__private_inner,
                    {
                        Self::new_embedded::<T>(0)
                    }

                    #[doc = r"Constructs a new instance of the vtable head section with a custom offset value."]
                    pub const fn new_embedded<'__private_inner, T>(__internal_this_offset: usize,) -> Self
                    where
                        T: #trait_ident + ::fimo_ffi::ptr::ObjectId + '__private_inner,
                    {
                        #(#interface_new_impl)*

                        Self {
                            __internal_head: ::fimo_ffi::ptr::VTableHead::new_embedded_::<'__private_inner, T, #dyn_trait_ident>(
                                __internal_this_offset,
                                Self::VERSION_MINOR,
                            ),
                            #(#interface_new_impl_offset_args)*
                            #(#interface_new_impl_table_args)*
                        }
                    }
                }

                unsafe impl ::fimo_ffi::ptr::ObjMetadataCompatible for #vtable_head_ident {}

                #(#head_impls)*
            }
        };

        // Data section definition.
        let vtable_data = {
            let mut methods = Vec::new();
            let mut method_types = Vec::new();
            let mut method_names = Vec::new();
            for method in self.mappings.iter() {
                let m_ident = &method.method.sig.ident;
                let m_doc = format!("Fn pointer for [`{}::{}`]", trait_item.ident, m_ident);
                let m_ty = &method.vtable_ty;

                methods.push(quote! {
                    #[doc = #m_doc]
                    pub #m_ident: #m_ty,
                });

                method_types.push(m_ty);
                method_names.push(m_ident);
            }

            let doc = format!(
                "Data section of the {} vtable",
                vtable_ident.get_ident().unwrap()
            );

            if methods.is_empty() {
                quote! {
                    #[repr(C)]
                    #[doc = #doc]
                    #trait_vis struct #vtable_data_ident {
                        #[doc = r"Invalid function pointer. Properly aligns the data segment."]
                        pub uninit: ::std::mem::MaybeUninit<fn()>,
                    }

                    impl #vtable_data_ident {
                        #[doc = r"Constructs a new instance of the vtable data section."]
                        pub const fn new(#(#method_names: #method_types),*) -> Self {
                            Self {
                                uninit: ::std::mem::MaybeUninit::uninit()
                            }
                        }
                    }
                }
            } else {
                quote! {
                    #[repr(C)]
                    #[doc = #doc]
                    #[allow(clippy::type_complexity)]
                    #trait_vis struct #vtable_data_ident {
                        #(#methods)*
                    }

                    impl #vtable_data_ident {
                        #[doc = r"Constructs a new instance of the vtable data section."]
                        #[allow(clippy::type_complexity)]
                        #[allow(clippy::too_many_arguments)]
                        pub const fn new(#(#method_names: #method_types),*) -> Self {
                            Self {
                                #(#method_names),*
                            }
                        }
                    }
                }
            }
        };

        // Add vtable shims and internal vtables.
        let new_for_impl = {
            // Generate shims.
            let mut vtable_shims = Vec::new();
            let mut method_idents = Vec::new();
            for method in self.mappings.iter() {
                let abi = &method.abi;
                let ident = &method.method.sig.ident;
                let receiver = &method.receiver;
                let receiver_lt = &method.receiver_lifetime;
                let inputs = &method.inputs;
                let input_names = method
                    .method
                    .sig
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|i| match i {
                        syn::FnArg::Receiver(_) => unreachable!(),
                        syn::FnArg::Typed(t) => t.pat.clone(),
                    })
                    .collect::<Vec<_>>();
                let phantom_parameter = &method.phantom_parameter;
                let output = &method.output;

                let marshaler = &method.marshaler;
                let demarshal_expr = &method.demarshal;

                let mut generics = method.generics.clone();
                generics.params.push(
                    syn::parse_quote!(T: #trait_ident + ::fimo_ffi::ptr::ObjectId + #receiver_lt),
                );

                let (impl_gen, _, _) = generics.split_for_impl();

                method_idents.push(ident.clone());
                vtable_shims.push(quote!{
                    #[allow(clippy::type_complexity)]
                    unsafe #abi fn #ident #impl_gen (__private_this: #receiver, #(#input_names: #inputs),* #phantom_parameter) -> #output {
                        let __private_this = __private_this.cast::<T>();
                        #(#demarshal_expr;)*

                        let __private_res = __private_this.#ident( #(#input_names),* );
                        #marshaler::marshal(__private_res)
                    }
                    #[allow(clippy::useless_transmute)]
                    let #ident = unsafe { std::mem::transmute(#ident::<T> as *const ()) };
                });
            }

            quote! {
                #(#vtable_shims)*

                Self::new(
                    #vtable_head_ident::new_embedded::<T>(offset),
                    #vtable_data_ident::new(
                        #(#method_idents),*
                    ),
                )
            }
        };

        quote! {
            #vtable

            impl #vtable_ident {
                #[doc = "Constructs a new vtable for a type implementing the base trait."]
                #[inline]
                pub const fn new_for<'__private_this, T> () -> Self
                where
                    T: #trait_ident + ::fimo_ffi::ptr::ObjectId + '__private_this,
                {
                    Self::new_for_embedded::<'__private_this, T>(0)
                }

                #[doc = "Constructs a new vtable for a type implementing the base trait with a custom offset."]
                #[inline]
                #[allow(clippy::let_unit_value)]
                pub const fn new_for_embedded<'__private_this, T>(offset: usize) -> Self
                where
                    T: #trait_ident + ::fimo_ffi::ptr::ObjectId + '__private_this,
                {
                    #new_for_impl
                }
            }

            #vtable_head
            #vtable_data
        }
    }

    fn generate_dyn_impl(
        &self,
        trait_item: &syn::ItemTrait,
        static_self_bound: bool,
    ) -> proc_macro2::TokenStream {
        let trait_ident = &trait_item.ident;

        // Add trait implementation
        let trait_impl = {
            let mut methods = Vec::new();
            for m in self.mappings.iter() {
                let ident = &m.method.sig.ident;
                let sig = &m.method.sig;
                let input_names = m
                    .method
                    .sig
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|i| match i {
                        syn::FnArg::Receiver(_) => unreachable!(),
                        syn::FnArg::Typed(t) => t.pat.clone(),
                    })
                    .collect::<Vec<_>>();

                let marshaler = &m.marshaler;
                let marshal_expr = &m.marshal;

                let vtable_ident = &self.vtable;

                let prelude = if m.version == 0 {
                    quote!()
                } else {
                    let version = m.version;
                    let def = m.method.default.as_ref().unwrap();

                    quote! {
                        let __private_metadata = ::fimo_ffi::ptr::metadata(self);

                        if __private_metadata.interface_version_minor() >= #version #def else
                    }
                };

                let phantom_call = m
                    .phantom_parameter
                    .as_ref()
                    .map(|_| quote!(std::marker::PhantomData,));

                let call = if m.optional.is_none() {
                    quote! {
                        {
                            unsafe {
                                let __private_vtable: & #vtable_ident = ::fimo_ffi::ptr::metadata(self).super_vtable();
                                let __private_vtable_data = __private_vtable.data();
                                let __private_this = ::fimo_ffi::ptr::ToPtr::to_ptr(self);

                                #(#marshal_expr;)*

                                let __private_res = (__private_vtable_data.#ident)(__private_this, #(#input_names),* #phantom_call);
                                #marshaler::demarshal(__private_res)
                            }
                        }
                    }
                } else if let Some(optional) = &m.optional {
                    if let Some(replace) = optional {
                        quote! {
                            {
                                let __private_vtable: & #vtable_ident = ::fimo_ffi::ptr::metadata(self).super_vtable();
                                let __private_vtable_data = __private_vtable.data();
                                if let Some(__private_ptr) = __private_vtable_data.#ident {
                                    unsafe {
                                        let __private_this = ::fimo_ffi::ptr::ToPtr::to_ptr(self);
                                        #(#marshal_expr;)*

                                        let __private_res = (__private_ptr)(__private_this, #(#input_names),* #phantom_call);
                                        #marshaler::marshal(__private_res)
                                    }
                                } else {
                                    #replace(self, #(#input_names),*)
                                }
                            }
                        }
                    } else {
                        let def = m.method.default.as_ref().unwrap();
                        quote! {
                            {
                                let __private_vtable: & #vtable_ident = ::fimo_ffi::ptr::metadata(self).super_vtable();
                                let __private_vtable_data = __private_vtable.data();
                                if let Some(__private_ptr) = __private_vtable_data.#ident {
                                    unsafe {
                                        let __private_this = ::fimo_ffi::ptr::ToPtr::to_ptr(self);
                                        #(#marshal_expr;)*

                                        let __private_res = (__private_ptr)(__private_this, #(#input_names),* #phantom_call);
                                        #marshaler::marshal(__private_res)
                                    }
                                } else {
                                    #def
                                }
                            }
                        }
                    }
                } else {
                    let def = m.method.default.as_ref().unwrap();

                    quote! {
                        #def
                    }
                };

                methods.push(quote! {
                    #[inline]
                    #[allow(clippy::let_unit_value)]
                    #sig {
                        #prelude
                        #call
                    }
                })
            }

            quote! {
                #(#methods)*
            }
        };

        let bounds = &trait_item.supertraits;
        let dyn_trait_ident = if static_self_bound {
            quote!(dyn #trait_ident + 'static)
        } else {
            quote!(dyn #trait_ident + '__private_inner)
        };

        quote! {
            impl<'__private_inner, T> #trait_ident for ::fimo_ffi::ptr::DynObj<T>
            where
                T: #trait_ident + ?Sized + '__private_inner,
                T: ::fimo_ffi::ptr::CastInto<#dyn_trait_ident>,
                ::fimo_ffi::ptr::DynObj<T>: #bounds,
            {
                #trait_impl
            }
        }
    }
}

pub fn interface_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as ItemInterface);
    let mut item = input.construct_trait();

    let InterfaceArgs {
        version,
        vtable,
        no_dyn_impl,
        abi,
        marshal,
        uuid,
    } = match input.get_input_args() {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    // Extract the UUID of the interface.
    // If none was provided, we use the reserved 0 UUID.
    let uuid = match uuid {
        Some(x) => match x {
            Ok(x) => *x.as_bytes(),
            Err(e) => return TokenStream::from(e.write_errors()),
        },
        None => [0; 16],
    };

    // Generate new vtable name if none was passed.
    let vtable = match vtable {
        Some(x) => x,
        None => quote::format_ident!("{}VTable", item.ident).into(),
    };

    // The initial implementation does not support generics or where clauses.
    assert!(item.generics.params.is_empty() && item.generics.where_clause.is_none());

    let constraints = match input.get_constraints() {
        Ok(c) => c,
        Err(e) => {
            let e = e.to_compile_error();

            return quote! {
                #item
                #e
            }
            .into();
        }
    };

    let context = match InterfaceContext::new(&mut item, version, &vtable, abi, marshal) {
        Ok(c) => c,
        Err(e) => {
            let e = e.write_errors();

            return quote! {
                #item
                #e
            }
            .into();
        }
    };

    let trait_ident = item.ident.clone();
    let mut dyn_trait_idents: Vec<syn::Type> = Vec::new();

    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + '__private_inner));
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + Send + '__private_inner));
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + Sync + '__private_inner));
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + Unpin + '__private_inner));
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + Send + Sync + '__private_inner));
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + Send + Unpin + '__private_inner));
    dyn_trait_idents.push(syn::parse_quote!(dyn #trait_ident + Sync + Unpin + '__private_inner));
    dyn_trait_idents
        .push(syn::parse_quote!(dyn #trait_ident + Send + Sync + Unpin + '__private_inner));

    // We check if the interface is frozen by looking for the frozen keyword.
    let is_frozen = input.frozen_token.is_some();

    // Base type for which we need to implement some additional traits.
    let dyn_trait_base = dyn_trait_idents[0].clone();

    let interface_version_major = context.version_major;
    let interface_version_minor = context.version_minor;

    let dyn_impl = if no_dyn_impl {
        quote! {}
    } else {
        context.generate_dyn_impl(&item, input.has_static_bound())
    };

    let vtable_impl =
        context.generate_vtable(&item, input.has_static_bound(), &input.get_interfaces());

    quote! {
        #item

        impl dyn #trait_ident {
            const fn __private_check_constraints() {
                #(#constraints)*
            }
        }

        impl<'__private_inner> ::fimo_ffi::ptr::ObjInterfaceBase for #dyn_trait_base {
            type VTable = #vtable;
            const INTERFACE_ID: ::fimo_ffi::ptr::Uuid = ::fimo_ffi::ptr::Uuid::from_bytes([#(#uuid),*]);
            const IS_FROZEN: bool = #is_frozen;
            const INTERFACE_VERSION_MAJOR: u32 = #interface_version_major;
            const INTERFACE_VERSION_MINOR: u32 = #interface_version_minor;
        }

        #(
            impl<'__private_inner> ::fimo_ffi::ptr::ObjInterface for #dyn_trait_idents {
                type Base = #dyn_trait_base;
            }
        )*

        // We don't support generics, therefore we can mark the interface as being downcast safe.
        unsafe impl<'__private_inner> ::fimo_ffi::ptr::DowncastSafe for #dyn_trait_base {}

        #dyn_impl

        #vtable_impl
    }
    .into()
}
