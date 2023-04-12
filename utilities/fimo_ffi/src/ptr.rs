//! Object pointer implementation.
use crate::marshal::CTypeBridge;
use crate::type_id::StableTypeId;
use crate::{ConstStr, Optional};
use std::alloc::Layout;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::{PhantomData, Unsize};
use std::ptr::NonNull;

pub use fimo_ffi_codegen::{interface, Object};
pub use uuid::{uuid, Uuid};

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
pub unsafe trait DowncastSafeInterface<'a>: ObjInterface<'a> {}

unsafe impl<'a, T> DowncastSafeInterface<'a> for T
where
    T: ObjInterface<'a> + ?Sized,
    T::Base: DowncastSafe,
{
}

trait ObjectId {
    /// Unique id of the object.
    const OBJECT_ID: Optional<StableTypeId>;

    /// Name of the object.
    const OBJECT_NAME: &'static str = std::any::type_name::<Self>();
}

impl<T: ?Sized> ObjectId for T {
    default const OBJECT_ID: Optional<StableTypeId> = Optional::None;
    default const OBJECT_NAME: &'static str = std::any::type_name::<Self>();
}

impl<T: DowncastSafe + ?Sized + 'static> ObjectId for T {
    const OBJECT_ID: Optional<StableTypeId> = Optional::Some(StableTypeId::of::<T>());
}

// We require a way to identify at runtime whether a type implements
// any of the predefined marker bounds, which are `Send`, `Sync` and
// `Unpin`. This is required, because we'd like to downcast from a
// `DynObj<dyn T>` to a `DynObj<dyn U + Marker>` only if the object
// which implements `U` also implements `Marker`.
const NONE_MARKER: usize = 0b0000;
const SEND_MARKER: usize = 0b0001;
const SYNC_MARKER: usize = 0b0010;
const UNPIN_MARKER: usize = 0b0100;

trait MarkerBounds {
    const IMPLEMENTED_MARKERS: usize;
}

trait MaybeImpl<T: ?Sized> {
    const IMPLEMENTED: usize;
}

impl<T: ?Sized, U: ?Sized> MaybeImpl<U> for T {
    default const IMPLEMENTED: usize = NONE_MARKER;
}

impl<T: Send + ?Sized> MaybeImpl<dyn Send> for T {
    const IMPLEMENTED: usize = SEND_MARKER;
}

impl<T: Sync + ?Sized> MaybeImpl<dyn Sync> for T {
    const IMPLEMENTED: usize = SYNC_MARKER;
}

impl<T: Unpin + ?Sized> MaybeImpl<dyn Unpin> for T {
    const IMPLEMENTED: usize = UNPIN_MARKER;
}

impl<T: ?Sized> MarkerBounds for T {
    const IMPLEMENTED_MARKERS: usize = <T as MaybeImpl<dyn Send>>::IMPLEMENTED
        | <T as MaybeImpl<dyn Sync>>::IMPLEMENTED
        | <T as MaybeImpl<dyn Unpin>>::IMPLEMENTED;
}

/// Specifies a new interface type.
pub trait ObjInterfaceBase {
    /// VTable of the interface.
    type VTable: ObjMetadataCompatible;

    /// Unique id of the interface.
    const INTERFACE_ID: Uuid;

    /// Interface is frozen.
    const IS_FROZEN: bool;

    /// Major version of the interface.
    const INTERFACE_VERSION_MAJOR: u32;

    /// Minor version of the interface.
    const INTERFACE_VERSION_MINOR: u32;

    /// Name of the interface.
    const INTERFACE_NAME: &'static str = std::any::type_name::<Self>();
}

/// Indicated that a type is usable with a [`DynObj`].
pub trait ObjInterface<'a>: 'a {
    /// Base type that specifies the used vtable.
    type Base: ObjInterfaceBase + ?Sized;

    #[doc(hidden)]
    const MARKER_BOUNDS: usize = <Self as MarkerBounds>::IMPLEMENTED_MARKERS;
}

/// Indicates that an object can be coerced to a [`DynObj`].
pub trait FetchVTable<Dyn>: Unsize<Dyn>
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
pub trait CastInto<'a, Dyn: ObjInterface<'a> + ?Sized>: ObjInterface<'a> {
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
pub trait CastFrom<'a, Dyn: ObjInterface<'a> + ?Sized>: ObjInterface<'a> {
    /// Casts the vtable to the super vtable of the same object.
    fn cast_from(obj: ObjMetadata<Dyn>) -> ObjMetadata<Self>;
}

impl<'a, T, U> CastFrom<'a, U> for T
where
    T: ObjInterface<'a> + ?Sized,
    U: ObjInterface<'a> + Unsize<T> + ?Sized,
    U::Base: IntoInterface<<T::Base as ObjInterfaceBase>::VTable>,
{
    fn cast_from(obj: ObjMetadata<U>) -> ObjMetadata<T> {
        let vtable = obj.vtable();
        let inner_vtable = <U::Base>::into_vtable(vtable);
        ObjMetadata::new(inner_vtable)
    }
}

