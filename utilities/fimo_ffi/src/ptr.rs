//! Object pointer implementation.

use crate::ConstStr;
use std::alloc::Layout;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::{PhantomData, Unsize};
use std::ptr::NonNull;

pub use fimo_ffi_codegen::{interface, vtable, ObjectId};
pub use uuid::Uuid;

/// Marks that the layout of a type is compatible with an [`ObjMetadata`].
///
/// # Safety
///
/// This trait can be implemented only if the type is prefixed
/// with the same members of the internal vtable head and is
/// laid out using the system C abi.
pub unsafe trait ObjMetadataCompatible: 'static {}

/// Marks that a type is compatible with the downcast operation.
///
/// # Safety
///
/// It is typically unsound to implement this trait on a type with
/// generic arguments, as the internal id's don't keep track of them.
pub unsafe trait DowncastSafe {}

/// Helper trait for interfaces that implement [`DowncastSafe`].
///
/// # Safety
///
/// Is automatically implemented if [`DowncastSafe`] is implemented for
/// the base interface.
pub unsafe trait DowncastSafeInterface: ObjInterface {}

unsafe impl<T> DowncastSafeInterface for T
where
    T: ObjInterface + ?Sized,
    T::Base: DowncastSafe,
{
}

/// Specifies a new object type.
pub trait ObjectId {
    /// Unique id of the object.
    const OBJECT_ID: Uuid;

    /// Name of the object.
    const OBJECT_NAME: &'static str = std::any::type_name::<Self>();
}

/// Specifies a new interface type.
pub trait ObjInterfaceBase {
    /// VTable of the interface.
    type VTable: ObjMetadataCompatible;

    /// Unique id of the interface.
    const INTERFACE_ID: Uuid;

    /// Name of the interface.
    const INTERFACE_NAME: &'static str = std::any::type_name::<Self>();
}

/// Indicated that a type is usable with a [`DynObj`].
pub trait ObjInterface {
    /// Base type that specifies the used vtable.
    type Base: ObjInterfaceBase + ?Sized;
}

/// Indicates that an object can be coerced to a [`DynObj`].
pub trait FetchVTable<Dyn>: ObjectId + Unsize<Dyn>
where
    Dyn: ObjInterfaceBase + ?Sized,
{
    /// Returns a static reference to the vtable describing the interface and object.
    fn fetch_interface() -> &'static Dyn::VTable;
}

/// Trait for upcasting a vtable.
///
/// The upcasting operation must maintain the ids and names of the original object and interface.
///
/// # Note
///
/// This trait is not transitive, i.e. defining conversions `A -> B` and `B -> C` does not imply
/// that `A -> C`. Such conversions must be manually implemented.
pub trait CastInto<Dyn: ObjInterface + ?Sized>: ObjInterface + Unsize<Dyn> {
    /// Retrieves a super vtable to the same object.
    fn cast_into(obj: ObjMetadata<Self>) -> ObjMetadata<Dyn>;
}

/// Trait for upcasting a vtable.
///
/// The upcasting operation must maintain the ids and names of the original object and interface.
///
/// # Note
///
/// This trait is not transitive, i.e. defining conversions `A -> B` and `B -> C` does not imply
/// that `A -> C`. Such conversions must be manually implemented.
pub trait CastFrom<Dyn: ObjInterface + Unsize<Self> + ?Sized>: ObjInterface {
    /// Casts the vtable to the super vtable of the same object.
    fn cast_from(obj: ObjMetadata<Dyn>) -> ObjMetadata<Self>;
}

impl<T, U> CastFrom<U> for T
where
    T: ObjInterface + ?Sized,
    U: ObjInterface + Unsize<T> + ?Sized,
    <U::Base as ObjInterfaceBase>::VTable: InnerVTable<<T::Base as ObjInterfaceBase>::VTable>,
{
    fn cast_from(obj: ObjMetadata<U>) -> ObjMetadata<T> {
        let vtable = obj.vtable();
        let inner = vtable.inner();
        ObjMetadata::new(inner)
    }
}

