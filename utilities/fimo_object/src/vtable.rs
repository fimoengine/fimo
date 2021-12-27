//! Object vtable utilities.
use crate::ConstStr;
use std::marker::PhantomData;

/// Definition of an Object id.
pub trait ObjectID: Sized {
    /// Unique object id.
    const OBJECT_ID: &'static str;
}

/// Definition of an object vtable.
pub trait VTable: 'static + Send + Sync + Sized {
    /// Required marker traits.
    type Markers;

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

/// Layout of the minimal object vtable.
///
/// Contains the data required for allocating/deallocating and casting any object.
#[repr(C)]
#[fimo_vtable("__internal_fimo_object_base")]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct BaseInterface;

/// Default vtable marker.
#[allow(missing_debug_implementations)]
pub struct DefaultMarker(PhantomData<*const ()>);