impl<'a, T, U> CastInto<'a, U> for T
where
    T: ObjInterface<'a> + Unsize<U> + ?Sized,
    U: CastFrom<'a, T> + ?Sized,
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
/// use fimo_ffi::ptr::{ObjInterfaceBase, Uuid, VTableHead, ObjMetadataCompatible, IntoInterface};
///
/// trait Interface {}
///
/// impl ObjInterfaceBase for dyn Interface {
///     type VTable = VTable;
///     const INTERFACE_ID: Uuid = Uuid::from_bytes([0; 16]);
///     const IS_FROZEN: bool = true;
///     const INTERFACE_VERSION_MAJOR: u32 = 0;
///     const INTERFACE_VERSION_MINOR: u32 = 0;
/// }
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
///     inner: &'static Inner,
///     // more fields ..
/// }
///
/// unsafe impl ObjMetadataCompatible for VTable {}
///
/// impl IntoInterface<Inner> for dyn Interface {
///     fn into_vtable(vtable: &Self::VTable) -> &Inner {
///         vtable.inner
///     }
/// }
///
/// std::compile_error!("invalid vtable definition");
/// ```
///
/// While the following vtable is valid:
///
/// ```
/// use fimo_ffi::ptr::{ObjInterfaceBase, Uuid, VTableHead, ObjMetadataCompatible, IntoInterface};
///
/// trait Interface {}
///
/// impl ObjInterfaceBase for dyn Interface {
///     type VTable = VTable;
///     const INTERFACE_ID: Uuid = Uuid::from_bytes([0; 16]);
///     const IS_FROZEN: bool = true;
///     const INTERFACE_VERSION_MAJOR: u32 = 0;
///     const INTERFACE_VERSION_MINOR: u32 = 0;
/// }
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
/// impl IntoInterface<Inner> for dyn Interface {
///     fn into_vtable(vtable: &Self::VTable) -> &Inner {
///         // safety: because both `VTable` and `Inner` are `repr(C)`
///         // and `inner` is the first field of the vtable we can simply
///         // transmute the reference directly.
///         unsafe { std::mem::transmute(vtable) }
///     }
/// }
///
/// impl IntoInterface<Other> for dyn Interface {
///     fn into_vtable(vtable: &Self::VTable) -> &Other {
///         &vtable.other
///     }
/// }
/// ```
pub trait IntoInterface<T: ObjMetadataCompatible>: ObjInterfaceBase {
    /// Fetches a reference to the inner vtable.
    fn into_vtable(vtable: &Self::VTable) -> &T;
}

/// The metadata for a `Dyn = dyn SomeTrait` object type.
#[repr(transparent)]
#[derive(Copy, CTypeBridge)]
pub struct ObjMetadata<Dyn: ?Sized> {
    vtable_ptr: &'static VTableHead,
    phantom: PhantomData<Dyn>,
}

impl<'a, Dyn: ObjInterface<'a> + ?Sized> ObjMetadata<Dyn> {
    /// Constructs a new `ObjMetadata` with a given vtable.
    #[inline]
    pub const fn new(vtable: &'static <Dyn::Base as ObjInterfaceBase>::VTable) -> Self {
        Self {
            // safety: the safety is guaranteed with the
            // implementation of ObjMetadataCompatible.
            vtable_ptr: unsafe { &*(vtable as *const _ as *const VTableHead) },
            phantom: PhantomData,
        }
    }