impl<T, U> CastInto<U> for T
where
    T: ObjInterface + Unsize<U> + ?Sized,
    U: CastFrom<T> + ?Sized,
{
    #[inline(always)]
    default fn cast_into(obj: ObjMetadata<Self>) -> ObjMetadata<U> {
        U::cast_from(obj)
    }
}

/// A trait used for implementing upcasting of vtables.
///
/// When implemented it indicates that the vtable can provide a vtable for an super interface.
///
/// <p style="background:rgba(255,181,77,0.16);padding:0.75em;">
/// <strong>Warning:</strong> The returned reference must stem from the same object as the vtable
/// and the head of the returned vtable may only differ at the offset field.
/// </p>
///
/// For example, the following vtable would be invalid because an [`ObjMetadata`] expects
/// to be able to revert to the original vtable by offsetting the vtable reference by the
/// amount of bytes specified in the `offset` field of the vtable head:
///
/// ```compile_fail
/// use fimo_ffi::ptr::{VTableHead, ObjMetadataCompatible, InnerVTable};
///
/// #[repr(C)]
/// struct Inner {
///     head: VTableHead,
/// }
///
/// unsafe impl ObjMetadataCompatible for Inner {}
///
/// #[repr(C)]
/// struct VTable {
///     head: VTableHead,
///     inner: &'static VTableInner,
///     // more fields ..
/// }
///
/// unsafe impl ObjMetadataCompatible for VTable {}
///
/// impl InnerVTable<Inner> for VTable {
///     fn inner(&self) -> &Inner {
///         self.inner
///     }
/// }
///
/// std::compile_error!("invalid vtable definition");
/// ```
///
/// While the following vtable is valid:
///
/// ```
/// use fimo_ffi::ptr::{VTableHead, ObjMetadataCompatible, InnerVTable};
///
/// #[repr(C)]
/// struct Inner {
///     head: VTableHead,
/// }
///
/// unsafe impl ObjMetadataCompatible for Inner {}
///
/// #[repr(C)]
/// struct Other {
///     head: VTableHead,
/// }
///
/// unsafe impl ObjMetadataCompatible for Other {}
///
/// // Possible vtable for an equivalent `trait VTable: Inner + Other { ... }`
/// #[repr(C)]
/// struct VTable {
///     // use the head of an `Inner` instead of duplicating it
///     inner: Inner,
///     // second vtable
///     other: Other,
///     // more fields ..
/// }
///
/// unsafe impl ObjMetadataCompatible for VTable {}
///
/// impl InnerVTable<Inner> for VTable {
///     fn inner(&self) -> &Inner {
///         // safety: because both `VTable` and `Inner` are `repr(C)`
///         // and `inner` is the first field of the vtable we can simply
///         // transmute the reference directly.
///         unsafe { std::mem::transmute(self) }
///     }
/// }
///
/// impl InnerVTable<Other> for VTable {
///     fn inner(&self) -> &Other {
///         &self.other
///     }
/// }
/// ```
pub trait InnerVTable<Table: ObjMetadataCompatible>: ObjMetadataCompatible {
    /// Fetches a reference to the inner vtable.
    fn inner(&self) -> &Table;
}

impl<T: ObjMetadataCompatible> InnerVTable<T> for T {
    default fn inner(&self) -> &T {
        self
    }
}

/// The metadata for a `Dyn = dyn SomeTrait` object type.
#[repr(transparent)]
pub struct ObjMetadata<Dyn: ?Sized> {
    vtable_ptr: &'static VTableHead,
    phantom: PhantomData<Dyn>,
}

impl<'a, Dyn: 'a + ?Sized> ObjMetadata<Dyn> {
    const HIDDEN_UUID: Uuid = Uuid::from_bytes([0; 16]);

