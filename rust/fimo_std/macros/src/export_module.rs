use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{
    AngleBracketedGenericArguments, AttrStyle, Attribute, Error, Expr, ExprCall, ExprMethodCall,
    GenericArgument, Generics, Ident, ItemConst, Lifetime, Path, Token, Type, Visibility,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token,
};

struct ItemExport {
    attrs: Vec<Attribute>,
    _vis: Visibility,
    _const_token: Token![const],
    _ident: Ident,
    _generics: Generics,
    _colon_token: Token![:],
    ty: Box<Type>,
    _eq_token: Token![=],
    exprs: Vec<BuilderExpr>,
    _semi_token: Token![;],
}

impl ItemExport {
    fn init_expr(&self) -> &BuilderExprInit {
        let init = self.exprs.first().unwrap();
        match init {
            BuilderExpr::Init(expr) => expr,
            _ => unreachable!(),
        }
    }

    fn description(&self) -> Option<&BuilderExprDescription> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Description(expr) => Some(expr),
                _ => None,
            })
            .next_back()
    }

    fn author(&self) -> Option<&BuilderExprAuthor> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Author(expr) => Some(expr),
                _ => None,
            })
            .next_back()
    }

    fn license(&self) -> Option<&BuilderExprLicense> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::License(expr) => Some(expr),
                _ => None,
            })
            .next_back()
    }

    fn parameters(&self) -> Vec<&BuilderExprParameter> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Parameter(expr) => Some(expr),
                _ => None,
            })
            .collect()
    }

    fn resources(&self) -> Vec<&BuilderExprResource> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Resource(expr) => Some(expr),
                _ => None,
            })
            .collect()
    }

    fn namespaces(&self) -> Vec<&BuilderExprNamespace> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Namespace(expr) => Some(expr),
                _ => None,
            })
            .collect()
    }

    fn imports(&self) -> Vec<&BuilderExprImport> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Import(expr) => Some(expr),
                _ => None,
            })
            .collect()
    }

    fn exports(&self) -> Vec<&BuilderExprExport> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::Export(expr) => Some(expr),
                _ => None,
            })
            .collect()
    }

    fn dyn_exports(&self) -> Vec<&BuilderExprDynExport> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::DynExport(expr) => Some(expr),
                _ => None,
            })
            .collect()
    }

    fn state(&self) -> Option<&BuilderExprState> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::State(expr) => Some(expr),
                _ => None,
            })
            .next_back()
    }

    fn on_start_event(&self) -> Option<&BuilderExprOnStartEvent> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::OnStartEvent(expr) => Some(expr),
                _ => None,
            })
            .next_back()
    }

    fn on_stop_event(&self) -> Option<&BuilderExprOnStopEvent> {
        self.exprs
            .iter()
            .filter_map(|expr| match expr {
                BuilderExpr::OnStopEvent(expr) => Some(expr),
                _ => None,
            })
            .next_back()
    }
}

impl ToTokens for ItemExport {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(
            self.attrs
                .iter()
                .filter(|&a| matches!(a.style, AttrStyle::Outer)),
        );
        {
            let ty = &self.ty;
            let exprs = &self.exprs;
            tokens.append_all(quote! {
                const _: ::fimo_std::module::exports::__PrivateBuildToken = #(#exprs)*;
                ::fimo_std::__private_sa::assert_type_eq_all!(#ty, &'static ::fimo_std::module::exports::Export<'static>);
            });
        }

        let init_expr = self.init_expr();
        let path = &init_expr.path;
        let view_ident = &init_expr.path.view_ident;
        let owned_ident = &init_expr.path.owned_ident;
        tokens.append_all(quote! {
            ::fimo_std::__private_sa::assert_type_eq_all!(#path, ::fimo_std::module::exports::Builder::<#view_ident<'_>, #owned_ident>);
        });

        let module_ident = Ident::new(&format!("__private_export_{view_ident}"), Span::call_site());

        let parameters = self.parameters();
        let parameters_table = generate_parameters(&parameters);

        let resources = self.resources();
        let resources_table = generate_resources(&resources);

        let imports = self.imports();
        let import_table = generate_imports(view_ident, owned_ident, &imports);

        let exports = self.exports();
        let dyn_exports = self.dyn_exports();
        let exports_table = generate_exports(view_ident, owned_ident, &exports, &dyn_exports);

        let state = self.state();
        let state_ref = state_ref(state);

        let name = &init_expr.args[0];
        let description = self.description();
        let author = self.author();
        let license = self.license();
        let namespaces = self.namespaces();
        let on_start_event = self.on_start_event();
        let on_stop_event = self.on_stop_event();
        let export = generate_export(
            view_ident,
            name,
            description,
            author,
            license,
            &parameters,
            &resources,
            &namespaces,
            &imports,
            &exports,
            &dyn_exports,
            state,
            on_start_event,
            on_stop_event,
        );

        let export_ident = &self._ident;
        tokens.append_all(quote! {
            mod #module_ident {
                use super::*;

                #parameters_table
                #resources_table
                #import_table
                #exports_table
                #state_ref

                ::fimo_std::instance! {
                    pub(crate) type #view_ident;
                    pub(crate) type #owned_ident;
                    with
                        Parameters = Parameters,
                        Resources = Resources,
                        Imports = Imports,
                        Exports = Exports,
                        State = State,
                }
            }

            const #export_ident: &::fimo_std::module::exports::Export<'_> = #export;

            pub(crate) type #view_ident<'a> = #module_ident::#view_ident<'a>;
            pub(crate) type #owned_ident = #module_ident::#owned_ident;
        });
    }
}

