//! Object pointer implementation.

use crate::ConstStr;
use std::alloc::Layout;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::{PhantomData, Unsize};
use std::ptr::NonNull;

pub use uuid::Uuid;

/// Marks that the layout of a type is compatible with an [`ObjMetadata`].
///
/// # Safety
///
/// This trait can be implemented only if the type is prefixed
/// with the same members of the internal vtable head and is
/// laid out using the system C abi.
pub unsafe trait ObjMetadataCompatible<'a>: 'static {}

/// Specifies a new object type.
pub trait ObjectId {
    /// Unique id of the object.
    const OBJECT_ID: Uuid;

    /// Name of the object.
    const OBJECT_NAME: &'static str = std::any::type_name::<Self>();
}

/// Specifies a new interface type.
pub trait ObjMetadataInterface<'a>: 'a {
    /// VTable of the interface.
    type VTable: ObjMetadataCompatible<'a>;

    /// Unique id of the interface.
    const INTERFACE_ID: Uuid;

    /// Name of the interface.
    const INTERFACE_NAME: &'static str = std::any::type_name::<Self>();
}

/// Indicated that a type is usable with a [`DynObj`].
pub trait ObjMetadataInterfaceCompatibility<'a>: 'a {
    /// Base type that specifies the used vtable.
    type Base: ObjMetadataInterface<'a> + ?Sized;
}

/// Indicates that an object can be coerced to a [`DynObj`].
pub trait FetchVTable<'a, Dyn>: ObjectId + Unsize<Dyn>
where
    Dyn: ObjMetadataInterface<'a> + ?Sized,
{
    /// Returns a static reference to the vtable describing the interface and object.
    fn fetch_interface() -> &'static Dyn::VTable;
}

/// Trait for upcasting a vtable.
///
/// The upcasting operation must maintain the ids and names of the original object and interface.
///
/// Example:
///
/// ```
/// #![feature(const_fn_fn_ptr_basics)]
/// #![feature(const_fn_trait_bound)]
/// #![feature(unsize)]
///
/// use fimo_object::ptr::{IBase, ObjMetadata};
/// use fimo_object::{base_vtable, base_interface, impl_upcast};
///
/// base_interface!{
///     #![vtable = AVTable]
///     #![uuid(0x1, 0x1, 0x1, 0x1, 0x1)]
///     trait A: (IBase) {
///         // trait definition
///     }
/// }
///
/// base_interface!{
///     #![vtable = BVTable]
///     #![uuid(0x2, 0x2, 0x2, 0x2, 0x2)]
///     trait B: (IBase) {
///         // trait definition
///     }
/// }
///
/// base_interface!{
///     #![vtable = VTable]
///     #![uuid(0x3, 0x3, 0x3, 0x3, 0x3)]
///     trait C: (A + B) {
///         // trait definition
///     }
/// }
///
/// base_vtable!{
///     #![interface = A]
///     struct AVTable {}
/// }
///
/// base_vtable!{
///     #![interface = B]
///     struct BVTable {}
/// }
///
/// base_vtable!{
///     // the macro adds a VTableHead to the start of the structure.
///     #![interface = C]
///     struct VTable {
///         // the content of the head of `VTable`, `a_vtable` and `b_vtable`
///         // must be identical except for the offset member which specifies
///         // the offset from the start of the head to the start of the `VTable`
///         // structure. For `a_vtable` it is `sizeof(VTableHead) + align` and for
///         // `b_vtable` it is `sizeof(VTableHead) + align + sizeof(AVTable) + align`.
///         // Downcasting i.e. retrieving a `VTable` from either a `AVTable` or `BVTable`
///         // is then a simple pointer offset:
///         // `(vtable as *const u8).wrapping_sub(offset) as *const VTable`
///         a_vtable: AVTable,
///         b_vtable: BVTable,
///         // other members
///     }
/// }
///
/// impl_upcast!{
///     impl (C) -> (A) obj: ObjMetadata<_> {
///         let vtable = obj.vtable();
///         let a_vtable = &vtable.a_vtable;
///         ObjMetadata::new(a_vtable)
///     }
/// }
///
/// impl_upcast!{
///     impl (C) -> (B) obj: ObjMetadata<_> {
///         let vtable = obj.vtable();
///         let b_vtable = &vtable.b_vtable;
///         ObjMetadata::new(b_vtable)
///     }
/// }
/// ```
pub trait CastSuper<'a, Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized>: Unsize<Dyn> {
    /// Retrieves a super vtable to the same object.
    fn cast_super(obj: ObjMetadata<Self>) -> ObjMetadata<Dyn>;
}