    /// Constructs a new `ObjMetadata` with a given vtable.
    #[inline]
    pub fn new(vtable: &'static <Dyn::Base as ObjInterfaceBase>::VTable) -> Self
    where
        Dyn: ObjInterface + 'a,
    {
        Self {
            // safety: the safety is guaranteed with the
            // implementation of ObjMetadataCompatible.
            vtable_ptr: unsafe { &*(vtable as *const _ as *const VTableHead) },
            phantom: Default::default(),
        }
    }

    /// Returns a vtable that is compatible with the current interface.
    #[inline]
    pub fn vtable(self) -> &'static <Dyn::Base as ObjInterfaceBase>::VTable
    where
        Dyn: ObjInterface + 'a,
    {
        // safety: the safety is guaranteed with the
        // implementation of ObjMetadataCompatible.
        unsafe { &*(self.vtable_ptr as *const _ as *const _) }
    }

    /// Returns the vtable to a parent object.
    #[inline]
    pub fn super_vtable<T>(self) -> &'a <T::Base as ObjInterfaceBase>::VTable
    where
        Dyn: CastInto<T> + 'a,
        T: ObjInterface + ?Sized + 'a,
    {
        let s = self.cast_super::<T>();
        s.vtable()
    }

    /// Returns if the contained type matches.
    #[inline]
    pub fn is<U>(self) -> bool
    where
        U: DowncastSafe + ObjectId + Unsize<Dyn>,
    {
        (self.object_id() == U::OBJECT_ID) && (U::OBJECT_ID != Self::HIDDEN_UUID)
    }

    /// Returns the super vtable.
    #[inline]
    pub fn cast_super<U>(self) -> ObjMetadata<U>
    where
        Dyn: CastInto<U> + 'a,
        U: ObjInterface + ?Sized + 'a,
    {
        CastInto::cast_into(self)
    }

    /// Returns if the a certain interface is implemented.
    #[inline]
    pub fn is_interface<'b, U>(self) -> bool
    where
        'a: 'b,
        'b: 'a,
        U: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'b,
    {
        (self.interface_id() == U::Base::INTERFACE_ID)
            && (U::Base::INTERFACE_ID != Self::HIDDEN_UUID)
    }

    /// Returns the metadata for the contained interface if it is of type `U`.
    #[inline]
    pub fn downcast_interface<'b, U>(self) -> Option<ObjMetadata<U>>
    where
        'a: 'b,
        'b: 'a,
        U: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'b,
    {
        if self.is_interface::<U>() {
            let vtable_ptr = self.vtable_ptr as *const _ as *const u8;
            let vtable_ptr = vtable_ptr.wrapping_sub(self.vtable_offset());
            let vtable_ptr = vtable_ptr as *const VTableHead;

            // safety: by construction we ensure that the offset points to the same allocation.
            unsafe {
                Some(ObjMetadata {
                    vtable_ptr: &*vtable_ptr,
                    phantom: PhantomData,
                })
            }
        } else {
            None
        }
    }

    /// Returns the size of the type associated with this vtable.
    #[inline]
    pub fn size_of(self) -> usize {
        self.vtable_ptr.__internal_object_size
    }

    /// Returns the alignment of the type associated with this vtable.
    #[inline]
    pub fn align_of(self) -> usize {
        self.vtable_ptr.__internal_object_alignment
    }

    /// Returns the layout of the type associated with this vtable.
    #[inline]
    pub fn layout(self) -> Layout {
        unsafe { Layout::from_size_align_unchecked(self.size_of(), self.align_of()) }
    }

    /// Returns the id of the type associated with this vtable.
    #[inline]
    pub fn object_id(self) -> Uuid {
        Uuid::from_bytes(self.vtable_ptr.__internal_object_id)
    }

    /// Returns the name of the type associated with this vtable.
    #[inline]
    pub fn object_name(self) -> &'static str {
        self.vtable_ptr.__internal_object_name.into()
    }

    /// Returns the id of the interface implemented with this vtable.
    #[inline]
    pub fn interface_id(self) -> Uuid {
        Uuid::from_bytes(self.vtable_ptr.__internal_interface_id)
    }

    /// Returns the name of the interface implemented with this vtable.
    #[inline]
    pub fn interface_name(self) -> &'static str {
        self.vtable_ptr.__internal_interface_name.into()
    }

    /// Offset from the downcasted vtable pointer to the current vtable.
    #[inline]
    pub fn vtable_offset(self) -> usize {
        self.vtable_ptr.__internal_vtable_offset
    }
}