impl Parse for ItemExport {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ItemConst {
            attrs,
            vis,
            const_token,
            ident,
            generics,
            colon_token,
            ty,
            eq_token,
            expr,
            semi_token,
        } = input.parse()?;
        let exprs = BuilderExpr::from_expr(&expr)?;
        match exprs.last() {
            Some(BuilderExpr::Build(_)) => {}
            _ => {
                return Err(Error::new_spanned(
                    &expr,
                    "expected call to the `build` method",
                ));
            }
        }

        Ok(ItemExport {
            attrs,
            _vis: vis,
            _const_token: const_token,
            _ident: ident,
            _generics: generics,
            _colon_token: colon_token,
            ty,
            _eq_token: eq_token,
            exprs,
            _semi_token: semi_token,
        })
    }
}

fn generate_parameters(parameters: &[&BuilderExprParameter]) -> TokenStream {
    let mut fields: Vec<TokenStream> = Default::default();
    let mut accessors: Vec<TokenStream> = Default::default();
    for param in parameters {
        let ident = param.table_name();
        let ty = param.0.generic_type_argument(0).unwrap();
        fields.push(quote! {
            #ident: ::core::pin::Pin<&'static ::fimo_std::module::parameters::Parameter<#ty>>
        });
        accessors.push(quote! {
            pub const fn #ident(&self) -> ::core::pin::Pin<&'_ ::fimo_std::module::parameters::Parameter<#ty>> {
                self.#ident
            }
        });
    }

    quote! {
        #[repr(C)]
        #[doc(hidden)]
        #[derive(Debug)]
        pub struct Parameters {
            #(#fields),*
        }

        impl Parameters {
            #(#accessors)*
        }
    }
}

fn generate_resources(resources: &[&BuilderExprResource]) -> TokenStream {
    let mut fields: Vec<TokenStream> = Default::default();
    let mut accessors: Vec<TokenStream> = Default::default();
    for res in resources {
        let ident = res.table_name();
        fields.push(quote! {
            #ident: ResourceWrapper
        });
        accessors.push(quote! {
            pub const fn #ident(&self) -> &str {
                self.#ident.as_str()
            }
        });
    }

    quote! {
        struct ResourceWrapper(::fimo_std::module::symbols::SliceRef<'static, u8>);
        impl ResourceWrapper {
            const fn as_str(&self) -> &str {
                unsafe {
                    #[cfg(debug_assertions)]
                    {
                        match ::core::str::from_utf8(self.0.as_slice()) {
                            ::core::result::Result::Ok(s) => s,
                            ::core::result::Result::Err(_) => panic!("expected utf8 string")
                        }
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        ::core::str::from_utf8_unchecked(self.0.as_slice())
                    }
                }
            }
        }
        unsafe impl Send for ResourceWrapper {}
        unsafe impl Sync for ResourceWrapper {}
        impl ::core::fmt::Debug for ResourceWrapper {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::result::Result<(), ::core::fmt::Error> {
                ::core::fmt::Debug::fmt(self.as_str(), f)
            }
        }

        #[repr(C)]
        #[doc(hidden)]
        #[derive(Debug)]
        pub struct Resources {
            #(#fields),*
        }

        impl Resources {
            #(#accessors)*
        }
    }
}

fn generate_imports(
    view_ident: &Ident,
    owned_ident: &Ident,
    imports: &[&BuilderExprImport],
) -> TokenStream {
    let symbols = imports
        .iter()
        .map(|import| {
            let ident = import.table_name();
            let symbol_t = import.0.generic_type_argument(0).unwrap();
            (ident, symbol_t)
        })
        .collect::<Vec<_>>();
    generate_vtable(view_ident, owned_ident, true, &symbols)
}

fn generate_exports(
    view_ident: &Ident,
    owned_ident: &Ident,
    exports: &[&BuilderExprExport],
    dyn_exports: &[&BuilderExprDynExport],
) -> TokenStream {
    let symbols = exports
        .iter()
        .map(|export| {
            let ident = export.table_name();
            let symbol_t = export.0.generic_type_argument(0).unwrap();
            (ident, symbol_t)
        })
        .chain(dyn_exports.iter().map(|export| {
            let ident = export.table_name();
            let symbol_t = export.0.generic_type_argument(0).unwrap();
            (ident, symbol_t)
        }))
        .collect::<Vec<_>>();
    generate_vtable(view_ident, owned_ident, false, &symbols)
}

fn generate_vtable(
    view_ident: &Ident,
    owned_ident: &Ident,
    is_imports: bool,
    symbols: &[(Ident, &Type)],
) -> TokenStream {
    let mut fields: Vec<TokenStream> = Default::default();
    for (ident, ty) in symbols {
        fields.push(quote! {
            #ident: ::fimo_std::module::symbols::SymbolRef<'static, #ty>
        });
    }

    let mut accessors: Vec<TokenStream> = Default::default();
    for (ident, ty) in symbols {
        accessors.push(quote! {
            pub const fn #ident(&self) -> ::fimo_std::module::symbols::SymbolRef<'_, #ty> {
                self.#ident
            }
        });
    }

    let vtable_name = if is_imports {
        quote::format_ident!("Imports")
    } else {
        quote::format_ident!("Exports")
    };
    let vtable_accessor = if is_imports {
        quote::format_ident!("imports")
    } else {
        quote::format_ident!("exports")
    };

    let mut provider_impls: Vec<TokenStream> = Default::default();
    for (ident, ty) in symbols {
        provider_impls.push(quote! {
            impl ::fimo_std::module::symbols::SymbolProvider<#ty> for ::core::pin::Pin<&'_ #view_ident<'_>> {
                fn access<'a>(self) -> ::fimo_std::module::symbols::SymbolRef<'a, #ty>
                where
                    Self: 'a
                {
                    let vtable = self.#vtable_accessor();
                    vtable.#ident()
                }
            }

            impl ::fimo_std::module::symbols::SymbolProvider<#ty> for &'_ #owned_ident {
                fn access<'a>(self) -> ::fimo_std::module::symbols::SymbolRef<'a, #ty>
                where
                    Self: 'a
                {
                    let vtable = self.#vtable_accessor();
                    vtable.#ident()
                }
            }
        });
    }

    quote! {
        #[repr(C)]
        #[doc(hidden)]
        #[derive(Debug)]
        pub struct #vtable_name {
            #(#fields),*
        }

        impl #vtable_name {
            #(#accessors)*
        }

        #(#provider_impls)*
    }
}

