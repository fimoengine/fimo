//! Object vtable utilities.
use std::marker::PhantomData;

/// Makes a struct usable as a vtable.
///
/// The `fimo_vtable` macro adds a set of predefined fields
/// to the struct, to make it's layout compatible with a fimo-object vtable.
/// Furthermore it implements the `VTable` trait for the given struct.
/// The attribute items are specified inside the `#![ ]` brackets preceding the struct definition.
/// The 'uuid' key is the unique id of the interface. The 'marker' key is an optional value which
/// specifies a path to a marker type to use when implementing the `VTable` trait and must
/// precede the 'uuid' key.
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
///     #![uuid(0x39ac803c, 0x6c11, 0x45b0, 0x9755, 0x91cb71070db5)]
///     struct InterfaceDefMar {
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
///     #![marker = Marker]
///     #![uuid(0xed6cfc9c, 0xb824, 0x41d4, 0xbef4, 0xe8da13b4e1e5)]
///     struct InterfaceCusMar {
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
        #![marker = $marker:ty]
        #![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)]
        $vis:vis struct $name:ident{
            $(
                $(#[$elem_attr:meta])* $elem_vis:vis $elem:ident: $elem_ty:ty
            ),* $(,)?
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
            /// Unique object id.
            pub __internal_object_id: [u8; 16],
            /// Name of the underlying object type.
            pub __internal_object_name: $crate::ConstStr<'static>,
            /// Unique interface id.
            pub __internal_interface_id: [u8; 16],
            /// Name of the interface type.
            pub __internal_interface_name: $crate::ConstStr<'static>,
            $($(#[$elem_attr])* $elem_vis $elem: $elem_ty),*
        }

        impl $name {
            /// Constructs a new instance of the vtable.
            #[allow(clippy::type_complexity)]
            #[allow(clippy::too_many_arguments)]
            pub const fn new<T: $crate::vtable::ObjectID>($($elem: $elem_ty),*) -> Self {
                Self {
                    __internal_drop_in_place: $crate::vtable::drop_obj_in_place::<T>,
                    __internal_object_size: std::mem::size_of::<T>(),
                    __internal_object_alignment: std::mem::align_of::<T>(),
                    __internal_object_id: *<T as $crate::vtable::ObjectID>::OBJECT_ID.as_bytes(),
                    __internal_object_name: unsafe { $crate::str::from_utf8_unchecked(
                        <T as $crate::vtable::ObjectID>::OBJECT_NAME.as_bytes(),
                    ) },
                    __internal_interface_id: *<Self as $crate::vtable::VTable>::INTERFACE_ID.as_bytes(),
                    __internal_interface_name: unsafe { $crate::str::from_utf8_unchecked(
                        <Self as $crate::vtable::VTable>::INTERFACE_NAME.as_bytes(),
                    ) },
                    $($elem),*
                }
            }
        }

        unsafe impl $crate::vtable::VTable for $name {
            type Marker = $marker;
            const INTERFACE_ID: $crate::vtable::Uuid = $crate::vtable::new_uuid($u1, $u2, $u3, (($u4 as u64) << 48) | $u5 as u64);
            const INTERFACE_NAME: &'static str = $crate::vtable::type_name::<$name>();

            unsafe fn drop_in_place(&self, obj: *mut ()) {
                (self.__internal_drop_in_place)(obj)
            }

            fn size_of(&self) -> usize {
                self.__internal_object_size
            }

            fn align_of(&self) -> usize {
                self.__internal_object_alignment
            }

            fn object_id(&self) -> $crate::vtable::Uuid {
                $crate::vtable::Uuid::from_bytes(self.__internal_object_id)
            }

            fn object_name(&self) -> &'static str {
                self.__internal_object_name.into()
            }

            fn interface_id(&self) -> $crate::vtable::Uuid {
                $crate::vtable::Uuid::from_bytes(self.__internal_interface_id)
            }

            fn interface_name(&self) -> &'static str {
                self.__internal_interface_name.into()
            }
        }
    };
    // struct with named fields and default marker
    (
        $(#[$attr:meta])*
        #![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)]
        $vis:vis struct $name:ident{
            $(
                $(#[$elem_attr:meta])* $elem_vis:vis $elem:ident: $elem_ty:ty
            ),* $(,)?
        }
    ) => {
        $crate::fimo_vtable!{
            $(#[$attr])*
            #![marker=$crate::vtable::DefaultMarker]
            #![uuid($u1,$u2,$u3,$u4,$u5)]
            $vis struct $name {
                $($(#[$elem_attr])* $elem_vis $elem: $elem_ty),*
            }
        }
    };
    // unit struct
    (
        $(#[$attr:meta])*
        #![marker = $marker:ty]
        #![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)]
        $vis:vis struct $name:ident;
    ) => {
        $crate::fimo_vtable!{
            $(#[$attr])*
            #![marker=$marker]
            #![uuid($u1,$u2,$u3,$u4,$u5)]
            $vis struct $name {}
        }
    };
    // unit struct with default marker
    (
        $(#[$attr:meta])*
        #![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)]
        $vis:vis struct $name:ident;
    ) => {
        $crate::fimo_vtable!{
            $(#[$attr])*
            #![marker=$crate::vtable::DefaultMarker]
            #![uuid($u1,$u2,$u3,$u4,$u5)]
            $vis struct $name;
        }
    };
}

/// Marks that a type is an object.
///
/// # Examples
///
/// ```
/// use fimo_object::is_object;
///
/// struct Obj;
/// is_object!{ #![uuid(0x6cf7178d, 0x472f, 0x454a, 0x9b52, 0x5f67b546fd92)] Obj }
/// ```
#[macro_export]
macro_rules! is_object {
    (#![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)] $name:ty) => {
        unsafe impl $crate::vtable::ObjectID for $name {
            const OBJECT_ID: $crate::vtable::Uuid =
                $crate::vtable::new_uuid($u1, $u2, $u3, (($u4 as u64) << 48) | $u5 as u64);
            const OBJECT_NAME: &'static str = $crate::vtable::type_name::<$name>();
        }
    };
}

/// Defines a new marker type.
#[macro_export]
macro_rules! fimo_marker {
    (
        $(#[$attr:meta])*
        $(#![requires($($marker:ident),+)])?
        $vis:vis marker $name:ident {
            $(
                $(#[$elem_attr:meta])* $elem_vis:vis $elem:ident: $elem_ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name {
            $(
                $(#[$elem_attr])*
                $elem_vis $elem:$elem_ty
            ),*
        }

        $crate::fimo_marker! {
            $name $($($marker)+)?
        }
    };
    (
        $(#[$attr:meta])*
        $(#![requires($($marker:ident),+)])?
        $vis:vis marker $name:ident;
    ) => {
        $crate::fimo_marker! {
            $(#[$attr])*
            $(#![requires($($marker),+)])?
            $vis marker $name {}
        }
    };
    ($name:ident) => {
        impl $name {
            /// Checks whether `T` is compatible with the marker.
            pub const fn type_is_compatible<T>() {}
        }

        unsafe impl<T> $crate::vtable::MarkerCompatible<T> for $name {}
    };
    ($name:ident $($marker:ident)+) => {
        impl $name {
            /// Checks whether `T` is compatible with the marker.
            pub const fn type_is_compatible<T>() where $(T: $marker),+ {}
        }

        unsafe impl<T> $crate::vtable::MarkerCompatible<T> for $name where $(T: $marker),+ {}
    };
}

pub use uuid::Uuid;

/// Marker trait indicating that a type `T` is compatible with the marker.
///
/// # Safety
///
/// The compatability with the marker can not be ensured by the compiler.
pub unsafe trait MarkerCompatible<T> {}

/// Definition of an Object.
///
/// # Safety
///
/// This trait requires that the id of the object is unique.
/// The [`is_object!`] macro automatically implements this trait.
pub unsafe trait ObjectID: Sized {
    /// Unique object id.
    const OBJECT_ID: Uuid = Uuid::nil();

    /// Name of the Object.
    const OBJECT_NAME: &'static str;
}

/// Definition of an object vtable.
///
/// # Safety
///
/// This trait requires that the start of the vtable conforms with the layout
/// of an [`IBaseInterface`] and that the id of the interface is unique.
/// The [`fimo_vtable!`] macro automatically implements this trait.
pub unsafe trait VTable: 'static + Send + Sync + Sized {
    /// Type used as a marker.
    type Marker;

    /// Unique interface id.
    const INTERFACE_ID: Uuid = Uuid::nil();

    /// Name of the interface.
    const INTERFACE_NAME: &'static str;

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

    /// Retrieves the unique id of the underlying object.
    fn object_id(&self) -> Uuid;

    /// Retrieves the name of the underlying object.
    fn object_name(&self) -> &'static str;

    /// Retrieves the unique id of the interface.
    fn interface_id(&self) -> Uuid;

    /// Retrieves the name of the interface.
    fn interface_name(&self) -> &'static str;

    /// Casts a `&Self` to a [`IBaseInterface`] reference.
    fn as_base(&self) -> &IBaseInterface {
        unsafe { std::mem::transmute(self) }
    }
}

fimo_vtable! {
    /// Layout of the minimal object vtable.
    ///
    /// Contains the data required for allocating/deallocating and casting any object.
    #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
    #![marker = DefaultMarker]
    #![uuid(0x0, 0x0, 0x0, 0x0, 0x0)]
    pub struct IBaseInterface;
}

fimo_marker! {
    /// Default vtable marker.
    #[allow(missing_debug_implementations)]
    pub marker DefaultMarker {
        _phantom: PhantomData<*const ()>
    }
}

/// Drops the pointed to value.
///
/// # Safety
///
/// See [std::ptr::drop_in_place].
pub unsafe extern "C" fn drop_obj_in_place<T: ObjectID>(ptr: *mut ()) {
    std::ptr::drop_in_place::<T>(ptr as *mut T)
}

/// Returns the name of a type as a string slice.
pub const fn type_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
}

/// Constructs a new [`Uuid`].
pub const fn new_uuid(d1: u32, d2: u16, d3: u16, d4: u64) -> Uuid {
    let d4 = d4.to_be_bytes();
    Uuid::from_fields(d1, d2, d3, &d4)
}