unsafe impl<Dyn: ?Sized> Send for ObjMetadata<Dyn> {}

unsafe impl<Dyn: ?Sized> Sync for ObjMetadata<Dyn> {}

impl<Dyn: ?Sized> Debug for ObjMetadata<Dyn> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjMetadata")
            .field(&(self.vtable_ptr as *const _))
            .finish()
    }
}

impl<Dyn: ?Sized> Unpin for ObjMetadata<Dyn> {}

impl<Dyn: ?Sized> Copy for ObjMetadata<Dyn> {}

impl<Dyn: ?Sized> Clone for ObjMetadata<Dyn> {
    fn clone(&self) -> Self {
        Self {
            vtable_ptr: self.vtable_ptr,
            phantom: self.phantom,
        }
    }
}

impl<Dyn: ?Sized> Eq for ObjMetadata<Dyn> {}

impl<Dyn: ?Sized> PartialEq for ObjMetadata<Dyn> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq::<VTableHead>(self.vtable_ptr, other.vtable_ptr)
    }
}

impl<Dyn: ?Sized> Ord for ObjMetadata<Dyn> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.vtable_ptr as *const VTableHead).cmp(&(other.vtable_ptr as *const VTableHead))
    }
}

impl<Dyn: ?Sized> PartialOrd for ObjMetadata<Dyn> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Dyn: ?Sized> Hash for ObjMetadata<Dyn> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.vtable_ptr, state)
    }
}

/// A type erased object similar to a trait object.
///
/// # Layout
///
/// It is guaranteed that `&DynObj<T>`, `&mut DynObj<T>`, `*const DynObj<T>`,
/// `*mut DynObj<T>`, [`RawObj<T>`] and [`RawObjMut<T>`] share the same memory layout.
///
/// # Note
///
/// Currently it is not possible to allocate an `DynObj<T>` with smart-pointers in std,
/// like `Box` and `Arc`. This is because they are unable to access the size and alignment
/// of the object, as `std::mem::size_of_val::<DynObj<T>>` and
/// `std::mem::align_of_val::<DynObj<T>>` return wrong numbers.
#[repr(transparent)]
#[allow(missing_debug_implementations)]
pub struct DynObj<Dyn: ?Sized> {
    _ptr: PhantomData<RawObj<Dyn>>,
    // makes `DynObj` into a DST with size 0 and alignment 1.
    _inner: [()],
}

// constructing and destructuring of `DynObj` pointers.

/// Forms a raw object pointer from a data address and metadata.
///
/// The pointer is safe to dereference if the metadata and pointer come from the same underlying
/// erased type and the object is still alive.
#[inline]
pub fn from_raw_parts<Dyn: ?Sized>(
    ptr: *const (),
    metadata: ObjMetadata<Dyn>,
) -> *const DynObj<Dyn> {
    let metadata: usize = unsafe { std::mem::transmute(metadata) };
    let inner: *const [()] = std::ptr::from_raw_parts(ptr, metadata);
    inner as *const DynObj<Dyn>
}

/// Forms a mutable raw object pointer from a data address and metadata.
///
/// The pointer is safe to dereference if the metadata and pointer come from the same underlying
/// erased type and the object is still alive.
#[inline]
pub fn from_raw_parts_mut<Dyn: ?Sized>(
    ptr: *mut (),
    metadata: ObjMetadata<Dyn>,
) -> *mut DynObj<Dyn> {
    let metadata: usize = unsafe { std::mem::transmute(metadata) };
    let inner: *mut [()] = std::ptr::from_raw_parts_mut(ptr, metadata);
    inner as *mut DynObj<Dyn>
}

/// Extracts the metadata component of the pointer.
#[inline]
pub fn metadata<Dyn: ?Sized>(ptr: *const DynObj<Dyn>) -> ObjMetadata<Dyn> {
    let metadata = std::ptr::metadata(ptr);
    unsafe { std::mem::transmute(metadata) }
}

/// Constructs a [`RawObj<T>`] from a `*const DynObj<T>`.
#[inline]
pub fn into_raw<Dyn: ?Sized>(ptr: *const DynObj<Dyn>) -> RawObj<Dyn> {
    let (ptr, metadata) = (ptr as *const (), metadata(ptr));
    raw_from_raw_parts(ptr, metadata)
}

