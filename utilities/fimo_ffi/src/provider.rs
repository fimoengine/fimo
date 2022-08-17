//! Implementation of the [`IProvider`] interface.

use crate::{
    interface,
    marshal::CTypeBridge,
    ptr::{DowncastSafe, DowncastSafeInterface, IBase},
    DynObj, ObjArc, ObjBox, ObjWeak, ObjectId, Optional,
};
use std::{fmt::Debug, marker::Unsize};

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "679d7373-a8f2-4d24-92ec-95e7fe01ca60"
    )]

    /// Interface for dynamically provide values.
    pub frozen interface IProvider: marker IBase {
        /// Fullfils a demanded type request.
        fn provide<'a>(&'a self, demand: &mut Demand<'a>);
    }
}

pub use private::{request_interface, request_interface_ref, request_object, request_object_ref};

/// Helper object for providing data by type.
#[repr(transparent)]
pub struct Demand<'a>(DynObj<dyn private::IErased + 'a>);

impl<'a> Demand<'a> {
    /// Provide an object.
    pub fn provide_object<T>(&mut self, fulfil: impl FnOnce() -> T) -> &mut Demand<'a>
    where
        T: CTypeBridge + ObjectContainer<'a>,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_object::<T>)
    }

    /// Provide a reference to an object.
    pub fn provide_object_ref<T>(&mut self, value: &'a T) -> &mut Demand<'a>
    where
        &'a T: CTypeBridge,
        T: ObjectContainer<'a> + ?Sized,
    {
        self.provide_val_impl(|| value, private::IErasedExt::downcast_object_ref::<T>)
    }

    /// Provide an interface.
    pub fn provide_interface<T>(&mut self, fulfil: impl FnOnce() -> T) -> &mut Demand<'a>
    where
        T: CTypeBridge + InterfaceContainer<'a>,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_interface::<T>)
    }

    /// Provide a reference to an interface.
    pub fn provide_interface_ref<T>(&mut self, interface: &'a T) -> &mut Demand<'a>
    where
        &'a T: CTypeBridge,
        T: InterfaceContainer<'a> + ?Sized,
    {
        self.provide_val_impl(
            || interface,
            private::IErasedExt::downcast_interface_ref::<T>,
        )
    }

    fn provide_val_impl<T: CTypeBridge + 'a>(
        &mut self,
        fulfil: impl FnOnce() -> T,
        downcast: impl for<'r> FnOnce(
            &'r mut DynObj<dyn private::IErased + 'a>,
        ) -> Option<&'r mut Optional<T::Type>>,
    ) -> &mut Demand<'a> {
        if let Some(res @ Optional::None) = downcast(&mut self.0) {
            *res = Optional::Some(fulfil().marshal())
        }
        self
    }
}

unsafe impl<'a, 'b> const CTypeBridge for &'b mut Demand<'a> {
    type Type = <&'b mut DynObj<dyn IBase + 'a> as CTypeBridge>::Type;

    fn marshal(self) -> Self::Type {
        self.0.marshal()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let x = <&'b mut DynObj<dyn private::IErased + 'a> as CTypeBridge>::demarshal(x);
        &mut *(x as *mut _ as *mut Demand<'a>)
    }
}

impl<'a> Debug for Demand<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Demand").finish_non_exhaustive()
    }
}

/// Trait for identifying types containing an object like an [`ObjBox`].
///
/// # Safety
///
/// The id of the container must be unique. Furthermore, the layout of the
/// container may not be modified without being assigned a new id.
pub unsafe trait ObjectContainer<'a>: 'a {
    /// Contained object type.
    type Object: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a;

    /// Id of the container.
    const ID: crate::ptr::Uuid;
}

// Implementation for general objects is safe, as even though
// we can not identify the type by the container id, we can still
// use the object id and version.
unsafe impl<'a, T> ObjectContainer<'a> for T
where
    T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
{
    type Object = T;

    const ID: crate::ptr::Uuid = crate::ptr::Uuid::nil();
}

unsafe impl<'a, T> ObjectContainer<'a> for ObjBox<T>
where
    T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
{
    type Object = <T as ObjectContainer<'a>>::Object;

    const ID: crate::ptr::Uuid = uuid::uuid!("15d8d88a-ff19-4243-bd23-f7745089dc37");
}