/// The metadata for a `Dyn = dyn SomeTrait` object type.
#[repr(transparent)]
pub struct ObjMetadata<Dyn: ?Sized> {
    vtable_ptr: &'static VTableHead,
    phantom: PhantomData<Dyn>,
}

impl<'a, Dyn: 'a + ?Sized> ObjMetadata<Dyn> {
    /// Constructs a new `ObjMetadata` with a given vtable.
    #[inline]
    pub fn new(vtable: &'static <Dyn::Base as ObjMetadataInterface<'a>>::VTable) -> Self
    where
        Dyn: ObjMetadataInterfaceCompatibility<'a>,
    {
        Self {
            // safety: the safety is guaranteed with the
            // implementation of ObjMetadataCompatible.
            vtable_ptr: unsafe { std::mem::transmute(vtable) },
            phantom: Default::default(),
        }
    }

    /// Returns a vtable that is compatible with the current interface.
    #[inline]
    pub fn vtable(self) -> &'static <Dyn::Base as ObjMetadataInterface<'a>>::VTable
    where
        Dyn: ObjMetadataInterfaceCompatibility<'a>,
    {
        // safety: the safety is guaranteed with the
        // implementation of ObjMetadataCompatible.
        unsafe { std::mem::transmute(self.vtable_ptr) }
    }

    /// Returns if the contained type matches.
    #[inline]
    pub fn is<U>(self) -> bool
    where
        U: ObjectId + Unsize<Dyn>,
    {
        self.object_id() == U::OBJECT_ID
    }

    /// Returns the super vtable.
    #[inline]
    pub fn cast_super<U>(self) -> ObjMetadata<U>
    where
        Dyn: CastSuper<'a, U>,
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        CastSuper::cast_super(self)
    }

    /// Returns if the a certain interface is implemented.
    #[inline]
    pub fn is_interface<U>(self) -> bool
    where
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        self.interface_id() == U::Base::INTERFACE_ID
    }

    /// Returns the metadata for the contained interface if it is of type `U`.
    #[inline]
    pub fn downcast_interface<U>(self) -> Option<ObjMetadata<U>>
    where
        U: ObjMetadataInterfaceCompatibility<'a> + Unsize<Dyn> + ?Sized,
    {
        if self.is_interface::<U>() {
            let vtable_ptr = self.vtable_ptr as *const VTableHead as *const u8;
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
            .field(&(self.vtable_ptr as *const VTableHead))
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
    let metadata: usize = std::ptr::metadata(ptr);
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
pub fn coerce_obj<'a, T, Dyn>(obj: &T) -> &DynObj<Dyn>
where
    T: FetchVTable<'a, Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
{
    unsafe { &*coerce_obj_raw(obj) }
}

/// Coerces a object pointer to a [`DynObj`] pointer.
#[inline]
pub fn coerce_obj_raw<'a, T, Dyn>(obj: *const T) -> *const DynObj<Dyn>
where
    T: FetchVTable<'a, Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
{
    let vtable = T::fetch_interface();
    let metadata = ObjMetadata::<Dyn>::new(vtable);
    from_raw_parts(obj as *const (), metadata)
}

/// Coerces a mutable object reference to a [`DynObj`] reference.
#[inline]
pub fn coerce_obj_mut<'a, T, Dyn>(obj: &mut T) -> &mut DynObj<Dyn>
where
    T: FetchVTable<'a, Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
{
    unsafe { &mut *coerce_obj_mut_raw(obj) }
}

/// Coerces a mutable object pointer to a [`DynObj`] pointer.
#[inline]
pub fn coerce_obj_mut_raw<'a, T, Dyn>(obj: *mut T) -> *mut DynObj<Dyn>
where
    T: FetchVTable<'a, Dyn::Base> + Unsize<Dyn>,
    Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
{
    let vtable = T::fetch_interface();
    let metadata = ObjMetadata::<Dyn>::new(vtable);
    from_raw_parts_mut(obj as *mut (), metadata)
}

// pointer info

/// Executes the destructor of the contained object.
///
/// # Safety
///
/// See [`std::ptr::drop_in_place()`];
pub unsafe fn drop_in_place<Dyn: ?Sized>(obj: *mut DynObj<Dyn>) {
    let metadata = metadata(obj);
    if let Some(drop) = metadata.vtable_ptr.__internal_drop_in_place {
        (drop)(obj as *mut ())
    }
}

/// Returns the size of the type associated with this vtable.
pub fn size_of_val<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> usize {
    let metadata = metadata(obj);
    metadata.size_of()
}

/// Retrieves the alignment of the object.
pub fn align_of_val<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> usize {
    let metadata = metadata(obj);
    metadata.align_of()
}

/// Returns the id of the type associated with this vtable.
pub fn object_id<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> crate::ptr::Uuid {
    let metadata = metadata(obj);
    metadata.object_id()
}

/// Returns the name of the type associated with this vtable.
pub fn object_name<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> &'static str {
    let metadata = metadata(obj);
    metadata.object_name()
}

/// Returns the id of the interface implemented with this vtable.
pub fn interface_id<Dyn: ?Sized>(obj: *const DynObj<Dyn>) -> crate::ptr::Uuid {
    let metadata = metadata(obj);
    metadata.interface_id()
}

/// Returns the name of the interface implemented with this vtable.
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
        Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        Self::new_embedded::<'a, T, Dyn>(0)
    }

    /// Constructs a new vtable with a custom offset.
    pub const fn new_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        let drop = if std::mem::needs_drop::<T>() { Some(drop_obj_in_place::<T> as _) } else { None };

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

