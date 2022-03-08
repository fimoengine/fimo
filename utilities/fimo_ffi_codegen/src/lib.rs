use proc_macro::TokenStream;

mod interface;
mod object;
mod vtable;

/// Marks a trait as a `DynObj` interface.
///
/// The attribute `#[interface(uuid = "...", vtable = "...", generate(...))` must be placed
/// on an object safe trait whose generic parameters only consist of lifetimes.
///
/// The uuid can be specified with the `uuid = "..."` key-value pair, where the
/// supplied string is a valid uuid, hyphenated or not. The uuid must be unique among all interface
/// definitions. Not adding an uuid "hides" the interface and disables the downcasting from a
/// `DynObj<T>` to a `DynObj<Self>`.
///
/// Further, the attribute requires a path to the vtable specified with the `vtable = "..."`
/// key-value pair. The vtable can be declared with the [`#[vtable]`](macro@vtable) attribute or
/// by adding the `generate_vtable` key to the attribute.
///
/// The `generate` key accepts an arbitrary number of existing vtables and generates a vtable
/// containing function pointers for all member functions defined in the trait and implements the trait for `DynObj`.
/// The `#[vtable_info(...)]` attribute can be added to every member function and parameter and controlls how they
/// are mapped for the vtable.
///
/// # Member functions keys
///
/// * `unsafe` marks the function in the vtable as unsafe. If not set, it inherits the unsafety of the member function.
/// * `ignore` skips the member from inclusion in the vtable.
/// * `lifetimes = "for<...>"` adds additional lifetimes the function definition in the vtable.
/// * `abi = r#"extern "...""#` changes the abi of the function in the vtable.
/// * `return_type = "Type"` changes the return type of the member in the vtable.
/// * `into = "Path"` maps the result from the return type of the trait to the one in the vtable.
/// * `into_expr = "Expressions"` Applies a list of expressions and returns the result. Is applied after `into`.
/// * `from = "Path"` like `into` but in reverse.
/// * `from_expr = "Expressions"` like `into_expr` but in reverse.
///
/// # Parameter keys
///
/// * `type = "Type"` changes the type of the parameter in the vtable.
/// * `into = "Path"` maps the result from the type of the trait to the one in the vtable.
/// * `into_expr = "Expressions"` Applies a list of expressions and writes the result in `p_i` where
///     `i` is the zero-indexed index of the parameter. Is applied after `into`.
/// * `from = "Path"` like `into` but in reverse.
/// * `from_expr = "Expressions"` like `into_expr` but in reverse.
///
/// The generated function renames the parameters to `p_i` and writes the result into the `res` variable.
/// The types `&self`/`&mut self` are mapped to `*const ()`/`*mut ()`.
///
/// # Examples
///
/// ```ignore
/// #![feature(unsize)]
///
/// use fimo_ffi_codegen::{interface, vtable};
///
/// #[interface(
///     uuid = "50edd609-7e2f-4834-b80d-2fb70e345bab",
///     vtable = "TraitVTable",
///     generate()
/// )]
/// trait Trait: Send {
///     fn do_something(&self);
/// }
///
/// #[interface(
///     uuid = "50edd609-7e2f-4834-b80d-2fb70e345bab",
///     vtable = "OtherTraitVTable",
///     generate(TraitVTable)
/// )]
/// trait OtherTrait: Trait {
///     // is not included in the vtable.
///     #[vtable_info(ignore)]
///     fn ignored(&mut self);
///
///     // is mapped to `map_ptr: fn(*const ()) -> *const usize`.
///     // `&usize` coerces to `*const usize` so we dont need the `into` function.
///     #[vtable_info(return_type = "*const usize", from_expr = "unsafe { &*res }")]
///     fn map_ptr(&self) -> &usize;
///
///     // is mapped to `map_u32: fn(*const ()) -> u32`.
///     #[vtable_info(return_type = "u32", into = "Into::into", from_expr = "res != 0")]
///     fn map_u32(&self) -> bool;
/// }
///
/// fn call<T: OtherTrait>(ptr: *const ()) -> bool {
///     let t = unsafe { &*(ptr as *const T) };
///     t.map_explicit()
/// }
/// ```
#[proc_macro_attribute]
pub fn interface(args: TokenStream, input: TokenStream) -> TokenStream {
    interface::interface_impl(args, input)
}