/// Constructs a `*const DynObj<T>` from a [`RawObj<T>`].
#[inline]
pub fn from_raw<Dyn: ?Sized>(ptr: RawObj<Dyn>) -> *const DynObj<Dyn> {
    from_raw_parts(ptr.ptr.cast().as_ptr(), ptr.metadata)
}

/// Constructs a [`RawObjMut<T>`] from a `*mut DynObj<T>`.
#[inline]
pub fn into_raw_mut<Dyn: ?Sized>(ptr: *mut DynObj<Dyn>) -> RawObjMut<Dyn> {
    let (ptr, metadata) = (ptr as *mut (), metadata(ptr));
    raw_from_raw_parts_mut(ptr, metadata)
}

/// Constructs a `*mut DynObj<T>` from a [`RawObjMut<T>`].
#[inline]
pub fn from_raw_mut<Dyn: ?Sized>(ptr: RawObjMut<Dyn>) -> *mut DynObj<Dyn> {
    from_raw_parts_mut(ptr.ptr.cast().as_ptr(), ptr.metadata)
}

/// Coerces an object reference to a [`DynObj`] reference.
#[inline]
pub fn coerce_obj<T, Dyn>(obj: &T) -> &DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface + ?Sized,
{
    unsafe { &*coerce_obj_raw(obj) }
}

/// Coerces a object pointer to a [`DynObj`] pointer.
#[inline]
pub fn coerce_obj_raw<T, Dyn>(obj: *const T) -> *const DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface + ?Sized,
{
    let vtable = T::fetch_interface();
    let metadata = ObjMetadata::<Dyn>::new(vtable);
    from_raw_parts(obj as *const (), metadata)
}

/// Coerces a mutable object reference to a [`DynObj`] reference.
#[inline]
pub fn coerce_obj_mut<T, Dyn>(obj: &mut T) -> &mut DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface + ?Sized,
{
    unsafe { &mut *coerce_obj_mut_raw(obj) }
}

/// Coerces a mutable object pointer to a [`DynObj`] pointer.
#[inline]
pub fn coerce_obj_mut_raw<T, Dyn>(obj: *mut T) -> *mut DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface + ?Sized,
{
    let vtable = T::fetch_interface();
    let metadata = ObjMetadata::<Dyn>::new(vtable);
    from_raw_parts_mut(obj as *mut (), metadata)
}

// basic vtable manipulations

/// Returns whether the contained object is of type `T`.
#[inline]
pub fn is<T, Dyn>(obj: *const DynObj<Dyn>) -> bool
where
    T: DowncastSafe + ObjectId + Unsize<Dyn>,
    Dyn: ?Sized,
{
    let metadata = metadata(obj);
    metadata.is::<T>()
}

/// Returns a pointer to the downcasted object if it is of type `T`.
#[inline]
pub fn downcast<T, Dyn>(obj: *const DynObj<Dyn>) -> Option<*const T>
where
    T: DowncastSafe + ObjectId + Unsize<Dyn>,
    Dyn: ?Sized,
{
    if is::<T, Dyn>(obj) {
        Some(obj as *const T)
    } else {
        None
    }
}

/// Returns a mutable pointer to the downcasted object if it is of type `T`.
#[inline]
pub fn downcast_mut<T, Dyn>(obj: *mut DynObj<Dyn>) -> Option<*mut T>
where
    T: DowncastSafe + ObjectId + Unsize<Dyn>,
    Dyn: ?Sized,
{
    if is::<T, Dyn>(obj) {
        Some(obj as *mut T)
    } else {
        None
    }
}

/// Returns a pointer to the super object.
#[inline]
pub fn cast_super<'a, T, U>(obj: *const DynObj<U>) -> *const DynObj<T>
where
    T: ObjInterface + ?Sized + 'a,
    U: CastInto<T> + ?Sized + 'a,
{
    let metadata = metadata(obj);
    let metadata = metadata.cast_super::<T>();
    from_raw_parts(obj as *const _, metadata)
}