unsafe impl<'a> ObjMetadataCompatible<'a> for VTableHead {}

/// Creates a new vtable for a given trait.
#[macro_export]
macro_rules! base_interface {
    (
        $(#[$attr:meta])*
        #![vtable = $vtable:ty]
        #![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)]
        $vis:vis trait $name:ident $(< $($gen:lifetime),+ >)? : ( $($req:tt)* )  {
            $( $body:tt )*
        }
    ) => {
        $(#[$attr])*
        $vis trait $name $(< $($gen),+ >)? : $($req)*
        {
            $($body)*
        }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? ($name $(< $($gen),+ >)? ) -> ($name $(< $($gen),+ >)?) obj: ObjMetadata<_> { unsafe { std::mem::transmute(obj) } } }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterface<'inner> for dyn $name $(< $($gen),+ >)? + 'inner {
            type VTable = $vtable;
            const INTERFACE_ID: $crate::ptr::Uuid = $crate::ptr::new_uuid($u1, $u2, $u3, (($u4 as u64) << 48) | $u5 as u64);
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Send + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Sync + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Unpin + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Send + Sync + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Send + Unpin + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Sync + Unpin + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }

        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> for dyn $name $(< $($gen),+ >)? + Send + Sync + Unpin + 'inner {
            type Base = dyn $name $(< $($gen),+ >)? + 'inner;
        }
    };
}

/// Implements the necessary traits for coercing a type to a [`IBase`] object.
#[macro_export]
macro_rules! base_object {
    (#![uuid($u1:literal, $u2:literal, $u3:literal, $u4:literal, $u5:literal)] $name:ty) => {
        impl $crate::ptr::ObjectId for $name {
            const OBJECT_ID: $crate::ptr::Uuid =
                $crate::ptr::new_uuid($u1, $u2, $u3, (($u4 as u64) << 48) | $u5 as u64);
        }

        impl $crate::ptr::IBase for $name {}

        impl<'a> $crate::ptr::FetchVTable<'a, dyn $crate::ptr::IBase + 'a> for $name {
            fn fetch_interface() -> &'static $crate::ptr::IBaseVTable {
                static VTABLE: $crate::ptr::IBaseVTable = $crate::ptr::new_ibase_vtable::<$name>();
                &VTABLE
            }
        }
    };
}

/// Helper trait for implementing trait upcasting for all possible combinations.
#[macro_export]
macro_rules! impl_upcast {
    (impl $(< $($gen:lifetime),+ >)? ($($source:tt)*) -> ($($dest:tt)*) $obj:ident : ObjMetadata<_> $body:block) => {
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)*) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Sync) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Unpin) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Unpin) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Sync + Unpin) -> (dyn $($dest)*) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)*) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send) -> (dyn $($dest)* + Send) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync) -> (dyn $($dest)* + Send) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Unpin) -> (dyn $($dest)* + Send) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Send) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Sync) -> (dyn $($dest)* + Sync) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync) -> (dyn $($dest)* + Sync) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Sync + Unpin) -> (dyn $($dest)* + Sync) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Sync) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Unpin) -> (dyn $($dest)* + Unpin) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Unpin) -> (dyn $($dest)* + Unpin) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Sync + Unpin) -> (dyn $($dest)* + Unpin) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Unpin) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync) -> (dyn $($dest)* + Send + Sync) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Send + Sync) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Unpin) -> (dyn $($dest)* + Send + Unpin) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Send + Unpin) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Sync + Unpin) -> (dyn $($dest)* + Sync + Unpin) $obj $body }
        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Sync + Unpin) $obj $body }

        $crate::impl_upcast!{ impl $(< $($gen),+ >)? inner (dyn $($source)* + Send + Sync + Unpin) -> (dyn $($dest)* + Send + Sync + Unpin) $obj $body }
    };
    (impl $(< $($gen:lifetime),+ >)? inner ($($source:tt)*) -> ($($dest:tt)*) $obj:ident $body:block) => {
        impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::CastSuper<'inner, $($dest)* + 'inner> for $($source)* + 'inner {
            fn cast_super($obj: ObjMetadata<$($source)* + 'inner>) -> ObjMetadata<$($dest)* + 'inner> {
                $body
            }
        }
    };
}