/// Generates a vtable compatible with a `ObjMetadata`.
///
/// The attribute must be placed on a named struct or unit struct definition
/// and must contain the implemented trait, e.g. `#[vtable(interface = "for<'a, 'b> Trait<'a, 'b>")]`.
/// Structs using this attribute specify `#[repr(C)]` automatically.
///
/// A struct can specify that it embeds other vtables by adding the `#[super_vtable(is = "...")]`
/// attribute to the vtable fields. Embedded vtables are either primary or secondary vtables.
/// A vtable `T` implements `impl<U> InnerVTable<U> for T where Primary: InnerVTable<U> { ... }`
/// and `impl InnerVTable<Primary> for T { ... }`, while for secondary vtables it only implements
/// `impl InnerVTable<Secondary> for T { ... }`. There can only exist up to one primary vtables but
/// there can be an arbitrary amount of secondary ones.
#[proc_macro_attribute]
pub fn vtable(args: TokenStream, input: TokenStream) -> TokenStream {
    vtable::vtable_impl(args, input)
}

/// Implements the traits necessary for coercing a type to a `DynObj`.
///
/// Adds `ObjectId` and multiple optional `FetchInterface<_>` implementations to a type.
/// Generic types are allowed as long as they require no generic type arguments.
///
/// The attribute requires a single `#[fetch_vtable(...)]` attribute on the type definition.
/// The attribute specifies an optional uuid and an arbitrary amount of traits for which the
/// coercion will be implemented.
///
/// The uuid can be specified with the `uuid = "..."` key value pair, where the
/// supplied string is a valid uuid, hyphenated or not. The uuid must be unique among all object
/// definitions. Not adding an uuid "hides" the type and disables the downcasting from a `DynObj`
/// to the concrete type. This is useful for types where no downcasting will ever be required.
///
/// Interfaces can be specified with the `interfaces(Trait1, Trait2, ..., TraitN)` syntax.
/// Each interface implements the trait:
///
/// ```ignore
/// // generic arguments are extended with an additional `+ 'inner` bound.
/// // I.e. `<'a, 'b: 'a>` turns into `<'a + 'inner, 'b: 'a + 'inner, 'inner>`
/// impl<'inner, ...> FetchVTable<dyn Trait + 'inner> for T<...> {
///     fn fetch_interface(
///     ) -> &'static <<(dyn Trait + 'inner) as ObjInterface>::Base as ObjInterfaceBase>::VTable {
///         type Ty = <<(dyn Trait + 'inner) as ObjInterface>::Base as ObjInterfaceBase>::VTable;
///         static VTABLE: Ty = Ty::new_for::<T<'_, ..., '_>>();
///         &VTABLE
///     }
/// }
/// ```
///
/// The vtables associated with the interfaces are expected to implement the following functions:
///
/// ```ignore
/// impl TraitVTable {
///     /// Constructs a new standalone vtable.
///     pub const fn new_for<'a, T>() -> Self
///     where
///         T: Trait + ObjectId + 'a,
///     {
///         Self::new_for_embedded::<'a, T, dyn Trait>(0)
///     }
///
///     /// Constructs the vtable when it is embedded inside another vtable.
///     /// The `offset` specifies the offset in bytes from the start of the
///     /// standalone vtable to the start of this embedded vtable. In case
///     /// this vtable embeds other vtables they must be constructed with
///     /// the offset set to `offset + offset_in_trait_vtable`.
///     pub const fn new_for_embedded<'a, T, Dyn>(offset: usize) -> Self
///     where
///         T: Trait + ObjectId + Unsize<Dyn> + 'a,
///         Dyn: ObjInterface + ?Sized + 'a,
///     {
///         ...
///     }
/// }
/// ```
#[proc_macro_derive(ObjectId, attributes(fetch_vtable))]
pub fn object(input: TokenStream) -> TokenStream {
    object::object_impl(input)
}