    /// Returns a vtable that is compatible with the current interface.
    #[inline]
    pub const fn vtable(self) -> &'static <Dyn::Base as ObjInterfaceBase>::VTable {
        // safety: the safety is guaranteed with the
        // implementation of ObjMetadataCompatible.
        unsafe { &*(self.vtable_ptr as *const _ as *const _) }
    }

    /// Returns the vtable to a parent object.
    #[inline]
    pub fn super_vtable<T>(self) -> &'a <T::Base as ObjInterfaceBase>::VTable
    where
        Dyn: CastInto<'a, T>,
        T: ObjInterface<'a> + ?Sized,
    {
        let s = self.cast_super::<T>();
        s.vtable()
    }

    /// Returns if the contained type matches.
    #[inline]
    pub fn is<U>(self) -> bool
    where
        U: Unsize<Dyn> + 'static,
    {
        self.vtable_ptr.is::<U>()
    }

    /// Returns the super vtable.
    #[inline]
    pub fn cast_super<U>(self) -> ObjMetadata<U>
    where
        Dyn: CastInto<'a, U>,
        U: ObjInterface<'a> + ?Sized,
    {
        CastInto::cast_into(self)
    }

    /// Checks whether the current metadata belongs to the outermost interface.
    #[inline]
    pub fn is_root_metadata(self) -> bool {
        self.vtable_ptr.vtable_offset == 0
    }

    /// Returns the metadata to the outermost interface.
    #[inline]
    pub fn get_root_metadata(self) -> ObjMetadata<dyn IBase + 'a> {
        ObjMetadata {
            vtable_ptr: self.vtable_ptr.get_root_head(),
            phantom: PhantomData,
        }
    }

    /// Returns if the current or root metadata belongs to a certain interface.
    #[inline]
    pub fn is_interface<U>(self) -> bool
    where
        U: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized,
    {
        self.vtable_ptr.is_interface::<U>() || self.vtable_ptr.get_root_head().is_interface::<U>()
    }

    /// Returns if the current metadata belongs to a certain interface.
    #[inline]
    pub fn current_is_interface<U>(self) -> bool
    where
        U: DowncastSafeInterface<'a> + Unsize<Dyn> + ?Sized,
    {
        self.vtable_ptr.is_interface::<U>()
    }

    /// Returns the metadata for the contained interface if it is of type `U`.
    #[inline]
    pub fn downcast_interface<'b, U>(self) -> Option<ObjMetadata<U>>
    where
        U: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized,
    {
        // If the current metadata already belongs to the interface we
        // simply reinterpret it.
        if self.current_is_interface::<U>() {
            Some(ObjMetadata {
                vtable_ptr: self.vtable_ptr,
                phantom: PhantomData,
            })
        } else {
            // Otherwise we check whether the root metadata describes the interface.
            let root = self.get_root_metadata();
            if root.current_is_interface::<U>() {
                Some(ObjMetadata {
                    vtable_ptr: root.vtable_ptr,
                    phantom: PhantomData,
                })
            } else {
                None
            }
        }
    }

    /// Returns the size of the type associated with this vtable.
    #[inline]
    pub const fn size_of(self) -> usize {
        self.vtable_ptr.object_info.size
    }

    /// Returns the alignment of the type associated with this vtable.
    #[inline]
    pub const fn align_of(self) -> usize {
        self.vtable_ptr.object_info.alignment
    }

    /// Returns the layout of the type associated with this vtable.
    #[inline]
    pub const fn layout(self) -> Layout {
        unsafe { Layout::from_size_align_unchecked(self.size_of(), self.align_of()) }
    }

    /// Returns the id of the type associated with this vtable.
    #[inline]
    pub const fn object_id(self) -> Option<StableTypeId> {
        match self.vtable_ptr.object_info.id {
            Optional::None => None,
            Optional::Some(x) => Some(x),
        }
    }

    /// Returns the implemented marker bounds of the object.
    #[inline]
    pub const fn object_markers(self) -> usize {
        self.vtable_ptr.object_info.markers
    }

    /// Returns the name of the type associated with this vtable.
    #[inline]
    pub const fn object_name(self) -> &'static str {
        self.vtable_ptr.object_info.name.as_ref()
    }

    /// Returns the id of the interface implemented with this vtable.
    #[inline]
    pub const fn interface_id(self) -> Uuid {
        Uuid::from_bytes(self.vtable_ptr.interface_info.id)
    }

    /// Returns the name of the interface implemented with this vtable.
    #[inline]
    pub const fn interface_name(self) -> &'static str {
        self.vtable_ptr.interface_info.name.as_ref()
    }

    /// Returns the major version number of the interface implemented with this vtable.
    #[inline]
    pub const fn interface_version_major(self) -> u32 {
        self.vtable_ptr.interface_info.version_major
    }

    /// Returns the minor version number of the interface implemented with this vtable.
    #[inline]
    pub const fn interface_version_minor(self) -> u32 {
        self.vtable_ptr.interface_info.version_minor
    }

    /// Offset from the downcasted vtable pointer to the current vtable.
    #[inline]
    pub const fn vtable_offset(self) -> usize {
        self.vtable_ptr.vtable_offset
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

impl<Dyn: ?Sized> From<RawObj<Dyn>> for *const DynObj<Dyn> {
    #[inline]
    fn from(x: RawObj<Dyn>) -> Self {
        from_raw(x)
    }
}

impl<Dyn: ?Sized> From<RawObjMut<Dyn>> for *mut DynObj<Dyn> {
    #[inline]
    fn from(x: RawObjMut<Dyn>) -> Self {
        from_raw_mut(x)
    }
}

/// Type erased interface type.
#[repr(C)]
#[derive(Clone, Copy, CTypeBridge)]
#[allow(missing_debug_implementations)]
pub struct OpaqueObj {
    v: RawObj<()>,
}

unsafe impl<Dyn: ?Sized> CTypeBridge for RawObj<Dyn> {
    type Type = OpaqueObj;

    fn marshal(self) -> Self::Type {
        unsafe { std::mem::transmute(self) }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        std::mem::transmute(x)
    }
}

unsafe impl<Dyn: ?Sized> CTypeBridge for RawObjMut<Dyn> {
    type Type = OpaqueObj;

    fn marshal(self) -> Self::Type {
        unsafe { std::mem::transmute(self) }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        std::mem::transmute(x)
    }
}

unsafe impl<Dyn: ?Sized> CTypeBridge for *const DynObj<Dyn> {
    type Type = OpaqueObj;

    fn marshal(self) -> Self::Type {
        unsafe {
            let raw = into_raw(self);
            std::mem::transmute(raw)
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let raw = std::mem::transmute::<_, RawObj<Dyn>>(x);
        from_raw(raw)
    }
}

unsafe impl<Dyn: ?Sized> CTypeBridge for *mut DynObj<Dyn> {
    type Type = OpaqueObj;

    fn marshal(self) -> Self::Type {
        unsafe {
            let raw = into_raw_mut(self);
            std::mem::transmute(raw)
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let raw = std::mem::transmute::<_, RawObjMut<Dyn>>(x);
        from_raw_mut(raw)
    }
}

unsafe impl<'a, Dyn: ?Sized> CTypeBridge for &'a DynObj<Dyn> {
    type Type = OpaqueObj;

    fn marshal(self) -> Self::Type {
        unsafe {
            let raw = into_raw(self);
            std::mem::transmute(raw)
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let raw = std::mem::transmute::<_, RawObj<Dyn>>(x);
        &*from_raw(raw)
    }
}

unsafe impl<'a, Dyn: ?Sized> CTypeBridge for &'a mut DynObj<Dyn> {
    type Type = OpaqueObj;

    fn marshal(self) -> Self::Type {
        unsafe {
            let raw = into_raw_mut(self);
            std::mem::transmute(raw)
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let raw = std::mem::transmute::<_, RawObjMut<Dyn>>(x);
        &mut *from_raw_mut(raw)
    }
}

#[doc(hidden)]
pub trait ToPtr<'a, T: ?Sized> {
    type Type;

    unsafe fn to_ptr(self) -> Self::Type;
}

/// Thin pointer to a `DynObj`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, CTypeBridge)]
pub struct ThisPtr<'a, Dyn: ?Sized> {
    ptr: *const (),
    _phantom: PhantomData<&'a Dyn>,
}

impl<'a, 'b, T: ?Sized, U: ?Sized + 'b> ToPtr<'b, U> for &'a DynObj<T> {
    type Type = ThisPtr<'b, U>;

    #[inline(always)]
    unsafe fn to_ptr(self) -> Self::Type {
        ThisPtr {
            ptr: self as *const _ as *const _,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'b, T: ?Sized, U: ?Sized + 'b> ToPtr<'b, U> for &'a mut DynObj<T> {
    type Type = MutThisPtr<'b, U>;

    #[inline(always)]
    unsafe fn to_ptr(self) -> Self::Type {
        MutThisPtr {
            ptr: self as *mut _ as *mut _,
            _phantom: PhantomData,
        }
    }
}

impl<'a, Dyn: ?Sized> ThisPtr<'a, Dyn> {
    /// Casts the pointer to a reference to a `T`.
    ///
    /// # Safety
    ///
    /// Only safe if the pointer points to a `T`.
    #[inline(always)]
    pub const unsafe fn cast<T>(self) -> &'a T {
        &*(self.ptr.cast())
    }
}

/// Mutable thin pointer to a `DynObj`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, CTypeBridge)]
pub struct MutThisPtr<'a, Dyn: ?Sized> {
    ptr: *mut (),
    _phantom: PhantomData<&'a mut Dyn>,
}

impl<'a, Dyn: ?Sized + 'a> MutThisPtr<'a, Dyn> {
    /// Casts the pointer to a mutable reference to a `T`.
    ///
    /// # Safety
    ///
    /// Only safe if the pointer points to a `T`.
    #[inline(always)]
    pub const unsafe fn cast<T>(self) -> &'a mut T {
        &mut *(self.ptr.cast())
    }
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
    let metadata: usize = (metadata.vtable_ptr as *const VTableHead).expose_addr();
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
    let metadata: usize = (metadata.vtable_ptr as *const VTableHead).expose_addr();
    let inner: *mut [()] = std::ptr::from_raw_parts_mut(ptr, metadata);
    inner as *mut DynObj<Dyn>
}

/// Extracts the metadata component of the pointer.
#[inline]
pub fn metadata<Dyn: ?Sized>(ptr: *const DynObj<Dyn>) -> ObjMetadata<Dyn> {
    let metadata = std::ptr::metadata(ptr);
    let metadata_ptr = std::ptr::from_exposed_addr(metadata as _);
    let metadata = unsafe { &*metadata_ptr };
    ObjMetadata {
        vtable_ptr: metadata,
        phantom: PhantomData,
    }
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
pub fn coerce_obj<'a, T, Dyn>(obj: &T) -> &DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface<'a> + ?Sized,
{
    unsafe { &*coerce_obj_raw(obj) }
}

/// Coerces a object pointer to a [`DynObj`] pointer.
#[inline]
pub fn coerce_obj_raw<'a, T, Dyn>(obj: *const T) -> *const DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface<'a> + ?Sized,
{
    let vtable = T::fetch_interface();
    let metadata = ObjMetadata::<Dyn>::new(vtable);
    from_raw_parts(obj as *const (), metadata)
}

/// Coerces a mutable object reference to a [`DynObj`] reference.
#[inline]
pub fn coerce_obj_mut<'a, T, Dyn>(obj: &mut T) -> &mut DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface<'a> + ?Sized,
{
    unsafe { &mut *coerce_obj_mut_raw(obj) }
}

/// Coerces a mutable object pointer to a [`DynObj`] pointer.
#[inline]
pub fn coerce_obj_mut_raw<'a, T, Dyn>(obj: *mut T) -> *mut DynObj<Dyn>
where
    T: FetchVTable<Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjInterface<'a> + ?Sized,
{
    let vtable = T::fetch_interface();
    let metadata = ObjMetadata::<Dyn>::new(vtable);
    from_raw_parts_mut(obj as *mut (), metadata)
}

// basic vtable manipulations

/// Returns whether the contained object is of type `T`.
#[inline]
pub fn is<'a, T, Dyn>(obj: *const DynObj<Dyn>) -> bool
where
    T: Unsize<Dyn> + 'static,
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.is::<T>()
}

/// Returns a pointer to the downcasted object if it is of type `T`.
#[inline]
pub fn downcast<'a, T, Dyn>(obj: *const DynObj<Dyn>) -> Option<*const T>
where
    T: Unsize<Dyn> + 'static,
    Dyn: ObjInterface<'a> + ?Sized,
{
    if is::<T, Dyn>(obj) {
        Some(obj as *const T)
    } else {
        None
    }
}

/// Returns a mutable pointer to the downcasted object if it is of type `T`.
#[inline]
pub fn downcast_mut<'a, T, Dyn>(obj: *mut DynObj<Dyn>) -> Option<*mut T>
where
    T: Unsize<Dyn> + 'static,
    Dyn: ObjInterface<'a> + ?Sized,
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
    T: ObjInterface<'a> + ?Sized,
    U: CastInto<'a, T> + ?Sized,
{
    let metadata = metadata(obj);
    let metadata = metadata.cast_super::<T>();
    from_raw_parts(obj as *const _, metadata)
}

/// Returns a mutable pointer to the super object.
#[inline]
pub fn cast_super_mut<'a, T, U>(obj: *mut DynObj<U>) -> *mut DynObj<T>
where
    T: ObjInterface<'a> + ?Sized,
    U: CastInto<'a, T> + ?Sized,
{
    let metadata = metadata(obj);
    let metadata = metadata.cast_super::<T>();
    from_raw_parts_mut(obj as *mut _, metadata)
}

/// Returns if the a certain interface is implemented.
#[inline]
pub fn is_interface<'a, T, Dyn>(obj: *const DynObj<Dyn>) -> bool
where
    T: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized,
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.is_interface::<T>()
}

