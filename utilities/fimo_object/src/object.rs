//! Object utilities.
use crate::raw::{CastError, RawObject, RawObjectMut};
use crate::vtable::{BaseInterface, ObjectID, VTable};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

/// Used for coercing a type to an Object reference.
pub trait CoerceObject<T: VTable>: ObjectID {
    /// Fetches a static reference to the vtable.
    fn get_vtable() -> &'static T;

    /// Coerces the Object to a `&Object<T>`.
    fn coerce_obj(&self) -> &Object<T> {
        let ptr = self as *const _ as *const ();
        let vtable: &'static T = Self::get_vtable();

        // safety: the reference stems from self so it is valid.
        unsafe { &*from_raw_parts(ptr, vtable) }
    }
}

/// Used for coercing a type to a mutable Object reference.
pub trait CoerceObjectMut<T: VTable>: CoerceObject<T> {
    /// Coerces the Object to a `&mut Object<T>`.
    fn coerce_obj_mut(&mut self) -> &mut Object<T> {
        let ptr = self as *mut _ as *mut ();
        let vtable: &'static T = Self::get_vtable();

        // safety: the reference stems from self so it is valid.
        unsafe { &mut *from_raw_parts_mut(ptr, vtable) }
    }
}

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
pub struct Object<T: VTable> {
    _phantom: PhantomData<&'static T>,
    // makes `ModuleLoader` into a DST with size 0 and alignment 1.
    _inner: [()],
}

impl Object<BaseInterface> {
    /// Casts the `&Object<BaseInterface>` to a `&Object<T>`.
    pub fn try_cast<T: VTable>(&self) -> Result<&Object<T>, CastError<&Self>> {
        let raw = into_raw(self);
        let casted = crate::raw::try_cast(raw);
        casted.map_or_else(
            |err| {
                Err(CastError {
                    obj: self,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| unsafe { Ok(&*from_raw(obj)) },
        )
    }

    /// Casts the `&mut Object<BaseInterface>` to a `&mut Object<T>`.
    pub fn try_cast_mut<T: VTable>(&mut self) -> Result<&mut Object<T>, CastError<&mut Self>> {
        let raw = into_raw_mut(self);
        let casted = crate::raw::try_cast_mut(raw);
        casted.map_or_else(
            |err| {
                Err(CastError {
                    obj: self,
                    required: err.required,
                    available: err.available,
                })
            },
            |obj| unsafe { Ok(&mut *from_raw_mut(obj)) },
        )
    }
}

impl<T: VTable> Object<T> {
    /// Casts an object to the base object.
    pub fn cast_base(&self) -> &Object<BaseInterface> {
        // safety: transmuting to the base interface is always sound.
        unsafe { std::mem::transmute::<&Self, _>(self) }
    }

    /// Casts an object to the base object.
    pub fn cast_base_mut(&mut self) -> &mut Object<BaseInterface> {
        // safety: transmuting to the base interface is always sound.
        unsafe { std::mem::transmute::<&mut Self, _>(self) }
    }

    /// Casts the object to a `&O`.
    pub fn try_cast_obj<O: ObjectID>(&self) -> Result<&O, CastError<&Self>> {
        let raw = into_raw(self);
        let res = crate::raw::try_cast_obj::<T, O>(raw);
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
}

unsafe impl<T: VTable> Send for Object<T> where <T as VTable>::Markers: Send {}
unsafe impl<T: VTable> Sync for Object<T> where <T as VTable>::Markers: Sync {}

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
