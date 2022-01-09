//! Object utilities.
use crate::raw::{CastError, RawObject, RawObjectMut};
use crate::vtable::{BaseInterface, ObjectID, VTable};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

/// Defines a new object with the specified wrapper.
///
/// # Examples
///
/// ```
/// #![feature(const_fn_trait_bound)]
/// #![feature(const_fn_fn_ptr_basics)]
///
/// use fimo_object::{fimo_vtable, fimo_object};
///
/// fimo_vtable! {
///     struct VTable<id = "unique id">;
/// }
///
/// fimo_object!(struct Obj<vtable = VTable>;);
/// ```
#[macro_export]
macro_rules! fimo_object {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<vtable = $vtable:ty> $(;)?
    ) => {
        $crate ::fimo_object! {
            $(#[$attr])*
            $vis struct $name<vtable = $vtable, no_debug>;
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, std::stringify!(($name)))
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident<vtable = $vtable:ty $(, no_debug)? > $(;)?
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        $vis struct $name {
            inner: $crate::object::Object<$vtable>,
        }
        impl $name {
            /// Splits the object reference into it's raw parts.
            #[inline]
            pub fn into_raw_parts(&self) -> (*const (), &'static $vtable) {
                $crate::object::into_raw_parts(&self.inner)
            }
            /// Splits the mutable object reference into it's raw parts.
            #[inline]
            pub fn into_raw_parts_mut(&mut self) -> (*mut (), &'static $vtable) {
                $crate::object::into_raw_parts_mut(&mut self.inner)
            }
            /// Constructs a reference to the object from it's raw parts.
            ///
            /// # Safety
            ///
            /// - The vtable must have a compatible layout.
            /// - The object pointer must be compatible with the vtable.
            #[inline]
            pub unsafe fn from_raw_parts(ptr: *const (), vtable: &'static $vtable) -> *const $name {
                $crate::object::from_raw_parts(ptr, vtable) as _
            }
            /// Constructs a mutable reference to the object from it's raw parts.
            ///
            /// # Safety
            ///
            /// - The vtable must have a compatible layout.
            /// - The object pointer must be compatible with the vtable.
            #[inline]
            pub unsafe fn from_raw_parts_mut(ptr: *mut (), vtable: &'static $vtable) -> *mut $name {
                $crate::object::from_raw_parts_mut(ptr, vtable) as _
            }
        }
        unsafe impl $crate::object::ObjPtrCompat for $name {}
        unsafe impl $crate::object::ObjectWrapper for $name {
            type VTable = $vtable;
            #[inline]
            fn as_object(ptr: *const Self) -> *const $crate::object::Object<Self::VTable> {
                ptr as _
            }
            #[inline]
            fn from_object(obj: *const $crate::object::Object<Self::VTable>) -> *const Self {
                obj as _
            }
        }
    };
}

/// Used for coercing a type to an Object reference.
pub trait CoerceObject<T: VTable>: ObjectID {
    /// Fetches a static reference to the vtable.
    fn get_vtable() -> &'static T;

    /// Coerces the Object to a `&Object<T>`.
    fn coerce_obj(&self) -> &Object<T> {
        // safety: dereferencing is safe, as the pointer stems from self.
        unsafe { &*Self::coerce_obj_raw(self) }
    }

    /// Coerces a pointer to `Self` to a pointer to [`Object`].
    ///
    /// # Safety
    ///
    /// This function is safe, but it may not be safe to
    /// dereference the resulting pointer.
    fn coerce_obj_raw(this: *const Self) -> *const Object<T> {
        let vtable: &'static T = Self::get_vtable();

        // safety: the reference stems from self so it is valid.
        unsafe { from_raw_parts(this as *const (), vtable) }
    }
}

/// Used for coercing a type to a mutable Object reference.
pub trait CoerceObjectMut<T: VTable>: CoerceObject<T> {
    /// Coerces the Object to a `&mut Object<T>`.
    fn coerce_obj_mut(&mut self) -> &mut Object<T> {
        // safety: dereferencing is safe, as the pointer stems from self.
        unsafe { &mut *Self::coerce_obj_mut_raw(self) }
    }

    /// Coerces a mutable pointer to `Self` to a mutable pointer to [`Object`].
    ///
    /// # Safety
    ///
    /// This function is safe, but it may not be safe to
    /// dereference the resulting pointer.
    fn coerce_obj_mut_raw(this: *mut Self) -> *mut Object<T> {
        let vtable: &'static T = Self::get_vtable();

        // safety: the reference stems from self so it is valid.
        unsafe { from_raw_parts_mut(this as *mut (), vtable) }
    }
}