/// Returns a pointer to the downcasted interface if it is of type `T`.
#[inline]
pub fn downcast_interface<'a, T, Dyn>(obj: *const DynObj<Dyn>) -> Option<*const DynObj<T>>
where
    T: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized,
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata
        .downcast_interface::<T>()
        .map(|metadata| from_raw_parts(obj as *const (), metadata))
}

/// Returns a mutable pointer to the downcasted interface if it is of type `T`.
#[inline]
pub fn downcast_interface_mut<'a, T, Dyn>(obj: *mut DynObj<Dyn>) -> Option<*mut DynObj<T>>
where
    T: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized,
    Dyn: ObjInterface<'a> + ?Sized,
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
    if let Some(drop) = metadata.vtable_ptr.object_info.drop_in_place {
        (drop)(obj as *mut ())
    }
}

/// Returns the size of the type associated with this vtable.
#[inline]
pub fn size_of_val<'a, Dyn>(obj: *const DynObj<Dyn>) -> usize
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.size_of()
}

/// Retrieves the alignment of the object.
#[inline]
pub fn align_of_val<'a, Dyn>(obj: *const DynObj<Dyn>) -> usize
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.align_of()
}

/// Retrieves the layout of the contained type.
#[inline]
pub fn layout_of_val<'a, Dyn>(obj: *const DynObj<Dyn>) -> Layout
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.layout()
}

