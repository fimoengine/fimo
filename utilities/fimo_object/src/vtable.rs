//! Object vtable utilities.
use crate::ConstStr;

/// Definition of an Object id.
pub trait ObjectID: Sized {
    /// Unique object id.
    const OBJECT_ID: &'static str;
}

/// Definition of an object vtable.
pub trait VTable: 'static + Send + Sync {
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
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct BaseInterface {
    /// Dropping procedure for the object.
    ///
    /// Consumes the pointer.
    pub drop_in_place: unsafe extern "C" fn(*mut ()),

    /// Size of the object.
    pub object_size: usize,

    /// Alignment of the object.
    pub object_alignment: usize,

    /// Unique id of the object type.
    pub object_id: ConstStr<'static>,

    /// Unique id of the interface type.
    pub interface_id: ConstStr<'static>,
}

impl VTable for BaseInterface {
    type Markers = ();
    const INTERFACE_ID: &'static str = "";

    unsafe fn drop_in_place(&self, obj: *mut ()) {
        (self.drop_in_place)(obj)
    }

    fn size_of(&self) -> usize {
        self.object_size
    }

    fn align_of(&self) -> usize {
        self.object_alignment
    }

    fn object_id(&self) -> ConstStr<'static> {
        self.object_id
    }

    fn interface_id(&self) -> ConstStr<'static> {
        self.interface_id
    }
}
