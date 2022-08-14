#![feature(once_cell)]

use proc_macro::TokenStream;

mod interface;
mod object;

/// Defines a new interface.
///
/// # Interface
///
/// If successfull, the interface definition is transformed into a coresponding
/// trait with the same name and bounds, and a vtable. The resulting vtable is
/// then compatible to be used with a `DynObj`. The definition of a interface
/// resembles trait definitions, but does not support any generic arguments.
///
/// ## Example syntax
///
/// ```ignore
/// pub frozen interface InterfaceName: marker Marker + OtherInterface @ frozen version("1.5") {
///     fn method_1(&self, param: usize);
///     fn method_2(&mut self) -> usize;
///     fn method_3<'a>(&'a self, n: &'a ()) -> &'a ();
///     ...
/// }
/// ```
///
/// ## Bounds
///
/// Similarly to traits, interfaces allow specifying bounds. The bounds
/// are either marker bounds or other interface bounds.
///
/// ### Marker Bounds
///
/// Marker bounds are traits from a predefined list of marker traits.
/// Currently the list consists of the following traits:
///
/// - `Send`: [`Send`] trait.
/// - `Sync`: [`Sync`] trait.
/// - `Unpin`: [`Unpin`] trait.
/// - `IBase`: Base trait for all interfaces.
///     Is required, if the interface does not specify any interface bounds.
///
/// ### Interface Bounds
///
/// Interface bounds are bounds to already defined interfaces.
/// Each bound must also specify a version with the syntax `version("major.minor")`
/// and whether the bound should be frozen to the specified version.
/// The frozen keyword can only be added if the bound itself is also marked as frozen.
/// Specifying an incompatible version will result in a compilation error.
///
/// ## Frozen interfaces
///
/// When an interface is marked as frozen it signifies that its definition
/// won't be altered without increasing the major interface version.
/// Once marked as frozen, the interface can be added as a frozen interface bound
/// to other interfaces. Freezing a bound may enable some optimizations regarding
/// the layout of a vtable and when accessing embedded vtables.
///
/// ## Members
///
/// Interfaces can specify zero or more interface methods.
/// The syntax is equivalent to trait methods.
///
/// ### Generics
///
/// Interface methods can specify generic lifetime parameters.
/// Lifetimes starting with two underscores (`__`) are reserved.
/// As we require the resulting trait to be object safe, we do not allow generic
/// type parameters. As an alternative, one can specify an extension trait containing
/// a generic version of a method and implement it for all the types that implement
/// the interface trait.
///
/// # Configuration
///
/// The attribute `#![interface_cfg(...)` can be placed as the first element
/// inside the macro, and allows to configure settings pertaining to the interface,
/// like the version a uuid and a default abi.
///
/// The attribute `#[interface_cfg(...)` can be specified on methods and
/// method parameters, and controlls how the method is mapped to a vtable.
/// If no attribute is specified it defaults to inheriting the settings from the
/// parent scope.
///
/// ## Interface CFG
///
/// ### Version
///
/// Unlike traits, interfaces carry some abi stability guarantees even if an interface
/// is modified. ABI compatible modifications are signaled by modifying the minor version,
/// while ABI breaks are recorded by the major version.
///
/// The minor version is inferred by the contained methods, while the major version is
/// specified as a global config. If no version is specified, it defaults to the
/// version `0.0`.
///
/// ### VTable Name
///
/// A global config allows to specify the identifier of the vtable to be generated.
/// If none is specified, it defaults to the name of the interface concatenated with
/// `VTable`, i. e. for an interface `Interface` it defaults to `InterfaceVTable`.
/// Given a name `Name`, the macro reserves the names `Name`, `NameHead` and `NameData`.
///
/// ### Calling Convention
///
/// By default, the the function pointers in the vtable adopt the calling
/// convention of the method. Otherwise it is also possible to specify it explicitly.
///
/// #### Example
///
/// - `abi = "inherit"`: Inherit from config from parent.
/// - `abi(explicit(abi = "ABI"))`: Explitit calling convention, i. e. `"C"` or `"Rust"`.
///
/// ### Marshaling
///
/// Method parameters and return types must be marshalled and subsequently be demarshalled,
/// when passing through the vtable shims. The marshalling is controlled by implementing a
/// trait with the signature below for each type we intend to use as a parameter or as a
/// return type. If no marshaler is specified, a default marshaler depending on the calling
/// convention of the method is used.
///
/// #### Marshaler
///
/// ```ignore
/// trait CustomMarshaler {
///     /// Type to marshal to.
///     type Type;
///
///     /// Marshals the type.
///     fn marshal(self) -> Self::Type;
///
///     /// Demarshals the type.
///     ///
///     /// # Safety
///     ///
///     /// The marshaling operation represents a non injective mapping
///     /// from the type `T` to an arbitrary type `U`. Therefore it is likely,
///     /// that multiple types are mapped to the same `U` type.
///     ///
///     /// When calling this method, one must ensure that the same marshaler
///     /// is used for both marshalling and demarshalling, i. e. `T::marshal`
///     /// followed by `T::demarshal`, or that the marshaler is able to work
///     /// with the value one intends to demarshal.
///     unsafe fn demarshal(x: Self::Type) -> Self;
/// }
/// ```
///
/// #### Default Marshaler
///
/// Defined default marshalers:
///
/// - `extern "Rust"` => `fimo_ffi::marshal::RustTypeBridge` (No implementation necessary).
/// - `extern "C"` or `extern "C-unwind"` => `fimo_ffi::marshal::CTypeBridge`.
///
/// #### Example
///
/// - `marshal = "auto"`: Use default marshaler.
///
/// ### Method VTable Mapping
///
/// Amongst the settings that can be customized is the `mapping` parameter.
/// This setting confgures wheter and how a method is mapped to a vtable.
/// If none is specified, the macro defaults to adding the method to the vtable.
///
/// #### Example
///
/// - `mapping = "include"`: Include method in the vtable (Default).
/// - `mapping = "exclude"`: Don't add the method to the vtable.
/// - `mapping(optional())`: Marks the method as being optional.
///     Uses the default implementation if the method is not present in the vtable.
/// - `mapping(optional(replace = "..."))`: Marks the method as being optional.
///     Calls the method specified in replace if the method is not present in the vtable.
///
/// ### UUID
///
/// Interfaces can be marked with a uuid. The uuid must be unique among all interfaces.
/// If left unspecified, it defaults to the zero uuid. A non-zero uuid in conjuction with
/// the major version uniquely identify an interface and allow for it to be downcasted at
/// runtime.
///
/// ## Global CFG
///
/// Allowed parameters in `#![interface_cfg(...)]`:
///
/// - `version = "..."`: Major version of the interface.
/// - `vtable = "Name"`: Name of the vtable.
/// - `no_dyn_impl`: Skip implementation for `DynObj`. Implementation detail.
/// - `abi`: Default calling convention of the methods. See above for the syntax.
/// - `marshal`: Default marshaler for the interface. See above for the syntax.
/// - `uuid = "..."`: UUID of the interface.
///
/// ## Method CFG
///
/// Allowed parameters in `#[interface_cfg(...)]`:
///
/// - `since_minor = "..."`: Minor version when a method was added to the interface.
/// - `abi`: Calling convention of the method. See above for the syntax.
/// - `mapping`: Mapping strategy for the method. See above for the syntax.
/// - `marshal`: Default marshaler for the method. See above for the syntax.
/// - `phantom_parameter = "..."`: Adds a parameter wrapped in a `PhantomData`. Used for
///     resolving unconstrained lifetime errors in the return type.
///
/// ## Parameter CFG
///
/// Allowed parameters in `#[interface_cfg(...)]`:
///
/// - `marshal`: Marshaler for the parameter. See above for the syntax.
///
/// # ABI Stability
///
/// Extending an interface does not break any backwards compatibility, if the interface
/// is not marked as frozen, the new methods are added at the end and they are marked
/// to require a higher minor version.
///
/// Once backwards compatiblity has been broken, one must increase the major version of
/// the interface and may remove the minor version attributes on the methods.
/// The following modifications are not backwards compatible:
///
/// - Changing the interface visibility.
/// - Removing the `frozen` modifier from the interface.
/// - Modifying any trait bound, including reordering.
/// - Modifying any existing interface method, including reordering.
/// - Adding methods before or inbetween existing methods.
/// - Adding methods at the end without specifying a higher minor version with
///     the `since_minor` parameter.
#[proc_macro]
pub fn interface(input: TokenStream) -> TokenStream {
    interface::interface_impl(input)
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