/// Returns the id of the type associated with this vtable.
#[inline]
pub fn object_id<'a, Dyn>(obj: *const DynObj<Dyn>) -> Option<StableTypeId>
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.object_id()
}

/// Returns the name of the type associated with this vtable.
#[inline]
pub fn object_name<'a, Dyn>(obj: *const DynObj<Dyn>) -> &'static str
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.object_name()
}

/// Returns the id of the interface implemented with this vtable.
#[inline]
pub fn interface_id<'a, Dyn>(obj: *const DynObj<Dyn>) -> crate::ptr::Uuid
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.interface_id()
}

/// Returns the name of the interface implemented with this vtable.
#[inline]
pub fn interface_name<'a, Dyn>(obj: *const DynObj<Dyn>) -> &'static str
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.interface_name()
}

/// Returns the major version number of the interface implemented with this vtable.
#[inline]
pub fn interface_version_major<'a, Dyn>(obj: *const DynObj<Dyn>) -> u32
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.interface_version_major()
}

/// Returns the minor version number of the interface implemented with this vtable.
#[inline]
pub fn interface_version_minor<'a, Dyn>(obj: *const DynObj<Dyn>) -> u32
where
    Dyn: ObjInterface<'a> + ?Sized,
{
    let metadata = metadata(obj);
    metadata.interface_version_minor()
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

impl<Dyn: ?Sized> From<&DynObj<Dyn>> for RawObj<Dyn> {
    #[inline]
    fn from(x: &DynObj<Dyn>) -> Self {
        into_raw(x)
    }
}

impl<Dyn: ?Sized> From<*const DynObj<Dyn>> for RawObj<Dyn> {
    #[inline]
    fn from(x: *const DynObj<Dyn>) -> Self {
        into_raw(x)
    }
}

/// Forms a raw object pointer from a data address and metadata.
///
/// The pointer is safe to dereference if the metadata and pointer come from the same underlying
/// erased type and the object is still alive.
#[inline]
pub const fn raw_from_raw_parts<Dyn: ?Sized>(
    ptr: *const (),
    metadata: ObjMetadata<Dyn>,
) -> RawObj<Dyn> {
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

impl<Dyn: ?Sized> From<&mut DynObj<Dyn>> for RawObjMut<Dyn> {
    #[inline]
    fn from(x: &mut DynObj<Dyn>) -> Self {
        into_raw_mut(x)
    }
}

impl<Dyn: ?Sized> From<*mut DynObj<Dyn>> for RawObjMut<Dyn> {
    #[inline]
    fn from(x: *mut DynObj<Dyn>) -> Self {
        into_raw_mut(x)
    }
}

/// Forms a mutable raw object pointer from a data address and metadata.
///
/// The pointer is safe to dereference if the metadata and pointer come from the same underlying
/// erased type and the object is still alive.
#[inline]
pub const fn raw_from_raw_parts_mut<Dyn: ?Sized>(
    ptr: *mut (),
    metadata: ObjMetadata<Dyn>,
) -> RawObjMut<Dyn> {
    RawObjMut {
        ptr: unsafe { NonNull::new_unchecked(ptr as _) },
        metadata,
    }
}

/// Generic vtable representation.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GenericVTable<Head: ObjMetadataCompatible, Data: 'static> {
    head: Head,
    data: Data,
}

/// Helper trait for supporting vtables with dynamic offsets to the data section.
///
/// # Safety
///
/// An implementation must ensure that the offset returned by [`DynamicDataOffset::data_offset`],
/// when applied to a pointer to the start of the vtable, points to the start of the data section.
pub unsafe trait DynamicDataOffset: ObjMetadataCompatible {
    /// Returns the offset in bytes from the start of the vtable to the start of the data section.
    fn data_offset(&self) -> usize;
}

trait VTableDataSpec<Data> {
    fn get_data(&self) -> &Data;
}

impl<Head, Data> VTableDataSpec<Data> for GenericVTable<Head, Data>
where
    Head: ObjMetadataCompatible,
    Data: 'static,
{
    default fn get_data(&self) -> &Data {
        &self.data
    }
}

impl<Head, Data> VTableDataSpec<Data> for GenericVTable<Head, Data>
where
    Head: DynamicDataOffset,
    Data: 'static,
{
    default fn get_data(&self) -> &Data {
        let this: *const u8 = (self as *const Self).cast();
        let offset = self.head.data_offset();

        // SAFETY: We offload the correctness to the implementation of `DynamicDataOffset`.
        unsafe {
            let data_ptr: *const Data = this.add(offset).cast();
            &*data_ptr
        }
    }
}

impl<Head, Data> GenericVTable<Head, Data>
where
    Head: ObjMetadataCompatible,
    Data: 'static,
{
    /// Constructs a new vtable.
    #[inline]
    pub const fn new(head: Head, data: Data) -> Self {
        Self { head, data }
    }

    /// Fetches a reference to the head section.
    #[inline]
    pub const fn head(&self) -> &Head {
        &self.head
    }

    /// Fetches a reference to the data section.
    #[inline]
    pub fn data(&self) -> &Data {
        self.get_data()
    }
}

unsafe impl<Head, Data> ObjMetadataCompatible for GenericVTable<Head, Data>
where
    Head: ObjMetadataCompatible,
    Data: 'static,
{
}

/// Information pertaining to the object a vtable stems from.
#[repr(C)]
#[derive(Copy, Clone, CTypeBridge)]
#[allow(missing_debug_implementations)]
pub struct VTableObjectInfo {
    /// Dropping procedure for the object.
    ///
    /// Consumes the pointer.
    pub drop_in_place: Option<unsafe extern "C-unwind" fn(*mut ())>,
    /// Size of the object.
    pub size: usize,
    /// Alignment of the object.
    pub alignment: usize,
    /// Unique object id.
    pub id: Optional<StableTypeId>,
    /// Implemented marker bounds of the object.
    pub markers: usize,
    /// Name of the underlying object type.
    pub name: ConstStr<'static>,
}

impl VTableObjectInfo {
    const fn new<T>() -> Self {
        unsafe extern "C-unwind" fn drop_ptr<T>(ptr: *mut ()) {
            std::ptr::drop_in_place(ptr as *mut T)
        }

        let drop = if std::mem::needs_drop::<T>() {
            Some(drop_ptr::<T> as _)
        } else {
            None
        };

        Self {
            drop_in_place: drop,
            size: std::mem::size_of::<T>(),
            alignment: std::mem::align_of::<T>(),
            id: <T as ObjectId>::OBJECT_ID,
            markers: T::IMPLEMENTED_MARKERS,
            name: unsafe { crate::str::from_utf8_unchecked(T::OBJECT_NAME.as_bytes()) },
        }
    }

    pub(crate) fn is<T: 'static>(&self) -> bool {
        self.id == <T as ObjectId>::OBJECT_ID
    }
}