macro_rules! impl_ibase_upcast {
    () => {
        impl_ibase_upcast!{ impl inner (dyn IBase) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Send) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Sync) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Unpin) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Send + Sync) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Send + Unpin) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Sync + Unpin) }
        impl_ibase_upcast!{ impl inner (dyn IBase + Send + Sync + Unpin) }
    };
    (impl inner ($($dest:tt)*)) => {
        impl<'a, 'b: 'a, T: Unsize<$($dest)* + 'a> + ?Sized + 'b> $crate::ptr::CastSuper<'a, $($dest)* + 'a> for T {
            default fn cast_super(obj: ObjMetadata<T>) -> ObjMetadata<$($dest)* + 'a> {
                unsafe { std::mem::transmute(obj) }
            }
        }
    };
}

/// Creates a vtable compatible with the [`IBase`] interface.
#[macro_export]
macro_rules! base_vtable {
    // struct with named fields
    (
        $(#[$attr:meta])*
        #![interface = $trait:tt]
        $vis:vis struct $name:ident $(< $($gen:lifetime),+ >)? {
            $(
                $(#[$elem_attr:meta])* $elem_vis:vis $elem:ident: $elem_ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        #[repr(C)]
        #[allow(missing_debug_implementations)]
        $vis struct $name $(< $($gen),+ >)? {
            /// Common head of the vtable.
            pub __internal_head: $crate::ptr::VTableHead,
            /// Marker for the contained trait.
            pub __internal_phantom: std::marker::PhantomData<for<'inner> fn(*const (dyn $trait + 'inner))>,
            $($(#[$elem_attr])* $elem_vis $elem: $elem_ty),*
        }

        impl<'inner $(, $($gen:'inner),+)?> $name $(< $($gen),+ >)? {
            /// Constructs a new instance of the vtable.
            #[allow(clippy::type_complexity)]
            #[allow(clippy::too_many_arguments)]
            pub const fn new<T>($($elem: $elem_ty),*) -> Self
            where T: $trait + $crate::ptr::ObjectId + 'inner
            {
                Self::new_embedded::<T, dyn $trait + 'inner>(0, $($elem),*)
            }

            /// Constructs a new instance of the vtable when embedded into another vtable.
            #[allow(clippy::type_complexity)]
            #[allow(clippy::too_many_arguments)]
            pub const fn new_embedded<T, Dyn>(internal_offset: usize, $($elem: $elem_ty),*) -> Self
            where
                T: $crate::ptr::ObjectId + std::marker::Unsize<Dyn> + 'inner,
                Dyn: $crate::ptr::ObjMetadataInterfaceCompatibility<'inner> + ?Sized
            {
                Self {
                    __internal_head: $crate::ptr::VTableHead::new_embedded::<'inner, T, Dyn>(internal_offset),
                    __internal_phantom: std::marker::PhantomData,
                    $($elem),*
                }
            }
        }

        unsafe impl<'inner $(, $($gen:'inner),+)?> $crate::ptr::ObjMetadataCompatible<'inner> for $name $(< $($gen),+ >)? {}
    };
}

/// Drops the pointed to value.
///
/// # Safety
///
/// See [std::ptr::drop_in_place].
pub unsafe extern "C-unwind" fn drop_obj_in_place<T>(ptr: *mut ()) {
    std::ptr::drop_in_place::<T>(ptr as *mut T)
}

/// Constructs a new [`Uuid`].
pub const fn new_uuid(d1: u32, d2: u16, d3: u16, d4: u64) -> Uuid {
    let d4 = d4.to_be_bytes();
    Uuid::from_fields(d1, d2, d3, &d4)
}

base_interface! {
    /// Base trait for all objects.
    #![vtable = IBaseVTable]
    #![uuid(0x0, 0x0, 0x0, 0x0, 0x0)]
    pub trait IBase: () {}
}

