//! Object vtable utilities.
use crate::ConstStr;
use std::marker::PhantomData;

/// Makes a struct usable as a vtable.
///
/// The `fimo_vtable` macro adds a set of predefined fields
/// to the struct, to make it's layout compatible with a fimo-object vtable.
/// Furthermore it implements the `VTable` trait for the given struct.
/// The attribute items are specified inside the `< >` brackets following the struct name.
/// The 'id' key is the unique id of the interface. The 'marker' key is an optional value which
/// specifies a path to a marker type to use when implementing the `VTable` trait.
///
/// # Struct Layout
///
/// The struct will be laid out according to the system´s C ABI, which is achieved be appending the
/// `[#repr(C)]` attribute to the struct definition. VTables are only passed as a pointer or
/// reference, as that allows adding new fields to the end of the table, without breaking the ABI.
///
/// # Example
///
/// ```
/// #![feature(const_fn_trait_bound)]
/// #![feature(const_fn_fn_ptr_basics)]
/// 
/// use fimo_object::fimo_vtable;
///
/// fimo_vtable! {
///     // VTable with default Marker
///     #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
///     struct InterfaceDefMar<id = "default_marker_interface"> {
///         pub get_name: for<'a> fn(*const ()) -> *const str
///     }
/// }
///
/// // Marker that is `Send` and `Sync`
/// struct Marker;
///
/// fimo_vtable! {
///     // VTable with custom Marker
///     #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
///     struct InterfaceCusMar<id = "default_marker_interface", marker = Marker> {
///         pub get_name: fn(*const ()) -> *const str
///     }
/// }
///
/// ```
#[macro_export]
macro_rules! fimo_vtable {
    // struct with named fields
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<id = $id:literal, marker = $marker:ty>{
            $(
                $(#[$elem_attr:meta])* $elem_vis:vis $elem:ident: $elem_ty:ty
            ),*
        }
    ) => {
        $(#[$attr])*
        #[repr(C)]
        $vis struct $name {
            /// Dropping procedure for the object.
            ///
            /// Consumes the pointer.
            pub __internal_drop_in_place: unsafe extern "C" fn(*mut ()),
            /// Size of the object.
            pub __internal_object_size: usize,
            /// Alignment of the object.
            pub __internal_object_alignment: usize,
            /// Unique id of the object type.
            pub __internal_object_id: $crate::ConstStr<'static>,
            /// Unique id of the interface type.
            pub __internal_interface_id: $crate::ConstStr<'static>,
            $($(#[$elem_attr])* $elem_vis $elem: $elem_ty),*
        }

        impl $name {
            /// Constructs a new instance of the vtable.
            pub const fn new<T: $crate::vtable::ObjectID>($($elem: $elem_ty),*) -> Self {
                Self {
                    __internal_drop_in_place: $crate::vtable::drop_obj_in_place::<T>,
                    __internal_object_size: std::mem::size_of::<T>(),
                    __internal_object_alignment: std::mem::align_of::<T>(),
                    __internal_object_id: unsafe { $crate::str::from_utf8_unchecked(
                        <T as $crate::vtable::ObjectID>::OBJECT_ID.as_bytes(),
                    ) },
                    __internal_interface_id: unsafe { $crate::str::from_utf8_unchecked(
                        <Self as $crate::vtable::VTable>::INTERFACE_ID.as_bytes(),
                    ) },
                    $($elem),*
                }
            }
        }

        impl $crate::vtable::VTable for $name {
            type Marker = $marker;
            const INTERFACE_ID: &'static str = $id;

            unsafe fn drop_in_place(&self, obj: *mut ()) {
                (self.__internal_drop_in_place)(obj)
            }

            fn size_of(&self) -> usize {
                self.__internal_object_size
            }

            fn align_of(&self) -> usize {
                self.__internal_object_alignment
            }

            fn object_id(&self) -> $crate::ConstStr<'static> {
                self.__internal_object_id
            }

            fn interface_id(&self) -> $crate::ConstStr<'static> {
                self.__internal_interface_id
            }
        }
    };
    // struct with named fields and default marker
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<id = $id:literal>{
            $(
                $(#[$elem_attr:meta])* $elem_vis:vis $elem:ident: $elem_ty:ty
            ),*
        }
    ) => {
        $crate::fimo_vtable!{
            $(#[$attr])*
            $vis struct $name<id=$id,marker=$crate::vtable::DefaultMarker> {
                $($(#[$elem_attr])* $elem_vis $elem: $elem_ty),*
            }
        }
    };
    // unit struct
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<id = $id:literal, marker = $marker:ty>;
    ) => {
        $crate::fimo_vtable!{
            $(#[$attr])*
            $vis struct $name<id=$id, marker=$marker> {}
        }
    };
    // unit struct with default marker
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<id = $id:literal>;
    ) => {
        $crate::fimo_vtable!{
            $(#[$attr])*
            $vis struct $name<id=$id,marker=$crate::vtable::DefaultMarker>;
        }
    };
}

/// Definition of an Object id.
pub trait ObjectID: Sized {
    /// Unique object id.
    const OBJECT_ID: &'static str;
}

/// Definition of an object vtable.
pub trait VTable: 'static + Send + Sync + Sized {
    /// Type used as a marker.
    type Marker;

    /// Unique interface id.
    const INTERFACE_ID: &'static str;

    /// Drops an object, consuming the pointer in the process.
    ///
    /// # Safety
    ///
    /// See [std::ptr::drop_in_place].
    unsafe fn drop_in_place(&self, obj: *mut ());

    /// Retrieves the size of the object.
    fn size_of(&self) -> usize;

    /// Retrieves the alignment of the object.
    fn align_of(&self) -> usize;

    /// Retrieves the unique id of the object.
    fn object_id(&self) -> ConstStr<'static>;

    /// Retrieves the unique id of the interface.
    fn interface_id(&self) -> ConstStr<'static>;
}

fimo_vtable! {
    /// Layout of the minimal object vtable.
    ///
    /// Contains the data required for allocating/deallocating and casting any object.
    #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
    pub struct BaseInterface<id = "__internal_fimo_object_base", marker = DefaultMarker>;
}

/// Default vtable marker.
#[allow(missing_debug_implementations)]
pub struct DefaultMarker(PhantomData<*const ()>);

/// Drops the pointed to value.
///
/// See [std::ptr::drop_in_place].
pub unsafe extern "C" fn drop_obj_in_place<T: ObjectID>(ptr: *mut ()) {
    std::ptr::drop_in_place::<T>(ptr as *mut T)
}