/// Information pertaining to the interface mapped by the vtable.
#[repr(C)]
#[derive(Debug, Copy, Clone, CTypeBridge, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VTableInterfaceInfo {
    /// Unique interface id.
    pub id: [u8; 16],
    /// Name of the interface type.
    pub name: ConstStr<'static>,
    /// Major version of the interface.
    pub version_major: u32,
    /// Minor version of the interface.
    pub version_minor: u32,
}

impl VTableInterfaceInfo {
    const HIDDEN_UUID: Uuid = Uuid::from_bytes([0; 16]);

    pub(crate) const fn new<'a, T: ObjInterface<'a> + ?Sized>() -> Self {
        Self {
            id: *T::Base::INTERFACE_ID.as_bytes(),
            name: unsafe { crate::str::from_utf8_unchecked(T::Base::INTERFACE_NAME.as_bytes()) },
            version_major: T::Base::INTERFACE_VERSION_MAJOR,
            version_minor: T::Base::INTERFACE_VERSION_MINOR,
        }
    }

    #[inline]
    pub(crate) fn is<'a, T>(&self, object_markers: usize) -> bool
    where
        T: DowncastSafeInterface<'a> + ?Sized,
    {
        (&self.id == T::Base::INTERFACE_ID.as_bytes())
            && (T::Base::INTERFACE_ID != Self::HIDDEN_UUID)
            && (self.version_major == T::Base::INTERFACE_VERSION_MAJOR)
            && ((object_markers & <T as MarkerBounds>::IMPLEMENTED_MARKERS)
                == <T as MarkerBounds>::IMPLEMENTED_MARKERS)
    }
}