base_vtable! {
    /// VTable of the [`IBase`] trait.
    #[derive(Copy, Clone)]
    #![interface = IBase]
    pub struct IBaseVTable {}
}

impl_ibase_upcast! {}

/// Helper trait for a [`DynObj<dyn IBase>`].
pub trait DynIBase<'a, Dyn: IBase + ?Sized + 'a> {
    /// Returns if the contained type matches.
    fn is<U>(&self) -> bool
    where
        U: ObjectId + Unsize<Dyn>;

    /// Returns the downcasted object if it is of type `U`.
    fn downcast<U>(&self) -> Option<&U>
    where
        U: ObjectId + Unsize<Dyn>;

    /// Returns the mutable downcasted object if it is of type `U`.
    fn downcast_mut<U>(&mut self) -> Option<&mut U>
    where
        U: ObjectId + Unsize<Dyn>;

    /// Returns the super object.
    fn cast_super<U>(&self) -> &DynObj<U>
    where
        Dyn: CastSuper<'a, U>,
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized;

    /// Returns the mutable super object.
    fn cast_super_mut<U>(&mut self) -> &mut DynObj<U>
    where
        Dyn: CastSuper<'a, U>,
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized;

    /// Returns if the a certain interface is implemented.
    fn is_interface<U>(&self) -> bool
    where
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized;

    /// Returns the downcasted interface if it is of type `U`.
    fn downcast_interface<U>(&self) -> Option<&DynObj<U>>
    where
        U: ObjMetadataInterfaceCompatibility<'a> + Unsize<Dyn> + ?Sized;

    /// Returns the mutable downcasted interface if it is of type `U`.
    fn downcast_interface_mut<U>(&mut self) -> Option<&mut DynObj<U>>
    where
        U: ObjMetadataInterfaceCompatibility<'a> + Unsize<Dyn> + ?Sized;
}

impl<T: IBase + ?Sized> IBase for DynObj<T> {}

impl<'a, T: IBase + ?Sized + 'a> DynIBase<'a, T> for DynObj<T> {
    #[inline]
    fn is<U>(&self) -> bool
    where
        U: ObjectId + Unsize<T>,
    {
        let metadata = metadata(self);
        metadata.is::<U>()
    }

    #[inline]
    fn downcast<U>(&self) -> Option<&U>
    where
        U: ObjectId + Unsize<T>,
    {
        if self.is::<U>() {
            unsafe { Some(&*(self as *const _ as *const U)) }
        } else {
            None
        }
    }

    #[inline]
    fn downcast_mut<U>(&mut self) -> Option<&mut U>
    where
        U: ObjectId + Unsize<T>,
    {
        if self.is::<U>() {
            unsafe { Some(&mut *(self as *mut _ as *mut U)) }
        } else {
            None
        }
    }

    #[inline]
    fn cast_super<U>(&self) -> &DynObj<U>
    where
        T: CastSuper<'a, U>,
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        let metadata = metadata(self);
        let metadata = metadata.cast_super::<U>();
        unsafe { &*(from_raw_parts(self as *const _ as *const (), metadata)) }
    }

    #[inline]
    fn cast_super_mut<U>(&mut self) -> &mut DynObj<U>
    where
        T: CastSuper<'a, U>,
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        let metadata = metadata(self);
        let metadata = metadata.cast_super::<U>();
        unsafe { &mut *(from_raw_parts_mut(self as *mut _ as *mut (), metadata)) }
    }

    #[inline]
    fn is_interface<U>(&self) -> bool
    where
        U: ObjMetadataInterfaceCompatibility<'a> + ?Sized,
    {
        let metadata = metadata(self);
        metadata.is_interface::<U>()
    }

    #[inline]
    fn downcast_interface<U>(&self) -> Option<&DynObj<U>>
    where
        U: ObjMetadataInterfaceCompatibility<'a> + Unsize<T> + ?Sized,
    {
        let metadata = metadata(self);
        if let Some(metadata) = metadata.downcast_interface::<U>() {
            unsafe { Some(&*(from_raw_parts(self as *const _ as *const (), metadata))) }
        } else {
            None
        }
    }

    #[inline]
    fn downcast_interface_mut<U>(&mut self) -> Option<&mut DynObj<U>>
    where
        U: ObjMetadataInterfaceCompatibility<'a> + Unsize<T> + ?Sized,
    {
        let metadata = metadata(self);
        if let Some(metadata) = metadata.downcast_interface::<U>() {
            unsafe { Some(&mut *(from_raw_parts_mut(self as *mut _ as *mut (), metadata))) }
        } else {
            None
        }
    }
}
