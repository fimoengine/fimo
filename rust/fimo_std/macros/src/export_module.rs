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
                const _: ::fimo_std::module::__PrivateBuildToken = #(#exprs)*;
                ::fimo_std::__private_sa::assert_type_eq_all!(#ty, &'static ::fimo_std::module::exports::Export<'static>);
            });
        }

        let init_expr = self.init_expr();
        let path = &init_expr.path;
        let view_ident = &init_expr.path.view_ident;
        let owned_ident = &init_expr.path.owned_ident;
        tokens.append_all(quote! {
            ::fimo_std::__private_sa::assert_type_eq_all!(#path, ::fimo_std::module::Builder::<#view_ident<'_>, #owned_ident>);
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
        let export = generate_export(
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
        );

        let export_ident = &self._ident;
        tokens.append_all(quote! {
            mod #module_ident {
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
        let ty = &param.0.t_arg;
        fields.push(quote! {
            #ident: ::core::pin::Pin<&'static ::fimo_std::module::Parameter<#ty>>
        });
        accessors.push(quote! {
            pub const fn #ident(&self) -> ::core::pin::Pin<&'_ ::fimo_std::module::Parameter<#ty>> {
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
        struct ResourceWrapper(*const ::core::ffi::c_char);
        impl ResourceWrapper {
            const fn as_str(&self) -> &str {
                unsafe {
                    let cstr = ::core::ffi::CStr::from_ptr(self.0);
                    #[cfg(debug_assertions)]
                    {
                        match cstr.to_str() {
                            ::core::result::Result::Ok(s) => s,
                            ::core::result::Result::Err(_) => panic!("expected utf8 string")
                        }
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        let len = cstr.count_bytes();
                        ::core::str::from_utf8_unchecked(::core::slice::from_raw_parts(self.0, len))
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
            let symbol_t = &import.0.t_arg;
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
            let symbol_t = &export.0.t_arg;
            (ident, symbol_t)
        })
        .chain(dyn_exports.iter().map(|export| {
            let ident = export.table_name();
            let symbol_t = &export.0.t_arg;
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
        let t = &x.0.t_arg;
        quote! {pub type State = #t;}
    } else {
        quote! {pub type State = ();}
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_export(
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
        let ty = &param.0.t_arg;
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

                type Repr = <#ty as ::fimo_std::module::ParameterCast>::Repr;
                const READ: Option<fn(::fimo_std::module::ParameterData<'_, Repr>) -> Repr> = #read;
                if let Some(x) = READ {
                    extern "C" fn __private_read(parameter: ::fimo_std::module::ParameterData<'_, ()>, value: ::core::ptr::NonNull<()>) {
                        unsafe {
                            type Repr = <#ty as ::fimo_std::module::ParameterCast>::Repr;
                            let parameter = ::core::mem::transmute::<
                                ::fimo_std::module::ParameterData<'_, ()>,
                                ::fimo_std::module::ParameterData<'_, Repr>,
                            >(parameter);
                            let value = value.cast::<Repr>();

                            let f = READ.unwrap_unchecked();
                            value.write(f(parameter));
                        }
                    }
                    p = p.with_read(Some(__private_read));
                }

                const WRITE: Option<fn(::fimo_std::module::ParameterData<'_, Repr>, Repr)> = #write;
                if let Some(x) = WRITE {
                    extern "C" fn __private_write(parameter: ::fimo_std::module::ParameterData<'_, ()>, value: ::fimo_std::ffi::ConstNonNull<()>) {
                        unsafe {
                            type Repr = <#ty as ::fimo_std::module::ParameterCast>::Repr;
                            let parameter = ::core::mem::transmute::<
                                ::fimo_std::module::ParameterData<'_, ()>,
                                ::fimo_std::module::ParameterData<'_, Repr>,
                            >(parameter);
                            let value = value.cast::<Repr>();

                            let f = WRITE.unwrap_unchecked();
                            f(parameter, *value.as_ref());
                        }
                    }
                    p = p.with_write(Some(__private_write));
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
            let t = &imp.0.t_arg;
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
            let t = &imp.0.t_arg;
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
            let t = &exp.0.t_arg;
            let value = &exp.0.args[1];
            quote! {
                const {
                    let name = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAME;
                    let namespace = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAMESPACE;
                    let version = <#t as ::fimo_std::module::symbols::SymbolInfo>::VERSION;
                    ::fimo_std::module::exports::SymbolExport::new(
                        #value,
                        version,
                        name
                    ).with_namespace(namespace)
                }
            }
        })
        .collect::<Vec<_>>();

    let dyn_exports = dyn_exports
        .iter()
        .map(|&exp| {
            let t = &exp.0.t_arg;
            let init = &exp.0.args[1];
            let deinit = &exp.0.args[2];
            quote! {
                const {
                    extern "C" fn constructor(
                        instance: ::core::pin::Pin<& ::fimo_std::module::OpaqueInstanceView<'_>>,
                        symbol: &mut ::core::ptr::NonNull<()>,
                    ) -> ::fimo_std::error::AnyResult {
                        let f = const { #init };
                        unsafe {
                            let instance = ::core::mem::transmute(instance);
                            type T = <#t as ::fimo_std::module::symbols::SymbolInfo>::Type;
                            match f(instance) {
                                ::core::result::Result::Ok(x) => {
                                    let opaque = ::fimo_std::module::symbols::SymbolPointer::into_opaque_ptr(x).cast_mut();
                                    *symbol = ::core::ptr::NonNull::new(opaque).expect("null pointers are not allowed");
                                    ::fimo_std::error::AnyResult::new_ok()
                                }
                                ::core::result::Result::Err(x) => {
                                    let x = <::fimo_std::error::AnyError>::new(x);
                                    ::fimo_std::error::AnyResult::new_err(x)
                                }
                            }
                        }
                    }
                    extern "C" fn destructor(symbol: ::core::ptr::NonNull<()>) {
                        let f = const { #deinit };
                        type T = <#t as ::fimo_std::module::symbols::SymbolInfo>::Type;
                        let symbol = <T as ::fimo_std::module::symbols::SymbolPointer>::from_opaque_ptr(symbol.as_ptr());
                        f(symbol)
                    }

                    let name = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAME;
                    let namespace = <#t as ::fimo_std::module::symbols::SymbolInfo>::NAMESPACE;
                    let version = <#t as ::fimo_std::module::symbols::SymbolInfo>::VERSION;
                    unsafe {
                        ::fimo_std::module::exports::DynamicSymbolExport::new(
                            constructor,
                            destructor,
                            version,
                            name
                        ).with_namespace(namespace)
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let (constructor, destructor) = if let Some(state) = state {
        let init = &state.0.args[0];
        let deinit = &state.0.args[1];

        let constructor = quote! {
            {
                extern "C" fn __private_init(
                    instance: ::core::pin::Pin<& ::fimo_std::module::OpaqueInstanceView<'_>>,
                    set: ::fimo_std::module::LoadingSetView<'_>,
                    state: &mut ::core::option::Option<::core::ptr::NonNull<()>>,
                ) -> ::fimo_std::error::AnyResult {
                    let f = const { #init };
                    unsafe {
                        let instance = std::mem::transmute(instance);
                        match f(instance, set) {
                            ::core::result::Result::Ok(x) => {
                                *state = ::core::option::Option::Some(x.cast());
                                ::fimo_std::error::AnyResult::new_ok()
                            }
                            ::core::result::Result::Err(x) => {
                                let x = <::fimo_std::error::AnyError>::new(x);
                                ::fimo_std::error::AnyResult::new_err(x)
                            }
                        }
                    }
                }
                Some(__private_init)
            }
        };
        let destructor = quote! {
            {
                extern "C" fn __private_deinit(
                    instance: ::core::pin::Pin<& ::fimo_std::module::OpaqueInstanceView<'_>>,
                    state: ::core::option::Option<::core::ptr::NonNull<()>>,
                ) {
                    let f = const { #deinit };
                    unsafe {
                        let instance = std::mem::transmute(instance);
                        let state = state.expect("expected a non-null pointer").cast();
                        f(instance, state)
                    }
                }
                Some(__private_deinit)
            }
        };
        (constructor, destructor)
    } else {
        (quote!(None), quote!(None))
    };

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
            const MODIFIERS: &[::fimo_std::module::exports::Modifier<'static>] = &[];

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
                    #constructor,
                    #destructor,
                    None,
                    None,
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
    ($(fn $method:ident ($(<inline $table_name:ident>,)? $name:ident, $builder:path, $inner:ident);)*) => {
        $(
            builder_expr!(op $method ($($table_name,)? $name, $builder, $inner));
        )*
    };
    (op $method:ident ($name:ident, $builder:path, $inner:ident)) => {
        struct $name($inner);
        impl $name {
            fn from_expr(expr: &ExprMethodCall) -> syn::Result<Vec<BuilderExpr>> {
                assert_eq!(expr.method.to_string(), std::stringify!($method));
                let this = Self($inner::new(expr)?);
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
    (op $method:ident (table_name, $name:ident, $builder:path, $inner:ident)) => {
        struct $name($inner);
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

                let this = Self($inner::new(expr)?);
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
    fn with_description(BuilderExprDescription, BuilderExpr::Description, BuilderExprInner);
    fn with_author(BuilderExprAuthor, BuilderExpr::Author, BuilderExprInner);
    fn with_license(BuilderExprLicense, BuilderExpr::License, BuilderExprInner);
    fn with_parameter(<inline table_name>, BuilderExprParameter, BuilderExpr::Parameter, BuilderExprInnerT);
    fn with_resource(<inline table_name>, BuilderExprResource, BuilderExpr::Resource, BuilderExprInner);
    fn with_namespace(BuilderExprNamespace, BuilderExpr::Namespace, BuilderExprInner);
    fn with_import(<inline table_name>, BuilderExprImport, BuilderExpr::Import, BuilderExprInnerT);
    fn with_export(<inline table_name>, BuilderExprExport, BuilderExpr::Export, BuilderExprInnerT);
    fn with_dynamic_export(<inline table_name>, BuilderExprDynExport, BuilderExpr::DynExport, BuilderExprInnerTU);
    fn with_state(BuilderExprState, BuilderExpr::State, BuilderExprInnerTU);
    fn build(BuilderExprBuild, BuilderExpr::Build, BuilderExprInner);
}

struct BuilderExprInner {
    dot_token: Token![.],
    method: Ident,
    turbofish: Option<AngleBracketedGenericArguments>,
    paren_token: token::Paren,
    args: Punctuated<Expr, Token![,]>,
}

impl BuilderExprInner {
    fn new(expr: &ExprMethodCall) -> syn::Result<Self> {
        if let Some(attr) = expr.attrs.first() {
            return Err(Error::new_spanned(attr, "attributes not supported"));
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

struct BuilderExprInnerT {
    dot_token: Token![.],
    method: Ident,
    colon2_token: Option<Token![::]>,
    lt_token: Token![<],
    generic_args: Punctuated<GenericArgument, Token![,]>,
    t_arg: Type,
    trailing_comma_token: Option<Token![,]>,
    gt_token: Token![>],
    paren_token: token::Paren,
    args: Punctuated<Expr, Token![,]>,
}

impl BuilderExprInnerT {
    fn new(expr: &ExprMethodCall) -> syn::Result<Self> {
        if let Some(attr) = expr.attrs.first() {
            return Err(Error::new_spanned(attr, "attributes not supported"));
        }

        let dot_token = expr.dot_token;
        let method = expr.method.clone();
        let paren_token = expr.paren_token;
        let args = expr.args.clone();

        let mut turbofish = match expr.turbofish.clone() {
            Some(turbofish) => turbofish,
            None => {
                return Err(Error::new_spanned(
                    &expr.method,
                    "expected generic arguments",
                ));
            }
        };
        if turbofish.args.is_empty() {
            return Err(Error::new_spanned(
                turbofish,
                "expected at least 1 generic argument",
            ));
        }

        let colon2_token = turbofish.colon2_token;
        let lt_token = turbofish.lt_token;
        let (t_arg, trailing_comma_token) = turbofish.args.pop().unwrap().into_tuple();
        let t_arg = match t_arg {
            GenericArgument::Type(x) => x,
            _ => return Err(Error::new_spanned(turbofish.args, "expected type argument")),
        };
        let generic_args = turbofish.args;
        let gt_token = turbofish.gt_token;

        Ok(Self {
            dot_token,
            method,
            colon2_token,
            lt_token,
            generic_args,
            t_arg,
            trailing_comma_token,
            gt_token,
            paren_token,
            args,
        })
    }
}

impl ToTokens for BuilderExprInnerT {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.dot_token.to_tokens(tokens);
        self.method.to_tokens(tokens);
        self.colon2_token.to_tokens(tokens);
        self.lt_token.to_tokens(tokens);
        self.generic_args.to_tokens(tokens);
        self.t_arg.to_tokens(tokens);
        self.trailing_comma_token.to_tokens(tokens);
        self.gt_token.to_tokens(tokens);
        self.paren_token.surround(tokens, |tokens| {
            self.args.to_tokens(tokens);
        });
    }
}

struct BuilderExprInnerTU {
    dot_token: Token![.],
    method: Ident,
    colon2_token: Option<Token![::]>,
    lt_token: Token![<],
    generic_args: Punctuated<GenericArgument, Token![,]>,
    t_arg: Type,
    comma_token: Token![,],
    u_arg: Type,
    trailing_comma_token: Option<Token![,]>,
    gt_token: Token![>],
    paren_token: token::Paren,
    args: Punctuated<Expr, Token![,]>,
}

impl BuilderExprInnerTU {
    fn new(expr: &ExprMethodCall) -> syn::Result<Self> {
        if let Some(attr) = expr.attrs.first() {
            return Err(Error::new_spanned(attr, "attributes not supported"));
        }

        let dot_token = expr.dot_token;
        let method = expr.method.clone();
        let paren_token = expr.paren_token;
        let args = expr.args.clone();

        let mut turbofish = match expr.turbofish.clone() {
            Some(turbofish) => turbofish,
            None => {
                return Err(Error::new_spanned(
                    &expr.method,
                    "expected generic arguments",
                ));
            }
        };
        if turbofish.args.len() < 2 {
            return Err(Error::new_spanned(
                turbofish,
                "expected at least 2 generic arguments",
            ));
        }

        let colon2_token = turbofish.colon2_token;
        let lt_token = turbofish.lt_token;
        let (u_arg, trailing_comma_token) = turbofish.args.pop().unwrap().into_tuple();
        let u_arg = match u_arg {
            GenericArgument::Type(x) => x,
            _ => return Err(Error::new_spanned(turbofish.args, "expected type argument")),
        };

        let (t_arg, comma_token) = turbofish.args.pop().unwrap().into_tuple();
        let comma_token = comma_token.unwrap();
        let t_arg = match t_arg {
            GenericArgument::Type(x) => x,
            _ => return Err(Error::new_spanned(turbofish.args, "expected type argument")),
        };
        let generic_args = turbofish.args;
        let gt_token = turbofish.gt_token;

        Ok(Self {
            dot_token,
            method,
            colon2_token,
            lt_token,
            generic_args,
            t_arg,
            comma_token,
            u_arg,
            trailing_comma_token,
            gt_token,
            paren_token,
            args,
        })
    }
}

impl ToTokens for BuilderExprInnerTU {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.dot_token.to_tokens(tokens);
        self.method.to_tokens(tokens);
        self.colon2_token.to_tokens(tokens);
        self.lt_token.to_tokens(tokens);
        self.generic_args.to_tokens(tokens);
        self.t_arg.to_tokens(tokens);
        self.comma_token.to_tokens(tokens);
        self.u_arg.to_tokens(tokens);
        self.trailing_comma_token.to_tokens(tokens);
        self.gt_token.to_tokens(tokens);
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