/// The common head of all vtables.
#[repr(C)]
#[derive(Copy, Clone, CTypeBridge)]
#[allow(missing_debug_implementations)]
pub struct VTableHead {
    /// Information pertaining to the mapped object.
    pub object_info: VTableObjectInfo,
    /// Information pertaining to the mapped interface.
    pub interface_info: VTableInterfaceInfo,
    /// Offset from the downcasted vtable pointer to the current vtable pointer.
    pub vtable_offset: usize,
}

impl VTableHead {
    /// Constructs a new vtable.
    pub const fn new<'a, T, Dyn>() -> Self
    where
        T: Unsize<Dyn> + 'a,
        Dyn: ObjInterface<'a> + ?Sized,
    {
        Self::new_embedded::<'a, T, Dyn>(0)
    }

    /// Constructs a new vtable with a custom offset.
    pub const fn new_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: Unsize<Dyn> + 'a,
        Dyn: ObjInterface<'a> + ?Sized,
    {
        Self {
            object_info: VTableObjectInfo::new::<T>(),
            interface_info: VTableInterfaceInfo::new::<Dyn>(),
            vtable_offset: offset,
        }
    }

    #[inline]
    pub(crate) fn is<T: 'static>(&self) -> bool {
        self.object_info.is::<T>()
    }

    #[inline]
    pub(crate) fn get_root_head(&self) -> &Self {
        let vtable_ptr = self as *const _ as *const u8;
        let vtable_ptr = vtable_ptr.wrapping_sub(self.vtable_offset);
        let vtable_ptr = vtable_ptr as *const VTableHead;

        // SAFETY: By construction we ensure that the offset points to the same allocation.
        unsafe { &*vtable_ptr }
    }

    #[inline]
    pub(crate) fn is_interface<'a, T>(&self) -> bool
    where
        T: DowncastSafeInterface<'a> + ?Sized,
    {
        self.interface_info.is::<T>(self.object_info.markers)
    }
}