unsafe impl<'a, T> ObjectContainer<'a> for ObjArc<T>
where
    T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
{
    type Object = <T as ObjectContainer<'a>>::Object;

    const ID: crate::ptr::Uuid = uuid::uuid!("69664012-95c2-487e-b9b2-8dc1237fb6da");
}

unsafe impl<'a, T> ObjectContainer<'a> for ObjWeak<T>
where
    T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
{
    type Object = <T as ObjectContainer<'a>>::Object;

    const ID: crate::ptr::Uuid = uuid::uuid!("62f25b42-fffe-413a-94a8-361e3101c0bf");
}

/// Trait for identifying types containing an interface like an [`ObjBox`].
///
/// # Safety
///
/// The id of the container must be unique. Furthermore, the layout of the
/// container may not be modified without being assigned a new id.
pub unsafe trait InterfaceContainer<'a>: 'a {
    /// Contained interface type.
    type Interface: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a;

    /// Id of the container.
    const ID: crate::ptr::Uuid;
}

unsafe impl<'a, T> InterfaceContainer<'a> for DynObj<T>
where
    T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
{
    type Interface = T;

    const ID: crate::ptr::Uuid = crate::ptr::Uuid::nil();
}

unsafe impl<'a, T> InterfaceContainer<'a> for ObjBox<DynObj<T>>
where
    T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
{
    type Interface = <DynObj<T> as InterfaceContainer<'a>>::Interface;

    const ID: crate::ptr::Uuid = uuid::uuid!("15d8d88a-ff19-4243-bd23-f7745089dc37");
}

unsafe impl<'a, T> InterfaceContainer<'a> for ObjArc<DynObj<T>>
where
    T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
{
    type Interface = <DynObj<T> as InterfaceContainer<'a>>::Interface;

    const ID: crate::ptr::Uuid = uuid::uuid!("69664012-95c2-487e-b9b2-8dc1237fb6da");
}

unsafe impl<'a, T> InterfaceContainer<'a> for ObjWeak<DynObj<T>>
where
    T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
{
    type Interface = <DynObj<T> as InterfaceContainer<'a>>::Interface;

    const ID: crate::ptr::Uuid = uuid::uuid!("62f25b42-fffe-413a-94a8-361e3101c0bf");
}

mod private {
    use std::marker::PhantomData;
    use std::mem::MaybeUninit;

    use crate::marshal::CTypeBridge;
    use crate::ptr::{IBase, ObjInterface, VTableInterfaceInfo, VTableObjectInfo};
    use crate::{interface, ObjectId, Optional};