fn state_ref(state: Option<&BuilderExprState>) -> TokenStream {
    if let Some(x) = state {
        let t = x.0.generic_type_argument(0).unwrap();
        quote! {pub type State = #t;}
    } else {
        quote! {pub type State = ();}
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_export(
    view_ident: &Ident,
    name: &Expr,
    description: Option<&BuilderExprDescription>,
    author: Option<&BuilderExprAuthor>,
    license: Option<&BuilderExprLicense>,
    parameters: &[&BuilderExprParameter],
    resources: &[&BuilderExprResource],
    namespaces: &[&BuilderExprNamespace],
    imports: &[&BuilderExprImport],
    exports: &[&BuilderExprExport],
    dyn_exports: &[&BuilderExprDynExport],
    state: Option<&BuilderExprState>,
    on_start_event: Option<&BuilderExprOnStartEvent>,
    on_stop_event: Option<&BuilderExprOnStopEvent>,
) -> TokenStream {
    let description = description.map_or(quote!(None), |x| {
        let args = &x.0.args;
        quote!(Some(#args))
    });
    let author = author.map_or(quote!(None), |x| {
        let args = &x.0.args;
        quote!(Some(#args))
    });
    let license = license.map_or(quote!(None), |x| {
        let args = &x.0.args;
        quote!(Some(#args))
    });

    let parameters = parameters.iter().map(|&param| {
        let ty = param.0.generic_type_argument(0).unwrap();
        let name = &param.0.args[1];
        let default_value = &param.0.args[2];
        let read_group = &param.0.args[3];
        let write_group = &param.0.args[4];
        let read = &param.0.args[5];
        let write = &param.0.args[6];
        quote! {
            const {
                let mut p = ::fimo_std::module::exports::Parameter::new::<#ty>(#default_value, #name);
                if let Some(x) = #read_group {
                    p = p.with_read_group(x);
                }
                if let Some(x) = #write_group {
                    p = p.with_write_group(x);
                }

                type Repr = <#ty as ::fimo_std::module::parameters::ParameterCast>::Repr;
                const READ: Option<fn(::fimo_std::module::parameters::ParameterData<'_, Repr>) -> Repr> = #read;
                if READ.is_some() {
                    unsafe extern "C" fn __private_read(parameter: ::fimo_std::module::parameters::ParameterData<'_, ()>, value: ::core::ptr::NonNull<()>) {
                        unsafe {
                            type Repr = <#ty as ::fimo_std::module::parameters::ParameterCast>::Repr;
                            let parameter = ::core::mem::transmute::<
                                ::fimo_std::module::parameters::ParameterData<'_, ()>,
                                ::fimo_std::module::parameters::ParameterData<'_, Repr>,
                            >(parameter);
                            let value = value.cast::<Repr>();

                            let f = READ.unwrap_unchecked();
                            value.write(f(parameter));
                        }
                    }
                    let __private_read = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_read as _) };
                    p = p.with_read(__private_read);
                }

                const WRITE: Option<fn(::fimo_std::module::parameters::ParameterData<'_, Repr>, Repr)> = #write;
                if WRITE.is_some() {
                    unsafe extern "C" fn __private_write(parameter: ::fimo_std::module::parameters::ParameterData<'_, ()>, value: ::fimo_std::utils::ConstNonNull<()>) {
                        unsafe {
                            type Repr = <#ty as ::fimo_std::module::parameters::ParameterCast>::Repr;
                            let parameter = ::core::mem::transmute::<
                                ::fimo_std::module::parameters::ParameterData<'_, ()>,
                                ::fimo_std::module::parameters::ParameterData<'_, Repr>,
                            >(parameter);
                            let value = value.cast::<Repr>();

                            let f = WRITE.unwrap_unchecked();
                            f(parameter, *value.as_ref());
                        }
                    }
                    let __private_write = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_write as _) };
                    p = p.with_write(__private_write);
                }
                p
            }
        }
    }).collect::<Vec<_>>();

    let resources = resources
        .iter()
        .map(|&res| {
            let path = &res.0.args[1];
            quote! {
                const {
                    ::fimo_std::module::exports::Resource::new(#path)
                }
            }
        })
        .collect::<Vec<_>>();

    let num_namespaces = namespaces.len() + imports.len();
    let namespaces = namespaces
        .iter()
        .map(|&ns| {
            let ns = &ns.0.args[0];
            quote! {
                insert_in_ns(&mut namespaces, ::fimo_std::module::exports::Namespace::new(#ns));
            }
        })
        .chain(imports.iter().map(|imp| {
            let t = imp.0.generic_type_argument(0).unwrap();
            quote! {
                insert_in_ns(
                    &mut namespaces,
                    ::fimo_std::module::exports::Namespace::new(
                        <#t as ::fimo_std::module::symbols::SymbolInfo>::NAMESPACE
                    )
                );
            }
        }))
        .collect::<Vec<_>>();

    let imports = imports
        .iter()
        .map(|&imp| {
            let t = imp.0.generic_type_argument(0).unwrap();
            quote! {
                const {
                    let name = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAME;
                    let namespace = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAMESPACE;
                    let version = <#t as ::fimo_std::module::symbols::SymbolInfo>::VERSION;
                    ::fimo_std::module::exports::SymbolImport::new(
                        version,
                        name
                    ).with_namespace(namespace)
                }
            }
        })
        .collect::<Vec<_>>();

    let exports = exports
        .iter()
        .map(|&exp| {
            let t = exp.0.generic_type_argument(0).unwrap();
            let linkage = &exp.0.args[1];
            let value = &exp.0.args[2];
            quote! {
                const {
                    type T = <#t as ::fimo_std::module::symbols::SymbolInfo>::Type;
                    let name = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAME;
                    let namespace = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAMESPACE;
                    let version = <#t as ::fimo_std::module::symbols::SymbolInfo>::VERSION;
                    let linkage = const { #linkage };
                    ::fimo_std::module::exports::SymbolExport::new::<T>(
                        #value,
                        version,
                        name,
                        linkage,
                    ).with_namespace(namespace)
                }
            }
        })
        .collect::<Vec<_>>();

    let dyn_exports = dyn_exports
        .iter()
        .map(|&exp| {
            let t = exp.0.generic_type_argument(0).unwrap();
            let linkage = &exp.0.args[1];
            let init = &exp.0.args[2];
            let deinit = &exp.0.args[3];
            quote! {
                const {
                    unsafe extern "C" fn __private_constructor(
                        instance: ::core::pin::Pin<& ::fimo_std::module::instance::OpaqueInstanceView<'_>>,
                    ) -> ::fimo_std::r#async::EnqueuedFuture<
                            ::fimo_std::r#async::Fallible<
                                ::core::ptr::NonNull<()>,
                                dyn ::fimo_std::module::symbols::Share,
                            >
                        > {
                        let f = const { #init };
                        unsafe {
                            type T = <#t as ::fimo_std::module::symbols::SymbolInfo>::Type;
                            let instance: ::core::pin::Pin<
                                &::fimo_std::module::instance::Stage1InstanceView<'_, #view_ident<'_>>
                            >   = ::core::mem::transmute(instance);
                            let fut = f(instance);
                            let fut = async move {
                                ::fimo_std::r#async::Fallible::new_result(
                                    fut.await
                                        .map_err(<::fimo_std::error::AnyError>::new)
                                        .map(|x| {
                                            let opaque = unsafe{ <T as ::fimo_std::module::symbols::SymbolPointer>::ptr_from_target(x) };
                                            let opaque = opaque.as_ptr().cast_mut();
                                            ::core::ptr::NonNull::new(opaque).expect("null pointers are not allowed")
                                        })
                                )
                            };
                            unsafe {
                                ::fimo_std::r#async::Future::new(fut)
                                    .enqueue_unchecked()
                                    .expect("could not enqueue future")
                            }
                        }
                    }
                    unsafe extern "C" fn __private_destructor(
                        instance: ::core::pin::Pin<& ::fimo_std::module::instance::OpaqueInstanceView<'_>>,
                        symbol: ::core::ptr::NonNull<()>,
                    ) {
                        let f = const { #deinit };
                        let instance: ::core::pin::Pin<
                            &::fimo_std::module::instance::Stage1InstanceView<'_, #view_ident<'_>>
                        >   = unsafe{ ::core::mem::transmute(instance) };
                        type T = <#t as ::fimo_std::module::symbols::SymbolInfo>::Type;
                        let symbol = ::fimo_std::utils::ConstNonNull::new(symbol.as_ptr()).expect("should not be null");
                        let symbol = unsafe{ <T as ::fimo_std::module::symbols::SymbolPointer>::target_from_ptr(symbol) };
                        f(instance, symbol)
                    }
                    let __private_constructor = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_constructor as _) };
                    let __private_destructor = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_destructor as _) };

                    let name = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAME;
                    let namespace = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAMESPACE;
                    let version = <#t as ::fimo_std::module::symbols::SymbolInfo>::VERSION;
                    let linkage = const { #linkage };
                    unsafe {
                        ::fimo_std::module::exports::DynamicSymbolExport::new(
                            __private_constructor,
                            __private_destructor,
                            version,
                            name,
                            linkage,
                        ).with_namespace(namespace)
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let mut modifiers: Vec<TokenStream> = Vec::new();
    if let Some(state) = state {
        let init = &state.0.args[0];
        let deinit = &state.0.args[1];
        modifiers.push(quote! {
            {
                unsafe extern "C" fn __private_init(
                    instance: ::core::pin::Pin<& ::fimo_std::module::instance::OpaqueInstanceView<'_>>,
                    set: ::fimo_std::module::loading_set::LoadingSetView<'_>,
                ) -> ::fimo_std::r#async::EnqueuedFuture<
                        ::fimo_std::r#async::Fallible<
                            ::core::option::Option<::core::ptr::NonNull<()>>,
                            dyn ::fimo_std::module::symbols::Share,
                        >
                    >
                {
                    let f = const { #init };
                    unsafe {
                        let instance: ::core::pin::Pin<
                            &::fimo_std::module::instance::Stage0InstanceView<'_, #view_ident<'_>>
                        >   = ::core::mem::transmute(instance);
                        let fut = f(instance, set);
                        let fut = async move {
                            ::fimo_std::r#async::Fallible::new_result(
                                fut.await
                                    .map_err(<::fimo_std::error::AnyError>::new)
                                    .map(|x| ::core::option::Option::Some(x.cast()))
                            )
                        };
                        unsafe {
                            ::fimo_std::r#async::Future::new(fut)
                                .enqueue_unchecked()
                                .expect("could not enqueue future")
                        }
                    }
                }
                unsafe extern "C" fn __private_deinit(
                    instance: ::core::pin::Pin<& ::fimo_std::module::instance::OpaqueInstanceView<'_>>,
                    state: ::core::option::Option<::core::ptr::NonNull<()>>,
                ) {
                    let f = const { #deinit };
                    unsafe {
                        let instance: ::core::pin::Pin<
                            &::fimo_std::module::instance::Stage0InstanceView<'_, #view_ident<'_>>
                        >   = ::core::mem::transmute(instance);
                        let state = state.expect("expected a non-null pointer").cast();
                        f(instance, state)
                    }
                }
                let modifier = &const {
                    let __private_init = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_init as _) };
                    let __private_deinit = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_deinit as _) };
                    unsafe { ::fimo_std::module::exports::InstanceStateModifier::new(__private_init, __private_deinit) }
                };
                ::fimo_std::module::exports::Modifier::InstanceState(modifier)
            }
        });
    }
    if let Some(event) = on_start_event {
        let on_event = &event.0.args[0];
        modifiers.push(quote! {
            {
                unsafe extern "C" fn __private_on_event(
                    instance: ::core::pin::Pin<& ::fimo_std::module::instance::OpaqueInstanceView<'_>>,
                ) -> ::fimo_std::r#async::EnqueuedFuture<
                        ::fimo_std::r#async::Fallible<
                            (),
                            dyn ::fimo_std::module::symbols::Share,
                        >
                    >
                {
                    let f = const { #on_event };
                    unsafe {
                        let instance: ::core::pin::Pin<&#view_ident<'_>> = ::core::mem::transmute(instance);
                        let fut = f(instance);
                        let fut = async move {
                            ::fimo_std::r#async::Fallible::new_result(
                                fut.await
                                    .map_err(<::fimo_std::error::AnyError>::new)
                            )
                        };
                        unsafe {
                            ::fimo_std::r#async::Future::new(fut)
                                .enqueue_unchecked()
                                .expect("could not enqueue future")
                        }
                    }
                }
                let modifier = &const {
                    let __private_on_event = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_on_event as _) };
                    unsafe { ::fimo_std::module::exports::StartEventModifier::new(__private_on_event) }
                };
                ::fimo_std::module::exports::Modifier::StartEvent(modifier)
            }
        });
    }
    if let Some(event) = on_stop_event {
        let on_event = &event.0.args[0];
        modifiers.push(quote! {
            {
                unsafe extern "C" fn __private_on_event(
                    instance: ::core::pin::Pin<& ::fimo_std::module::instance::OpaqueInstanceView<'_>>,
                ) {
                    let f = const { #on_event };
                    unsafe {
                        let instance: ::core::pin::Pin<&#view_ident<'_>> = ::core::mem::transmute(instance);
                        f(instance)
                    }
                }
                let modifier = &const {
                    let __private_on_event = unsafe { ::fimo_std::module::symbols::AssertSharable::new(__private_on_event as _) };
                    unsafe { ::fimo_std::module::exports::StopEventModifier::new(__private_on_event) }
                };
                ::fimo_std::module::exports::Modifier::StopEvent(modifier)
            }
        });
    }

    quote! {
        {
            const PARAMETERS: &[::fimo_std::module::exports::Parameter<'static>] = &[
                    #(#parameters),*
            ];
            const RESOURCES: &[::fimo_std::module::exports::Resource<'static>] = &[
                #(#resources),*
            ];
            const NAMESPACES: &[::fimo_std::module::exports::Namespace<'static>] = {
                const fn insert_in_ns(
                    namespaces: &mut [::fimo_std::module::exports::Namespace<'static>; #num_namespaces],
                    value: ::fimo_std::module::exports::Namespace<'static>,
                ) {
                    if value.name().is_empty() { return; }

                    // Check that it is not a duplicate.
                    let mut i = 0usize;
                    while i < namespaces.len() {
                        let ns = namespaces[i];
                        let name_bytes = ns.name().to_bytes();
                        let value_bytes = value.name().to_bytes();

                        // Duplicate.
                        if name_bytes.len() == value_bytes.len() {
                            let mut j = 0usize;
                            while j < name_bytes.len() {
                                let l = name_bytes[j];
                                let r = value_bytes[j];
                                if l != r {
                                    break;
                                }
                                j += 1;
                            }

                            if j == name_bytes.len() {
                                return;
                            }
                        }

                        // Found empty slot.
                        if name_bytes.is_empty() {
                            namespaces[i] = value;
                            return;
                        }
                        i += 1;
                    }

                    unreachable!();
                }

                const NS: [::fimo_std::module::exports::Namespace<'static>; #num_namespaces] = const {
                    let mut namespaces = [::fimo_std::module::exports::Namespace::new(c""); #num_namespaces];
                    #(#namespaces)*
                    namespaces
                };

                // Find the first global namespace.
                let mut i = 0usize;
                let slice = NS.as_slice();
                while i < slice.len() {
                    let ns = slice[i];
                    if ns.name().is_empty() {
                        break;
                    }
                    i += 1;
                }
                slice.split_at(i).0
            };
            const IMPORTS: &[::fimo_std::module::exports::SymbolImport<'static>] = &[
                #(#imports),*
            ];
            const EXPORTS: &[::fimo_std::module::exports::SymbolExport<'static>] = &[
                #(#exports),*
            ];
            const DYN_EXPORTS: &[::fimo_std::module::exports::DynamicSymbolExport<'static>] = &[
                #(#dyn_exports),*
            ];
            const MODIFIERS: &[::fimo_std::module::exports::Modifier<'static>] = &[
                #(#modifiers),*
            ];

            const EXPORT: ::fimo_std::module::exports::Export<'_> = unsafe {
                ::fimo_std::module::exports::Export::__new_private(
                    #name,
                    #description,
                    #author,
                    #license,
                    PARAMETERS,
                    RESOURCES,
                    NAMESPACES,
                    IMPORTS,
                    EXPORTS,
                    DYN_EXPORTS,
                    MODIFIERS,
                )
            };

            #[used(linker)]
            #[cfg_attr(windows, unsafe(link_section = "fi_mod$u"))]
            #[cfg_attr(
                all(unix, target_vendor = "apple"),
                unsafe(link_section = "__DATA,fimo_module")
            )]
            #[cfg_attr(all(unix, not(target_vendor = "apple")), unsafe(link_section = "fimo_module"))]
            static EXPORT_STATIC: &::fimo_std::module::exports::Export<'_> = &EXPORT;

            EXPORT_STATIC
        }
    }
}

enum BuilderExpr {
    Init(BuilderExprInit),
    Description(BuilderExprDescription),
    Author(BuilderExprAuthor),
    License(BuilderExprLicense),
    Parameter(BuilderExprParameter),
    Resource(BuilderExprResource),
    Namespace(BuilderExprNamespace),
    Import(BuilderExprImport),
    Export(BuilderExprExport),
    DynExport(BuilderExprDynExport),
    State(BuilderExprState),
    OnStartEvent(BuilderExprOnStartEvent),
    OnStopEvent(BuilderExprOnStopEvent),
    Build(BuilderExprBuild),
}

impl BuilderExpr {
    fn from_expr(expr: &Expr) -> syn::Result<Vec<Self>> {
        let expr = match expr {
            Expr::MethodCall(expr) => expr,
            Expr::Call(expr) => {
                let expr = BuilderExprInit::from_expr(expr)?;
                return Ok(vec![Self::Init(expr)]);
            }
            _ => return Err(Error::new_spanned(expr, "expected method call")),
        };

        let method_name = expr.method.to_string();
        match &*method_name {
            "with_description" => BuilderExprDescription::from_expr(expr),
            "with_author" => BuilderExprAuthor::from_expr(expr),
            "with_license" => BuilderExprLicense::from_expr(expr),
            "with_parameter" => BuilderExprParameter::from_expr(expr),
            "with_resource" => BuilderExprResource::from_expr(expr),
            "with_namespace" => BuilderExprNamespace::from_expr(expr),
            "with_import" => BuilderExprImport::from_expr(expr),
            "with_export" => BuilderExprExport::from_expr(expr),
            "with_dynamic_export" => BuilderExprDynExport::from_expr(expr),
            "with_state" => BuilderExprState::from_expr(expr),
            "with_on_start_event" => BuilderExprOnStartEvent::from_expr(expr),
            "with_on_stop_event" => BuilderExprOnStopEvent::from_expr(expr),
            "build" => BuilderExprBuild::from_expr(expr),
            _ => Err(Error::new_spanned(&expr.method, "unknown builder method")),
        }
    }
}

impl ToTokens for BuilderExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            BuilderExpr::Init(expr) => expr.to_tokens(tokens),
            BuilderExpr::Description(expr) => expr.to_tokens(tokens),
            BuilderExpr::Author(expr) => expr.to_tokens(tokens),
            BuilderExpr::License(expr) => expr.to_tokens(tokens),
            BuilderExpr::Parameter(expr) => expr.to_tokens(tokens),
            BuilderExpr::Resource(expr) => expr.to_tokens(tokens),
            BuilderExpr::Namespace(expr) => expr.to_tokens(tokens),
            BuilderExpr::Import(expr) => expr.to_tokens(tokens),
            BuilderExpr::Export(expr) => expr.to_tokens(tokens),
            BuilderExpr::DynExport(expr) => expr.to_tokens(tokens),
            BuilderExpr::State(expr) => expr.to_tokens(tokens),
            BuilderExpr::OnStartEvent(expr) => expr.to_tokens(tokens),
            BuilderExpr::OnStopEvent(expr) => expr.to_tokens(tokens),
            BuilderExpr::Build(expr) => expr.to_tokens(tokens),
        }
    }
}

struct BuilderExprInit {
    path: BuilderPath,
    separator: Token![::],
    function: Ident,
    paren_token: token::Paren,
    args: Punctuated<Expr, Token![,]>,
}

impl BuilderExprInit {
    fn from_expr(expr: &ExprCall) -> syn::Result<Self> {
        if !expr.attrs.is_empty() {
            return Err(Error::new_spanned(
                &expr.attrs[0],
                "attributes not supported",
            ));
        }
        let path = match &*expr.func {
            Expr::Path(x) => x,
            _ => return Err(Error::new_spanned(&*expr.func, "expected path")),
        };
        if !path.attrs.is_empty() || path.qself.is_some() {
            return Err(Error::new_spanned(path, "invalid path"));
        }
        let mut path = path.path.clone();

        let function_segment = match path.segments.pop() {
            Some(x) => x.into_value(),
            None => return Err(Error::new_spanned(path, "invalid builder path")),
        };
        let separator = match path.segments.pop_punct() {
            Some(x) => x,
            None => return Err(Error::new_spanned(path, "invalid builder path")),
        };

        let path = BuilderPath::from_path(path)?;
        let function = function_segment.ident;
        if function != "new" {
            return Err(Error::new_spanned(
                function,
                "expected call to the `new` function",
            ));
        }
        let paren_token = expr.paren_token;
        let args = expr.args.clone();

        Ok(Self {
            path,
            separator,
            function,
            paren_token,
            args,
        })
    }
}

impl ToTokens for BuilderExprInit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.separator.to_tokens(tokens);
        self.function.to_tokens(tokens);
        self.paren_token.surround(tokens, |tokens| {
            self.args.to_tokens(tokens);
        });
    }
}

struct BuilderPath {
    path: Path,
    ident: Ident,
    separator_token: Token![::],
    lt_token: Token![<],
    view_ident: Ident,
    view_lt_token: Token![<],
    view_lifetime: Lifetime,
    view_gt_token: Token![>],
    comma_token: Token![,],
    owned_ident: Ident,
    trailing_comma_token: Option<Token![,]>,
    gt_token: Token![>],
}

impl BuilderPath {
    fn from_path(mut path: Path) -> syn::Result<Self> {
        let builder_segment = match path.segments.pop() {
            Some(x) => x.into_value(),
            None => return Err(Error::new_spanned(path, "invalid builder path")),
        };
        let ident = builder_segment.ident;
        let mut builder_generics = match builder_segment.arguments {
            syn::PathArguments::AngleBracketed(x) => x,
            _ => {
                return Err(Error::new_spanned(
                    builder_segment.arguments,
                    "expected generic arguments",
                ));
            }
        };
        let separator_token = match builder_generics.colon2_token {
            Some(x) => x,
            None => {
                return Err(Error::new_spanned(
                    builder_generics.colon2_token,
                    "expected `::`",
                ));
            }
        };
        let lt_token = builder_generics.lt_token;
        let gt_token = builder_generics.gt_token;

        if builder_generics.args.len() != 2 {
            return Err(Error::new_spanned(
                builder_generics.args,
                "expected 2 generic arguments",
            ));
        }
        let (owned_arg, trailing_comma_token) = builder_generics.args.pop().unwrap().into_tuple();
        let (view_arg, comma_token) = builder_generics.args.pop().unwrap().into_tuple();
        let comma_token = comma_token.unwrap();

        let view_arg = match view_arg {
            syn::GenericArgument::Type(x) => x,
            _ => {
                return Err(Error::new_spanned(view_arg, "expected generic type"));
            }
        };
        let mut view_arg = match view_arg {
            Type::Path(x) => x,
            _ => return Err(Error::new_spanned(view_arg, "expected path")),
        };
        if view_arg.path.leading_colon.is_some()
            || view_arg.path.segments.len() != 1
            || view_arg.qself.is_some()
        {
            return Err(Error::new_spanned(view_arg, "invalid path"));
        }
        let view_arg = view_arg.path.segments.pop().unwrap().into_value();
        let view_ident = view_arg.ident;

        let mut view_arg = match view_arg.arguments {
            syn::PathArguments::AngleBracketed(x) => x,
            _ => {
                return Err(Error::new_spanned(
                    view_arg.arguments,
                    "expected generic argument",
                ));
            }
        };
        let view_lt_token = view_arg.lt_token;
        let view_gt_token = view_arg.gt_token;
        if view_arg.args.len() != 1 {
            return Err(Error::new_spanned(view_arg, "expected 1 argument"));
        }
        let view_lifetime = view_arg.args.pop().unwrap().into_value();
        let view_lifetime = match view_lifetime {
            syn::GenericArgument::Lifetime(x) => x,
            _ => return Err(Error::new_spanned(view_lifetime, "expected lifetime")),
        };
        if view_lifetime.ident != "_" {
            return Err(Error::new_spanned(
                view_lifetime,
                "expected inferred lifetime `'_`",
            ));
        }

        let owned_arg = match owned_arg {
            syn::GenericArgument::Type(x) => x,
            _ => {
                return Err(Error::new_spanned(owned_arg, "expected generic type"));
            }
        };
        let mut owned_arg = match owned_arg {
            Type::Path(x) => x,
            _ => return Err(Error::new_spanned(owned_arg, "expected path")),
        };
        if owned_arg.path.leading_colon.is_some()
            || owned_arg.path.segments.len() != 1
            || owned_arg.qself.is_some()
        {
            return Err(Error::new_spanned(owned_arg, "invalid path"));
        }
        let owned_arg = owned_arg.path.segments.pop().unwrap().into_value();
        if !owned_arg.arguments.is_empty() {
            return Err(Error::new_spanned(
                owned_arg.arguments,
                "no arguments allowed",
            ));
        }
        let owned_ident = owned_arg.ident;

        Ok(Self {
            path,
            ident,
            separator_token,
            lt_token,
            view_ident,
            view_lt_token,
            view_lifetime,
            view_gt_token,
            comma_token,
            owned_ident,
            trailing_comma_token,
            gt_token,
        })
    }
}

impl ToTokens for BuilderPath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.separator_token.to_tokens(tokens);
        self.lt_token.to_tokens(tokens);
        self.view_ident.to_tokens(tokens);
        self.view_lt_token.to_tokens(tokens);
        self.view_lifetime.to_tokens(tokens);
        self.view_gt_token.to_tokens(tokens);
        self.comma_token.to_tokens(tokens);
        self.owned_ident.to_tokens(tokens);
        if let Some(trailing_comma_token) = &self.trailing_comma_token {
            trailing_comma_token.to_tokens(tokens);
        }
        self.gt_token.to_tokens(tokens);
    }
}