unsafe impl ObjMetadataCompatible for VTableHead {}

interface! {
    #![interface_cfg(no_dyn_impl)]

    /// Base trait for all objects.
    pub frozen interface IBase {}
}

impl<T: ?Sized> IBase for T {}

#[doc(hidden)]
pub const fn __assert_ibase<T: IBase + ?Sized>() {}

/// Helper trait for a [`DynObj<dyn IBase>`].
pub trait IBaseExt<'a, Dyn: ObjInterface<'a> + IBase + ?Sized> {
    /// Returns if the contained type matches.
    fn is<U>(&self) -> bool
    where
        U: Unsize<Dyn> + 'static;

    /// Returns the downcasted object if it is of type `U`.
    fn downcast<U>(&self) -> Option<&U>
    where
        U: Unsize<Dyn> + 'static;

    /// Returns the mutable downcasted object if it is of type `U`.
    fn downcast_mut<U>(&mut self) -> Option<&mut U>
    where
        U: Unsize<Dyn> + 'static;

    /// Returns the super object.
    fn cast_super<U>(&self) -> &DynObj<U>
    where
        Dyn: CastInto<'a, U>,
        U: ObjInterface<'a> + ?Sized;

    /// Returns the mutable super object.
    fn cast_super_mut<U>(&mut self) -> &mut DynObj<U>
    where
        Dyn: CastInto<'a, U>,
        U: ObjInterface<'a> + ?Sized;

    /// Returns if the a certain interface is implemented.
    fn is_interface<U>(&self) -> bool
    where
        U: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized;

    /// Returns the downcasted interface if it is of type `U`.
    fn downcast_interface<U>(&self) -> Option<&DynObj<U>>
    where
        U: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized;

    /// Returns the mutable downcasted interface if it is of type `U`.
    fn downcast_interface_mut<U>(&mut self) -> Option<&mut DynObj<U>>
    where
        U: DowncastSafeInterface<'a> + Unsize<Dyn> + Unsize<dyn IBase + 'a> + ?Sized;
}

impl<'a, T: ObjInterface<'a> + ?Sized> IBaseExt<'a, T> for DynObj<T> {
    #[inline]
    fn is<U>(&self) -> bool
    where
        U: Unsize<T> + 'static,
    {
        is::<U, _>(self)
    }

    #[inline]
    fn downcast<U>(&self) -> Option<&U>
    where
        U: Unsize<T> + 'static,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast::<U, _>(self).map(|u| unsafe { &*u })
    }

    #[inline]
    fn downcast_mut<U>(&mut self) -> Option<&mut U>
    where
        U: Unsize<T> + 'static,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast_mut::<U, _>(self).map(|u| unsafe { &mut *u })
    }

    #[inline]
    fn cast_super<U>(&self) -> &DynObj<U>
    where
        T: CastInto<'a, U>,
        U: ObjInterface<'a> + ?Sized,
    {
        // safety: the pointer stems from the reference so it is always safe
        unsafe { &*cast_super::<U, _>(self) }
    }

    #[inline]
    fn cast_super_mut<U>(&mut self) -> &mut DynObj<U>
    where
        T: CastInto<'a, U>,
        U: ObjInterface<'a> + ?Sized,
    {
        // safety: the pointer stems from the reference so it is always safe
        unsafe { &mut *cast_super_mut::<U, _>(self) }
    }

    #[inline]
    fn is_interface<U>(&self) -> bool
    where
        U: DowncastSafeInterface<'a> + Unsize<T> + Unsize<dyn IBase + 'a> + ?Sized,
    {
        is_interface::<U, _>(self)
    }

    #[inline]
    fn downcast_interface<U>(&self) -> Option<&DynObj<U>>
    where
        U: DowncastSafeInterface<'a> + Unsize<T> + Unsize<dyn IBase + 'a> + ?Sized,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast_interface::<U, _>(self).map(|u| unsafe { &*u })
    }

    #[inline]
    fn downcast_interface_mut<U>(&mut self) -> Option<&mut DynObj<U>>
    where
        U: DowncastSafeInterface<'a> + Unsize<T> + Unsize<dyn IBase + 'a> + ?Sized,
    {
        // safety: the pointer stems from the reference so it is always safe
        downcast_interface_mut::<U, _>(self).map(|u| unsafe { &mut *u })
    }
}