/// Returns a mutable pointer to the super object.
#[inline]
pub fn cast_super_mut<'a, T, U>(obj: *mut DynObj<U>) -> *mut DynObj<T>
where
    T: ObjInterface + ?Sized + 'a,
    U: CastInto<T> + ?Sized + 'a,
{
    let metadata = metadata(obj);
    let metadata = metadata.cast_super::<T>();
    from_raw_parts_mut(obj as *mut _, metadata)
}

/// Returns if the a certain interface is implemented.
#[inline]
pub fn is_interface<'a, 'b, T, Dyn>(obj: *const DynObj<Dyn>) -> bool
where
    'a: 'b,
    'b: 'a,
    T: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'b,
    Dyn: ?Sized + 'a,
{
    let metadata = metadata(obj);
    metadata.is_interface::<T>()
}

/// Returns a pointer to the downcasted interface if it is of type `T`.
#[inline]
pub fn downcast_interface<'a, 'b, T, Dyn>(obj: *const DynObj<Dyn>) -> Option<*const DynObj<T>>
where
    'a: 'b,
    'b: 'a,
    T: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'b,
    Dyn: ?Sized + 'a,
{
    let metadata = metadata(obj);
    metadata
        .downcast_interface::<T>()
        .map(|metadata| from_raw_parts(obj as *const (), metadata))
}

/// Returns a mutable pointer to the downcasted interface if it is of type `T`.
#[inline]
pub fn downcast_interface_mut<'a, 'b, T, Dyn>(obj: *mut DynObj<Dyn>) -> Option<*mut DynObj<T>>
where
    'a: 'b,
    'b: 'a,
    T: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'b,
    Dyn: ?Sized + 'a,
{
    let metadata = metadata(obj);
    metadata
        .downcast_interface::<T>()
        .map(|metadata| from_raw_parts_mut(obj as *mut (), metadata))
}

// pointer info

/// Executes the destructor of the contained object.
///
/// # Safety
///
/// See [`std::ptr::drop_in_place()`];
#[inline]
pub unsafe fn drop_in_place<Dyn: ?Sized>(obj: *mut DynObj<Dyn>) {
    let metadata = metadata(obj);
    if let Some(drop) = metadata.vtable_ptr.__internal_drop_in_place {
        (drop)(obj as *mut ())
    }
}

/// Returns the size of the type associated with this vtable.
#[inline]
pub fn size_of_val<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> usize {
    let metadata = metadata(obj);
    metadata.size_of()
}

/// Retrieves the alignment of the object.
#[inline]
pub fn align_of_val<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> usize {
    let metadata = metadata(obj);
    metadata.align_of()
}

/// Retrieves the layout of the contained type.
#[inline]
pub fn layout_of_val<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> Layout {
    let metadata = metadata(obj);
    metadata.layout()
}

/// Returns the id of the type associated with this vtable.
#[inline]
pub fn object_id<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> crate::ptr::Uuid {
    let metadata = metadata(obj);
    metadata.object_id()
}

/// Returns the name of the type associated with this vtable.
#[inline]
pub fn object_name<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> &'static str {
    let metadata = metadata(obj);
    metadata.object_name()
}

/// Returns the id of the interface implemented with this vtable.
#[inline]
pub fn interface_id<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> crate::ptr::Uuid {
    let metadata = metadata(obj);
    metadata.interface_id()
}

/// Returns the name of the interface implemented with this vtable.
#[inline]
pub fn interface_name<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> &'static str {
    let metadata = metadata(obj);
    metadata.interface_name()
}

/// Raw representation of an immutable object.
#[repr(C)]
#[allow(missing_debug_implementations)]
pub struct RawObj<Dyn: ?Sized> {
    ptr: NonNull<u8>,
    metadata: ObjMetadata<Dyn>,
}

unsafe impl<Dyn: Send + ?Sized> Send for RawObj<Dyn> {}

unsafe impl<Dyn: Sync + ?Sized> Sync for RawObj<Dyn> {}

impl<Dyn: ?Sized> Copy for RawObj<Dyn> {}