macro_rules! builder_expr {
    ($(fn $method:ident ($(<inline $table_name:ident>,)? $name:ident, $builder:path $(, $type_args:literal)?);)*) => {
        $(
            builder_expr!(op $method ($($table_name,)? $name, $builder $(, $type_args)?));
        )*
    };
    (op $method:ident ($name:ident, $builder:path $(, $type_args:literal)?)) => {
        struct $name(BuilderExprInner);
        impl $name {
            fn from_expr(expr: &ExprMethodCall) -> syn::Result<Vec<BuilderExpr>> {
                assert_eq!(expr.method.to_string(), std::stringify!($method));
                let this = Self(BuilderExprInner::new(expr, false $(|| $type_args)?)?);
                let op = $builder(this);

                let mut ops = BuilderExpr::from_expr(&expr.receiver)?;
                ops.push(op);
                Ok(ops)
            }
        }
        impl ToTokens for $name {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                if std::stringify!($method) == "build" {
                    tokens.append_all(quote!(.__private_build()));
                    return;
                }

                self.0.to_tokens(tokens);
            }
        }
    };
    (op $method:ident (table_name, $name:ident, $builder:path $(, $type_args:literal)?)) => {
        struct $name(BuilderExprInner);
        impl $name {
            fn from_expr(expr: &ExprMethodCall) -> syn::Result<Vec<BuilderExpr>> {
                assert_eq!(expr.method.to_string(), std::stringify!($method));
                let table_name = match expr.args.first() {
                    Some(expr) => expr,
                    None => return Err(Error::new_spanned(&expr.args, "missing arguments")),
                };
                match table_name {
                    Expr::Lit(expr) => {
                        if let Some(attr) = expr.attrs.first() {
                            return Err(Error::new_spanned(attr, "attributes not supported"));
                        }
                        match &expr.lit {
                            syn::Lit::Str(lit) => _ = lit.parse::<Ident>()?,
                            _ => return Err(Error::new_spanned(table_name, "expected a string literal")),
                        }
                    }
                    _ => return Err(Error::new_spanned(table_name, "expected a string literal")),
                }

                let this = Self(BuilderExprInner::new(expr, false $(|| $type_args)?)?);
                let op = $builder(this);

                let mut ops = BuilderExpr::from_expr(&expr.receiver)?;
                ops.push(op);
                Ok(ops)
            }
            fn table_name(&self) -> Ident {
                let table_name = self.0.args.first().unwrap();
                match table_name {
                    Expr::Lit(expr) => {
                        match &expr.lit {
                            syn::Lit::Str(lit) => Ident::new(&lit.value(), lit.span()),
                            _ => unreachable!()
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        impl ToTokens for $name {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                self.0.to_tokens(tokens);
            }
        }
    };
}

builder_expr! {
    fn with_description(BuilderExprDescription, BuilderExpr::Description);
    fn with_author(BuilderExprAuthor, BuilderExpr::Author);
    fn with_license(BuilderExprLicense, BuilderExpr::License);
    fn with_parameter(<inline table_name>, BuilderExprParameter, BuilderExpr::Parameter, true);
    fn with_resource(<inline table_name>, BuilderExprResource, BuilderExpr::Resource);
    fn with_namespace(BuilderExprNamespace, BuilderExpr::Namespace);
    fn with_import(<inline table_name>, BuilderExprImport, BuilderExpr::Import, true);
    fn with_export(<inline table_name>, BuilderExprExport, BuilderExpr::Export, true);
    fn with_dynamic_export(<inline table_name>, BuilderExprDynExport, BuilderExpr::DynExport, true);
    fn with_state(BuilderExprState, BuilderExpr::State, true);
    fn with_on_start_event(BuilderExprOnStartEvent, BuilderExpr::OnStartEvent);
    fn with_on_stop_event(BuilderExprOnStopEvent, BuilderExpr::OnStopEvent);
    fn build(BuilderExprBuild, BuilderExpr::Build);
}

struct BuilderExprInner {
    dot_token: Token![.],
    method: Ident,
    turbofish: Option<AngleBracketedGenericArguments>,
    paren_token: token::Paren,
    args: Punctuated<Expr, Token![,]>,
}

impl BuilderExprInner {
    fn new(expr: &ExprMethodCall, require_first_gen_arg: bool) -> syn::Result<Self> {
        if let Some(attr) = expr.attrs.first() {
            return Err(Error::new_spanned(attr, "attributes not supported"));
        }

        if require_first_gen_arg {
            let elem = expr
                .turbofish
                .iter()
                .flat_map(|args| args.args.iter())
                .find_map(|arg| match arg {
                    GenericArgument::Type(x) => Some(x),
                    _ => None,
                });
            match elem {
                Some(Type::Infer(x)) => {
                    return Err(Error::new_spanned(
                        x,
                        "first generic type argument may not be inferred",
                    ));
                }
                Some(_) => {}
                None => {
                    if let Some(turbofish) = &expr.turbofish {
                        return Err(Error::new_spanned(turbofish, "missing generic arguments"));
                    } else {
                        return Err(Error::new_spanned(
                            &expr.method,
                            "missing generic arguments",
                        ));
                    }
                }
            }
        }

        let dot_token = expr.dot_token;
        let method = expr.method.clone();
        let turbofish = expr.turbofish.clone();
        let paren_token = expr.paren_token;
        let args = expr.args.clone();

        Ok(Self {
            dot_token,
            method,
            turbofish,
            paren_token,
            args,
        })
    }

    fn generic_type_argument(&self, index: usize) -> Option<&Type> {
        self.turbofish
            .iter()
            .flat_map(|args| args.args.iter())
            .filter_map(|arg| match arg {
                GenericArgument::Type(x) => Some(x),
                _ => None,
            })
            .nth(index)
    }
}

impl ToTokens for BuilderExprInner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.dot_token.to_tokens(tokens);
        self.method.to_tokens(tokens);
        if let Some(turbofish) = &self.turbofish {
            turbofish.to_tokens(tokens);
        }
        self.paren_token.surround(tokens, |tokens| {
            self.args.to_tokens(tokens);
        });
    }
}

pub fn export_module_impl(_args: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    let item = item.into();
    let item = parse_macro_input!(item as ItemExport);

    let item = quote! {
        #item
    };
    item.into()
}