/// Marker trait for a wrapper around an [`Object<T>`].
///
/// # Safety
///
/// The implementor must ensure that the type is only a wrapper the object.
pub unsafe trait ObjectWrapper: ObjPtrCompat {
    /// VTable of the object.
    type VTable: VTable;

    /// Casts a pointer to Self to an object.
    fn as_object(ptr: *const Self) -> *const Object<Self::VTable>;

    /// Casts a pointer to Self to an object.
    fn as_object_mut(ptr: *mut Self) -> *mut Object<Self::VTable> {
        let obj = Self::as_object(ptr);
        obj as *mut _
    }

    /// Casts a pointer to an object to a pointer to Self.
    fn from_object(obj: *const Object<Self::VTable>) -> *const Self;

    /// Casts a pointer to an object to a pointer to Self.
    fn from_object_mut(obj: *mut Object<Self::VTable>) -> *mut Self {
        let this = Self::from_object(obj);
        this as *mut _
    }

    /// Splits the object up into it's raw parts.
    fn into_raw_parts(ptr: *const Self) -> (*const (), &'static Self::VTable) {
        let obj = Self::as_object(ptr);
        into_raw_parts(obj)
    }

    /// Splits the object up into it's raw parts.
    fn into_raw_parts_mut(ptr: *mut Self) -> (*mut (), &'static Self::VTable) {
        let obj = Self::as_object_mut(ptr);
        into_raw_parts_mut(obj)
    }

    /// Constructs the object from it's raw parts.
    ///
    /// # Safety
    ///
    /// See [`from_raw_parts`].
    unsafe fn from_raw_parts(ptr: *const (), vtable: &'static Self::VTable) -> *const Self {
        let obj = from_raw_parts(ptr, vtable);
        Self::from_object(obj)
    }

    /// Constructs the object from it's raw parts.
    ///
    /// # Safety
    ///
    /// See [`from_raw_parts_mut`].
    unsafe fn from_raw_parts_mut(ptr: *mut (), vtable: &'static Self::VTable) -> *mut Self {
        let obj = from_raw_parts_mut(ptr, vtable);
        Self::from_object_mut(obj)
    }
}

/// Marker for types compatible with the custom pointer types.
///
/// # Safety
///
/// This marker can be safely implemented for Sized types or
/// types only wrapping an [`ObjectWrapper`].
pub unsafe trait ObjPtrCompat {}

unsafe impl<T> ObjPtrCompat for T {}

/// An object
///
/// # Layout
///
/// It is guaranteed that `&Object<T>`, `&mut Object<T>`, `*const Object<T>`,
/// `*mut Object<T>`, `RawObject<T>` and `RawObjectMut<T>` share the same memory layout.
///
/// # Note
///
/// Currently it is not possible to allocate an `Object<T>` with smart-pointers in std,
/// like `Box` and `Arc`. This is because they are unable to access the size and alignment
/// of the object, as `std::mem::size_of_val::<Object<T>>` and
/// `std::mem::align_of_val::<Object<T>>` return wrong numbers.
#[repr(transparent)]
pub struct Object<T: VTable> {
    _phantom: PhantomData<&'static T>,
    // makes `ModuleLoader` into a DST with size 0 and alignment 1.
    _inner: [()],
}

impl<T: VTable> Object<T> {
    /// Casts an object to the base object.
    pub fn cast_base(&self) -> &Object<BaseInterface> {
        unsafe { &*Self::cast_base_raw(self) }
    }

    /// Casts a `*const Object<T>` to a `*const Object<BaseInterface>`.
    pub fn cast_base_raw(o: *const Self) -> *const Object<BaseInterface> {
        // safety: transmuting to the base interface is always sound.
        o as _
    }

    /// Casts an object to the base object.
    pub fn cast_base_mut(&mut self) -> &mut Object<BaseInterface> {
        unsafe { &mut *Self::cast_base_mut_raw(self) }
    }

    /// Casts a `*mut Object<T>` to a `*mut Object<BaseInterface>`.
    pub fn cast_base_mut_raw(o: *mut Self) -> *mut Object<BaseInterface> {
        // safety: transmuting to the base interface is always sound.
        o as _
    }