impl<Dyn: ?Sized> Clone for RawObj<Dyn> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            metadata: self.metadata,
        }
    }
}

impl<Dyn: Unpin + ?Sized> Unpin for RawObj<Dyn> {}

/// Forms a raw object pointer from a data address and metadata.
///
/// The pointer is safe to dereference if the metadata and pointer come from the same underlying
/// erased type and the object is still alive.
#[inline]
pub fn raw_from_raw_parts<Dyn: ?Sized>(ptr: *const (), metadata: ObjMetadata<Dyn>) -> RawObj<Dyn> {
    RawObj {
        ptr: unsafe { NonNull::new_unchecked(ptr as *mut _) },
        metadata,
    }
}

/// Raw representation of a mutable object.
#[repr(C)]
#[allow(missing_debug_implementations)]
pub struct RawObjMut<Dyn: ?Sized> {
    ptr: NonNull<u8>,
    metadata: ObjMetadata<Dyn>,
}

unsafe impl<Dyn: Send + ?Sized> Send for RawObjMut<Dyn> {}

unsafe impl<Dyn: Sync + ?Sized> Sync for RawObjMut<Dyn> {}

impl<Dyn: ?Sized> Copy for RawObjMut<Dyn> {}

impl<Dyn: ?Sized> Clone for RawObjMut<Dyn> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            metadata: self.metadata,
        }
    }
}

impl<Dyn: Unpin + ?Sized> Unpin for RawObjMut<Dyn> {}

/// Forms a mutable raw object pointer from a data address and metadata.
///
/// The pointer is safe to dereference if the metadata and pointer come from the same underlying
/// erased type and the object is still alive.
#[inline]
pub fn raw_from_raw_parts_mut<Dyn: ?Sized>(
    ptr: *mut (),
    metadata: ObjMetadata<Dyn>,
) -> RawObjMut<Dyn> {
    RawObjMut {
        ptr: unsafe { NonNull::new_unchecked(ptr as _) },
        metadata,
    }
}

/// The common head of all vtables.
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct VTableHead {
    /// Dropping procedure for the object.
    ///
    /// Consumes the pointer.
    pub __internal_drop_in_place: Option<unsafe extern "C-unwind" fn(*mut ())>,
    /// Size of the object.
    pub __internal_object_size: usize,
    /// Alignment of the object.
    pub __internal_object_alignment: usize,
    /// Unique object id.
    pub __internal_object_id: [u8; 16],
    /// Name of the underlying object type.
    pub __internal_object_name: ConstStr<'static>,
    /// Unique interface id.
    pub __internal_interface_id: [u8; 16],
    /// Name of the interface type.
    pub __internal_interface_name: ConstStr<'static>,
    /// Offset from the downcasted vtable pointer to the current vtable pointer.
    pub __internal_vtable_offset: usize,
}

impl VTableHead {
    /// Constructs a new vtable.
    pub const fn new<'a, T, Dyn>() -> Self
    where
        T: ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        Self::new_embedded::<'a, T, Dyn>(0)
    }

    /// Constructs a new vtable with a custom offset.
    pub const fn new_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        unsafe extern "C-unwind" fn drop_ptr<T>(ptr: *mut ()) {
            std::ptr::drop_in_place(ptr as *mut T)
        }

        let drop = if std::mem::needs_drop::<T>() {
            Some(drop_ptr::<T> as _)
        } else {
            None
        };

        Self {
            __internal_drop_in_place: drop,
            __internal_object_size: std::mem::size_of::<T>(),
            __internal_object_alignment: std::mem::align_of::<T>(),
            __internal_object_id: *T::OBJECT_ID.as_bytes(),
            __internal_object_name: unsafe {
                crate::str::from_utf8_unchecked(T::OBJECT_NAME.as_bytes())
            },
            __internal_interface_id: *Dyn::Base::INTERFACE_ID.as_bytes(),
            __internal_interface_name: unsafe {
                crate::str::from_utf8_unchecked(Dyn::Base::INTERFACE_NAME.as_bytes())
            },
            __internal_vtable_offset: offset,
        }
    }
}