    use super::{Demand, IProvider, InterfaceContainer, ObjectContainer};

    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, CTypeBridge)]
    pub enum RequestType {
        Object = 0,
        ObjectRef = 1,
        Interface = 2,
        InterfaceRef = 3,
    }

    interface! {
        #![interface_cfg(
            abi(explicit(abi = "C-unwind")),
        )]

        pub frozen interface IErased: marker IBase {
            fn request_type(&self) -> RequestType;
            fn markers(&self) -> Option<usize>;
            fn container_id(&self) -> &[u8; 16];
            fn object_info(&self) -> Option<&VTableObjectInfo>;
            fn interface_info(&self) -> Option<&VTableInterfaceInfo>;
            fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>];
        }
    }

    pub trait IErasedExt<'a>: IErased + 'a {
        fn downcast_object<T>(&mut self) -> Option<&mut Optional<<T as CTypeBridge>::Type>>
        where
            T: CTypeBridge + ObjectContainer<'a>;

        fn downcast_object_ref<T>(&mut self) -> Option<&mut Optional<<&'a T as CTypeBridge>::Type>>
        where
            &'a T: CTypeBridge,
            T: ObjectContainer<'a> + ?Sized;

        fn downcast_interface<T>(&mut self) -> Option<&mut Optional<<T as CTypeBridge>::Type>>
        where
            T: CTypeBridge + InterfaceContainer<'a>;

        fn downcast_interface_ref<T>(
            &mut self,
        ) -> Option<&mut Optional<<&'a T as CTypeBridge>::Type>>
        where
            &'a T: CTypeBridge,
            T: InterfaceContainer<'a> + ?Sized;
    }

    impl<'a, U: IErased + ?Sized + 'a> IErasedExt<'a> for U {
        fn downcast_object<T>(&mut self) -> Option<&mut Optional<<T as CTypeBridge>::Type>>
        where
            T: CTypeBridge + ObjectContainer<'a>,
        {
            match self.request_type() {
                RequestType::Object => {
                    if self.container_id() != T::ID.as_bytes() {
                        return None;
                    }

                    if self.object_info()?.is::<T::Object>() {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Optional<<T as CTypeBridge>::Type>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_object_ref<T>(&mut self) -> Option<&mut Optional<<&'a T as CTypeBridge>::Type>>
        where
            &'a T: CTypeBridge,
            T: ObjectContainer<'a> + ?Sized,
        {
            match self.request_type() {
                RequestType::ObjectRef => {
                    if self.container_id() != T::ID.as_bytes() {
                        return None;
                    }

                    if self.object_info()?.is::<T::Object>() {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Optional<<&'a T as CTypeBridge>::Type>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_interface<T>(&mut self) -> Option<&mut Optional<<T as CTypeBridge>::Type>>
        where
            T: CTypeBridge + InterfaceContainer<'a>,
        {
            match self.request_type() {
                RequestType::Interface => {
                    if self.container_id() != T::ID.as_bytes() {
                        return None;
                    }

                    if self
                        .interface_info()?
                        .is_equal::<T::Interface>(self.markers()?)
                    {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Optional<<T as CTypeBridge>::Type>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_interface_ref<T>(
            &mut self,
        ) -> Option<&mut Optional<<&'a T as CTypeBridge>::Type>>
        where
            &'a T: CTypeBridge,
            T: InterfaceContainer<'a> + ?Sized,
        {
            match self.request_type() {
                RequestType::InterfaceRef => {
                    if self.container_id() != T::ID.as_bytes() {
                        return None;
                    }

                    if self
                        .interface_info()?
                        .is_equal::<T::Interface>(self.markers()?)
                    {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Optional<<&'a T as CTypeBridge>::Type>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    #[derive(ObjectId)]
    #[fetch_vtable(interfaces(IErased))]
    struct ObjectRequest<'a> {
        id: crate::ptr::Uuid,
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjectRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::Object
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
        }

        fn container_id(&self) -> &[u8; 16] {
            self.id.as_bytes()
        }

        fn object_info(&self) -> Option<&VTableObjectInfo> {
            Some(&self.info)
        }

        fn interface_info(&self) -> Option<&VTableInterfaceInfo> {
            None
        }

        fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>] {
            self.result_ptr
        }
    }

    #[derive(ObjectId)]
    #[fetch_vtable(interfaces(IErased))]
    struct ObjectRefRequest<'a> {
        id: crate::ptr::Uuid,
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjectRefRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::ObjectRef
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
        }

        fn container_id(&self) -> &[u8; 16] {
            self.id.as_bytes()
        }

        fn object_info(&self) -> Option<&VTableObjectInfo> {
            Some(&self.info)
        }

        fn interface_info(&self) -> Option<&VTableInterfaceInfo> {
            None
        }

        fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>] {
            self.result_ptr
        }
    }

    #[derive(ObjectId)]
    #[fetch_vtable(interfaces(IErased))]
    struct InterfaceRequest<'a> {
        markers: usize,
        id: crate::ptr::Uuid,
        info: VTableInterfaceInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for InterfaceRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::Interface
        }

        fn markers(&self) -> Option<usize> {
            Some(self.markers)
        }

        fn container_id(&self) -> &[u8; 16] {
            self.id.as_bytes()
        }

        fn object_info(&self) -> Option<&VTableObjectInfo> {
            None
        }

        fn interface_info(&self) -> Option<&VTableInterfaceInfo> {
            Some(&self.info)
        }

        fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>] {
            self.result_ptr
        }
    }

    #[derive(ObjectId)]
    #[fetch_vtable(interfaces(IErased))]
    struct InterfaceRefRequest<'a> {
        markers: usize,
        id: crate::ptr::Uuid,
        info: VTableInterfaceInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for InterfaceRefRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::InterfaceRef
        }

        fn markers(&self) -> Option<usize> {
            Some(self.markers)
        }

        fn container_id(&self) -> &[u8; 16] {
            self.id.as_bytes()
        }

        fn object_info(&self) -> Option<&VTableObjectInfo> {
            None
        }

        fn interface_info(&self) -> Option<&VTableInterfaceInfo> {
            Some(&self.info)
        }

        fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>] {
            self.result_ptr
        }
    }

    /// Requests an object from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use fimo_ffi::marshal::CTypeBridge;
    /// use fimo_ffi::{ObjBox, ObjArc, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_object};
    ///
    /// #[derive(CTypeBridge, ObjectId)]
    /// #[fetch_vtable(uuid = "82eeae84-5c20-46e6-8314-89d03b5a6766")]
    /// struct A(bool);
    ///
    /// #[derive(CTypeBridge, ObjectId)]
    /// #[fetch_vtable(uuid = "2d73c0c5-7d35-4473-8d22-1a7168f710c7")]
    /// struct B(usize);
    ///
    /// #[derive(CTypeBridge, ObjectId)]
    /// #[fetch_vtable(uuid = "18d6157b-7cb5-4a55-ae66-05e985921db1")]
    /// struct C(f32);
    ///
    /// struct Provider;
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object(|| A(true))
    ///               .provide_object(|| ObjBox::new(B(32)));
    ///     }
    /// }
    ///
    /// let a = request_object::<A>(&Provider).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// assert!(request_object::<ObjBox<A>>(&Provider).is_none());
    /// assert!(request_object::<ObjArc<A>>(&Provider).is_none());
    ///
    /// let b = request_object::<ObjBox<B>>(&Provider).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_object::<C>(&Provider).is_none());
    /// ```
    pub fn request_object<'a, T>(provider: &'a impl IProvider) -> Option<T>
    where
        T: CTypeBridge + ObjectContainer<'a>,
    {
        let mut result = MaybeUninit::new(Optional::None);
        let mut request = ObjectRequest {
            id: T::ID,
            info: VTableObjectInfo::new::<T::Object>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { Option::demarshal(result.assume_init()) }
    }

    /// Requests an object reference from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// use fimo_ffi::{ObjBox, ObjArc, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_object_ref};
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "82eeae84-5c20-46e6-8314-89d03b5a6766")]
    /// struct A(bool);
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "2d73c0c5-7d35-4473-8d22-1a7168f710c7")]
    /// struct B(usize);
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "18d6157b-7cb5-4a55-ae66-05e985921db1")]
    /// struct C(f32);
    ///
    /// struct Provider(A, ObjBox<B>, ObjArc<C>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object_ref(&self.0)
    ///               .provide_object_ref(&self.1);
    ///     }
    /// }
    ///
    /// let a = A(true);
    /// let b = ObjBox::new(B(32));
    /// let c = ObjArc::new(C(0.0));
    /// let p = Provider(a, b, c);
    ///
    /// let a = request_object_ref::<A>(&p).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// assert!(request_object_ref::<ObjBox<A>>(&p).is_none());
    /// assert!(request_object_ref::<ObjArc<A>>(&p).is_none());
    ///
    /// let b = request_object_ref::<ObjBox<B>>(&p).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_object_ref::<ObjArc<C>>(&p).is_none());
    /// ```
    pub fn request_object_ref<'a, T>(provider: &'a impl IProvider) -> Option<&'a T>
    where
        &'a T: CTypeBridge,
        T: ObjectContainer<'a> + ?Sized,
    {
        let mut result: MaybeUninit<Optional<<&'a T as CTypeBridge>::Type>> =
            MaybeUninit::new(Optional::None);
        let mut request = ObjectRefRequest {
            id: T::ID,
            info: VTableObjectInfo::new::<T::Object>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { Option::demarshal(result.assume_init()) }
    }

    /// Requests an interface from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use std::marker::PhantomData;
    /// use fimo_ffi::{ObjBox, ObjArc};
    /// use fimo_ffi::ptr::{coerce_obj, IBase};
    /// use fimo_ffi::{interface, ObjectId, DynObj};
    /// use fimo_ffi::provider::{IProvider, Demand, request_interface};
    ///
    /// #[derive(Copy, Clone, ObjectId)]
    /// #[fetch_vtable(uuid = "82eeae84-5c20-46e6-8314-89d03b5a6766", interfaces(IA))]
    /// struct A(bool);
    ///
    /// #[derive(Copy, Clone, ObjectId)]
    /// #[fetch_vtable(uuid = "2d73c0c5-7d35-4473-8d22-1a7168f710c7", interfaces(IB))]
    /// struct B(usize);
    ///
    /// #[derive(Copy, Clone, ObjectId)]
    /// #[fetch_vtable(uuid = "18d6157b-7cb5-4a55-ae66-05e985921db1", interfaces(IC))]
    /// struct C(f32, PhantomData<*const ()>);
    ///
    /// impl IA for A {
    ///     fn val(&self) -> bool {
    ///         self.0
    ///     }
    /// }
    ///
    /// impl IB for B {
    ///     fn val(&self) -> usize {
    ///         self.0
    ///     }
    /// }
    ///
    /// impl IC for C {
    ///     fn val(&self) -> f32 {
    ///         self.0
    ///     }
    /// }
    ///
    /// interface! {
    ///     #![interface_cfg(uuid = "82eeae84-5c20-46e6-8314-89d03b5a6766")]
    ///     interface IA: marker IBase {
    ///         fn val(&self) -> bool;
    ///     }
    /// }
    ///
    /// interface! {
    ///     #![interface_cfg(uuid = "2d73c0c5-7d35-4473-8d22-1a7168f710c7")]
    ///     interface IB: marker IBase {
    ///         fn val(&self) -> usize;
    ///     }
    /// }
    ///
    /// interface! {
    ///     #![interface_cfg(uuid = "18d6157b-7cb5-4a55-ae66-05e985921db1")]
    ///     interface IC: marker IBase {
    ///         fn val(&self) -> f32;
    ///     }
    /// }
    ///
    /// struct Provider(ObjBox<B>, ObjArc<DynObj<dyn IC + Unpin>>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_interface::<ObjBox<DynObj<dyn IB + Send + Sync>>>(|| ObjBox::coerce_obj(self.0.clone()))
    ///               .provide_interface::<ObjArc<DynObj<dyn IC + Unpin>>>(|| self.1.clone());
    ///     }
    /// }
    ///
    /// let b = ObjBox::new(B(32));
    /// let c = ObjArc::coerce_obj(ObjArc::new(C(0.0, PhantomData)));
    /// let p = Provider(b, c);
    ///
    /// assert!(request_interface::<ObjBox<DynObj<dyn IA>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IA>>>(&p).is_none());
    ///
    /// let b = request_interface::<ObjBox<DynObj<dyn IB + Send + Sync>>>(&p).unwrap();
    /// assert_eq!(b.val(), 32);
    ///
    /// // Provider provides a `ObjBox<DynObj<dyn IB + Send + Sync>>`
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB>>>(&p).is_none());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Send>>>(&p).is_none());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Sync>>>(&p).is_none());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Unpin>>>(&p).is_none());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Send + Sync>>>(&p).is_some());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Send + Unpin>>>(&p).is_none());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Sync + Unpin>>>(&p).is_none());
    /// assert!(request_interface::<ObjBox<DynObj<dyn IB + Send + Sync + Unpin>>>(&p).is_none());
    ///
    /// let c = request_interface::<ObjArc<DynObj<dyn IC + Unpin>>>(&p).unwrap();
    /// assert_eq!(c.val(), 0.0);
    ///
    /// // Provider provides a `DynObj<dyn IC + Unpin>`
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Send>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Sync>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Unpin>>>(&p).is_some());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Send + Sync>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Send + Unpin>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Sync + Unpin>>>(&p).is_none());
    /// assert!(request_interface::<ObjArc<DynObj<dyn IC + Send + Sync + Unpin>>>(&p).is_none());
    /// ```
    pub fn request_interface<'a, T>(provider: &'a impl IProvider) -> Option<T>
    where
        T: CTypeBridge + InterfaceContainer<'a>,
    {
        let mut result = MaybeUninit::new(Optional::None);
        let mut request = InterfaceRequest {
            id: T::ID,
            markers: <T::Interface as ObjInterface>::MARKER_BOUNDS,
            info: VTableInterfaceInfo::new::<T::Interface>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { Option::demarshal(result.assume_init()) }
    }

    /// Requests an interface reference from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use std::marker::PhantomData;
    /// use fimo_ffi::{ObjBox, ObjArc};
    /// use fimo_ffi::ptr::{coerce_obj, IBase};
    /// use fimo_ffi::{interface, ObjectId, DynObj};
    /// use fimo_ffi::provider::{IProvider, Demand, request_interface_ref};
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "82eeae84-5c20-46e6-8314-89d03b5a6766", interfaces(IA))]
    /// struct A(bool);
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "2d73c0c5-7d35-4473-8d22-1a7168f710c7", interfaces(IB))]
    /// struct B(usize);
    ///
    /// #[derive(ObjectId)]
    /// #[fetch_vtable(uuid = "18d6157b-7cb5-4a55-ae66-05e985921db1", interfaces(IC))]
    /// struct C(f32, PhantomData<*const ()>);
    ///
    /// impl IA for A {
    ///     fn val(&self) -> bool {
    ///         self.0
    ///     }
    /// }
    ///
    /// impl IB for B {
    ///     fn val(&self) -> usize {
    ///         self.0
    ///     }
    /// }
    ///
    /// impl IC for C {
    ///     fn val(&self) -> f32 {
    ///         self.0
    ///     }
    /// }
    ///
    /// interface! {
    ///     #![interface_cfg(uuid = "82eeae84-5c20-46e6-8314-89d03b5a6766")]
    ///     interface IA: marker IBase {
    ///         fn val(&self) -> bool;
    ///     }
    /// }
    ///
    /// interface! {
    ///     #![interface_cfg(uuid = "2d73c0c5-7d35-4473-8d22-1a7168f710c7")]
    ///     interface IB: marker IBase {
    ///         fn val(&self) -> usize;
    ///     }
    /// }
    ///
    /// interface! {
    ///     #![interface_cfg(uuid = "18d6157b-7cb5-4a55-ae66-05e985921db1")]
    ///     interface IC: marker IBase {
    ///         fn val(&self) -> f32;
    ///     }
    /// }
    ///
    /// struct Provider(A, ObjBox<DynObj<dyn IB + Send + Sync>>, ObjArc<DynObj<dyn IC + Unpin>>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_interface_ref::<DynObj<dyn IA>>(coerce_obj(&self.0))
    ///               .provide_interface_ref::<ObjBox<DynObj<dyn IB + Send + Sync>>>(&self.1)
    ///               .provide_interface_ref::<ObjArc<DynObj<dyn IC + Unpin>>>(&self.2);
    ///     }
    /// }
    ///
    /// let a = A(true);
    /// let b = ObjBox::coerce_obj(ObjBox::new(B(32)));
    /// let c = ObjArc::coerce_obj(ObjArc::new(C(0.0, PhantomData)));
    /// let p = Provider(a, b, c);
    ///
    /// let a = request_interface_ref::<DynObj<dyn IA>>(&p).unwrap();
    /// assert_eq!(a.val(), true);
    ///
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IA>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IA>>>(&p).is_none());
    ///
    /// let b = request_interface_ref::<ObjBox<DynObj<dyn IB + Send + Sync>>>(&p).unwrap();
    /// assert_eq!(b.val(), 32);
    ///
    /// // Provider provides a `ObjBox<DynObj<dyn IB + Send + Sync>>`
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Send>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Sync>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Unpin>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Send + Sync>>>(&p).is_some());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Send + Unpin>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Sync + Unpin>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjBox<DynObj<dyn IB + Send + Sync + Unpin>>>(&p).is_none());
    ///
    /// let c = request_interface_ref::<ObjArc<DynObj<dyn IC + Unpin>>>(&p).unwrap();
    /// assert_eq!(c.val(), 0.0);
    ///
    /// // Provider provides a `DynObj<dyn IC + Unpin>`
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Send>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Sync>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Unpin>>>(&p).is_some());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Send + Sync>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Send + Unpin>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Sync + Unpin>>>(&p).is_none());
    /// assert!(request_interface_ref::<ObjArc<DynObj<dyn IC + Send + Sync + Unpin>>>(&p).is_none());
    /// ```
    pub fn request_interface_ref<'a, T>(provider: &'a impl IProvider) -> Option<&'a T>
    where
        &'a T: CTypeBridge,
        T: InterfaceContainer<'a> + ?Sized,
    {
        let mut result: MaybeUninit<Optional<<&'a T as CTypeBridge>::Type>> =
            MaybeUninit::new(Optional::None);
        let mut request = InterfaceRefRequest {
            id: T::ID,
            markers: <T::Interface as ObjInterface>::MARKER_BOUNDS,
            info: VTableInterfaceInfo::new::<T::Interface>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { Option::demarshal(result.assume_init()) }
    }
}