    /// Casts the `&Object<T>` to a `&Object<U>`.
    pub fn try_cast<U: VTable>(&self) -> Result<&Object<U>, CastError<&Self>> {
        let casted = Self::try_cast_raw(self);
        casted.map_or_else(
            |err| {
                Err(CastError {
                    obj: self,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| unsafe { Ok(&*obj) },
        )
    }

    /// Casts the `*const Object<T>` to a `*const Object<U>`.
    pub fn try_cast_raw<U: VTable>(
        o: *const Self,
    ) -> Result<*const Object<U>, CastError<*const Self>> {
        let raw = into_raw(o);
        let casted = crate::raw::try_cast(raw);
        casted.map_or_else(
            |err| {
                Err(CastError {
                    obj: o,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| Ok(from_raw(obj)),
        )
    }

    /// Casts the `&mut Object<T>` to a `&mut Object<U>`.
    pub fn try_cast_mut<U: VTable>(&mut self) -> Result<&mut Object<U>, CastError<&mut Self>> {
        let casted = Self::try_cast_mut_raw(self);
        casted.map_or_else(
            |err| {
                Err(CastError {
                    obj: self,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| unsafe { Ok(&mut *obj) },
        )
    }

    /// Casts the `*mut Object<T>` to a `*mut Object<U>`.
    pub fn try_cast_mut_raw<U: VTable>(
        o: *mut Self,
    ) -> Result<*mut Object<U>, CastError<*mut Self>> {
        let raw = into_raw_mut(o);
        let casted = crate::raw::try_cast_mut(raw);
        casted.map_or_else(
            |err| {
                Err(CastError {
                    obj: o,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| Ok(from_raw_mut(obj)),
        )
    }

    /// Casts the object to a `&O`.
    pub fn try_cast_obj<O: ObjectID>(&self) -> Result<&O, CastError<&Self>> {
        let res = Self::try_cast_obj_raw(self);
        res.map_or_else(
            |err| {
                Err(CastError {
                    obj: self,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| unsafe { Ok(&*obj) },
        )
    }

    /// Casts the object pointer to a `*const O`.
    pub fn try_cast_obj_raw<O: ObjectID>(
        o: *const Self,
    ) -> Result<*const O, CastError<*const Self>> {
        let raw = into_raw(o);
        crate::raw::try_cast_obj::<T, O>(raw).map_err(|e| CastError {
            obj: o,
            required: e.required,
            available: e.available,
        })
    }

    /// Casts the object to a `&mut O`.
    pub fn try_cast_obj_mut<O: ObjectID>(&mut self) -> Result<&mut O, CastError<&mut Self>> {
        let raw = into_raw_mut(self);
        let res = crate::raw::try_cast_obj_mut::<T, O>(raw);
        res.map_or_else(
            |err| {
                Err(CastError {
                    obj: self,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| unsafe { Ok(&mut *obj) },
        )
    }

    /// Casts the mutable object pointer to a `*mut O`.
    pub fn try_cast_obj_mut_raw<O: ObjectID>(o: *mut Self) -> Result<*mut O, CastError<*mut Self>> {
        let raw = into_raw_mut(o);
        crate::raw::try_cast_obj_mut::<T, O>(raw).map_err(|e| CastError {
            obj: o,
            required: e.required,
            available: e.available,
        })
    }
}

unsafe impl<T: VTable> Send for Object<T> where <T as VTable>::Marker: Send {}
unsafe impl<T: VTable> Sync for Object<T> where <T as VTable>::Marker: Sync {}

impl<T: VTable> AsRef<Object<BaseInterface>> for Object<T> {
    fn as_ref(&self) -> &Object<BaseInterface> {
        self.cast_base()
    }
}

impl<T: VTable> AsMut<Object<BaseInterface>> for Object<T> {
    fn as_mut(&mut self) -> &mut Object<BaseInterface> {
        self.cast_base_mut()
    }
}

impl<T: VTable> Debug for Object<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (ptr, vtable) = into_raw_parts(self);

        f.debug_struct("Object")
            .field("ptr", &ptr)
            .field("vtable", &format!("{:p}", vtable))
            .field("object_id", &vtable.object_id())
            .field("interface_id", &vtable.interface_id())
            .finish()
    }
}

unsafe impl<T: VTable> ObjectWrapper for Object<T> {
    type VTable = T;

    fn as_object(ptr: *const Self) -> *const Object<Self::VTable> {
        ptr
    }

    fn from_object(obj: *const Object<Self::VTable>) -> *const Self {
        obj
    }
}

unsafe impl<T: VTable> ObjPtrCompat for Object<T> {}

/// Casts the object into it's raw representation.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn into_raw<T: VTable>(obj: *const Object<T>) -> RawObject<T> {
    // safety: we assume, that `*const Object<T>` has the same layout as `RawObject<T>`.
    unsafe { std::mem::transmute(obj) }
}

/// Casts the object into it's raw representation.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn into_raw_mut<T: VTable>(obj: *mut Object<T>) -> RawObjectMut<T> {
    // safety: we assume, that `*mut Object<T>` has the same layout as `RawObjectMut<T>`.
    unsafe { std::mem::transmute(obj) }
}

/// Constructs the object from it's raw representation.
pub fn from_raw<T: VTable>(obj: RawObject<T>) -> *const Object<T> {
    // safety: we assume, that `*const Object<T>` has the same layout as `RawObject<T>`.
    unsafe { std::mem::transmute(obj) }
}

/// Constructs the object from it's raw representation.
pub fn from_raw_mut<T: VTable>(obj: RawObjectMut<T>) -> *mut Object<T> {
    // safety: we assume, that `*mut Object<T>` has the same layout as `RawObjectMut<T>`.
    unsafe { std::mem::transmute(obj) }
}

/// Casts the object into it's raw parts.
pub fn into_raw_parts<T: VTable>(obj: *const Object<T>) -> (*const (), &'static T) {
    crate::raw::into_raw_parts(into_raw(obj))
}

/// Casts the object into it's raw parts.
pub fn into_raw_parts_mut<T: VTable>(obj: *mut Object<T>) -> (*mut (), &'static T) {
    crate::raw::into_raw_parts_mut(into_raw_mut(obj))
}

/// Constructs an object from it's raw parts.
///
/// # Safety
///
/// - The vtable must have a compatible layout.
/// - The object pointer must be compatible with the vtable.
pub unsafe fn from_raw_parts<T: VTable>(obj: *const (), vtable: &'static T) -> *const Object<T> {
    from_raw(crate::raw::from_raw_parts(obj, vtable))
}

/// Constructs an object from it's raw parts.
///
/// # Safety
///
/// - The vtable must have a compatible layout.
/// - The object pointer must be compatible with the vtable.
pub unsafe fn from_raw_parts_mut<T: VTable>(obj: *mut (), vtable: &'static T) -> *mut Object<T> {
    from_raw_mut(crate::raw::from_raw_parts_mut(obj, vtable))
}

/// Drops an object, consuming the object in the process.
///
/// # Safety
///
/// See [std::ptr::drop_in_place].
pub unsafe fn drop_in_place<T: VTable>(obj: *mut Object<T>) {
    let (ptr, vtable) = into_raw_parts_mut(obj);
    vtable.drop_in_place(ptr)
}

/// Retrieves the size of the object.
pub fn size_of_val<T: VTable>(obj: *const Object<T>) -> usize {
    let (_, vtable) = into_raw_parts(obj);
    vtable.size_of()
}

/// Retrieves the alignment of the object.
pub fn align_of_val<T: VTable>(obj: *const Object<T>) -> usize {
    let (_, vtable) = into_raw_parts(obj);
    vtable.align_of()
}

/// Retrieves the unique id of the object.
pub fn object_id<T: VTable>(obj: *const Object<T>) -> &'static str {
    let (_, vtable) = into_raw_parts(obj);
    vtable.object_id().into()
}

/// Retrieves the unique id of the interface.
pub fn interface_id<T: VTable>(obj: *const Object<T>) -> &'static str {
    let (_, vtable) = into_raw_parts(obj);
    vtable.interface_id().into()
}

#[cfg(test)]
mod tests {
    use crate::object::Object;
    use crate::raw::{RawObject, RawObjectMut};
    use crate::vtable::BaseInterface;

    #[test]
    fn layout() {
        let object_size = std::mem::size_of::<*const Object<BaseInterface>>();
        let object_mut_size = std::mem::size_of::<*mut Object<BaseInterface>>();
        let raw_object_size = std::mem::size_of::<RawObject<BaseInterface>>();
        let raw_object_mut_size = std::mem::size_of::<RawObjectMut<BaseInterface>>();
        assert_eq!(object_size, raw_object_size);
        assert_eq!(object_mut_size, raw_object_mut_size);

        let object_align = std::mem::align_of::<*const Object<BaseInterface>>();
        let object_mut_align = std::mem::align_of::<*mut Object<BaseInterface>>();
        let raw_object_align = std::mem::align_of::<RawObject<BaseInterface>>();
        let raw_object_mut_align = std::mem::align_of::<RawObjectMut<BaseInterface>>();
        assert_eq!(object_align, raw_object_align);
        assert_eq!(object_mut_align, raw_object_mut_align);
    }
}