unsafe impl ObjMetadataCompatible for VTableHead {}

/// Base trait for all objects.
#[interface(vtable = "IBaseVTable", no_dyn_impl, generate())]
pub trait IBase {}

impl<T: ?Sized> IBase for T {}

/// Helper trait for a [`DynObj<dyn IBase>`].
pub trait IBaseExt<'a, Dyn: IBase + ?Sized + 'a> {
    /// Returns if the contained type matches.
    fn is<U>(&self) -> bool
    where
        U: DowncastSafe + ObjectId + Unsize<Dyn>;

    /// Returns the downcasted object if it is of type `U`.
    fn downcast<U>(&self) -> Option<&U>
    where
        U: DowncastSafe + ObjectId + Unsize<Dyn>;

    /// Returns the mutable downcasted object if it is of type `U`.
    fn downcast_mut<U>(&mut self) -> Option<&mut U>
    where
        U: DowncastSafe + ObjectId + Unsize<Dyn>;

    /// Returns the super object.
    fn cast_super<U>(&self) -> &DynObj<U>
    where
        Dyn: CastInto<U> + 'a,
        U: ObjInterface + ?Sized + 'a;

    /// Returns the mutable super object.
    fn cast_super_mut<U>(&mut self) -> &mut DynObj<U>
    where
        Dyn: CastInto<U> + 'a,
        U: ObjInterface + ?Sized + 'a;

    /// Returns if the a certain interface is implemented.
    fn is_interface<U>(&self) -> bool
    where
        U: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'a;

    /// Returns the downcasted interface if it is of type `U`.
    fn downcast_interface<U>(&self) -> Option<&DynObj<U>>
    where
        U: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'a;

    /// Returns the mutable downcasted interface if it is of type `U`.
    fn downcast_interface_mut<U>(&mut self) -> Option<&mut DynObj<U>>
    where
        U: DowncastSafeInterface + Unsize<Dyn> + ?Sized + 'a;
}

impl<'a, T: ?Sized + 'a> IBaseExt<'a, T> for DynObj<T> {
    #[inline]
    fn is<U>(&self) -> bool
    where
        U: DowncastSafe + ObjectId + Unsize<T>,
    {
        is::<U, _>(self)
    }

    #[inline]
    fn downcast<U>(&self) -> Option<&U>
    where
        U: DowncastSafe + ObjectId + Unsize<T>,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast::<U, _>(self).map(|u| unsafe { &*u })
    }

    #[inline]
    fn downcast_mut<U>(&mut self) -> Option<&mut U>
    where
        U: DowncastSafe + ObjectId + Unsize<T>,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast_mut::<U, _>(self).map(|u| unsafe { &mut *u })
    }

    #[inline]
    fn cast_super<U>(&self) -> &DynObj<U>
    where
        T: CastInto<U> + 'a,
        U: ObjInterface + ?Sized + 'a,
    {
        // safety: the pointer stems from the reference so it is always safe
        unsafe { &*cast_super::<U, _>(self) }
    }

    #[inline]
    fn cast_super_mut<U>(&mut self) -> &mut DynObj<U>
    where
        T: CastInto<U> + 'a,
        U: ObjInterface + ?Sized + 'a,
    {
        // safety: the pointer stems from the reference so it is always safe
        unsafe { &mut *cast_super_mut::<U, _>(self) }
    }

    #[inline]
    fn is_interface<U>(&self) -> bool
    where
        U: DowncastSafeInterface + Unsize<T> + ?Sized + 'a,
    {
        is_interface::<U, _>(self)
    }

    #[inline]
    fn downcast_interface<U>(&self) -> Option<&DynObj<U>>
    where
        U: DowncastSafeInterface + Unsize<T> + ?Sized + 'a,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast_interface::<U, _>(self).map(|u| unsafe { &*u })
    }

    #[inline]
    fn downcast_interface_mut<U>(&mut self) -> Option<&mut DynObj<U>>
    where
        U: DowncastSafeInterface + Unsize<T> + ?Sized + 'a,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast_interface_mut::<U, _>(self).map(|u| unsafe { &mut *u })
    }
}
