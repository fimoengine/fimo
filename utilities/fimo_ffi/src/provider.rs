//! Implementation of the [`IProvider`] interface.

use crate::{
    interface,
    marshal::CTypeBridge,
    ptr::{DowncastSafe, DowncastSafeInterface, IBase},
    DynObj, ObjArc, ObjBox, ObjWeak, ObjectId,
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

pub use private::{
    request_interface, request_interface_arc, request_interface_box, request_interface_weak,
    request_object, request_object_arc, request_object_box, request_object_ref,
    request_object_weak,
};

/// Helper object for providing data by type.
#[repr(transparent)]
pub struct Demand<'a>(DynObj<dyn private::IErased + 'a>);

impl<'a> Demand<'a> {
    /// Provide an object.
    pub fn provide_object<T>(&mut self, fulfil: impl FnOnce() -> T) -> &mut Demand<'a>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_object::<T>)
    }

    /// Provide a reference to an object.
    pub fn provide_object_ref<T>(&mut self, value: &'a T) -> &mut Demand<'a>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        self.provide_val_impl(|| value, private::IErasedExt::downcast_object_ref::<T>)
    }

    /// Provide an object box.
    pub fn provide_object_box<T>(&mut self, fulfil: impl FnOnce() -> ObjBox<T>) -> &mut Demand<'a>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_object_box::<T>)
    }

    /// Provide an object arc.
    pub fn provide_object_arc<T>(&mut self, fulfil: impl FnOnce() -> ObjArc<T>) -> &mut Demand<'a>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_object_arc::<T>)
    }

    /// Provide an object weak.
    pub fn provide_object_weak<T>(&mut self, fulfil: impl FnOnce() -> ObjWeak<T>) -> &mut Demand<'a>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_object_weak::<T>)
    }

    /// Provide a reference to an interface.
    pub fn provide_interface<T>(&mut self, interface: &'a DynObj<T>) -> &mut Demand<'a>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        self.provide_val_impl(|| interface, private::IErasedExt::downcast_interface::<T>)
    }

    /// Provide a [`ObjBox`] to an interface.
    pub fn provide_interface_box<T>(
        &mut self,
        fulfil: impl FnOnce() -> ObjBox<DynObj<T>>,
    ) -> &mut Demand<'a>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_interface_box::<T>)
    }

    /// Provide a [`ObjArc`] to an interface.
    pub fn provide_interface_arc<T>(
        &mut self,
        fulfil: impl FnOnce() -> ObjArc<DynObj<T>>,
    ) -> &mut Demand<'a>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_interface_arc::<T>)
    }

    /// Provide a [`ObjWeak`] to an interface.
    pub fn provide_interface_weak<T>(
        &mut self,
        fulfil: impl FnOnce() -> ObjWeak<DynObj<T>>,
    ) -> &mut Demand<'a>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_interface_weak::<T>)
    }

    fn provide_val_impl<T: 'a>(
        &mut self,
        fulfil: impl FnOnce() -> T,
        downcast: impl for<'r> FnOnce(
            &'r mut DynObj<dyn private::IErased + 'a>,
        ) -> Option<&'r mut Option<T>>,
    ) -> &mut Demand<'a> {
        if let Some(res @ None) = downcast(&mut self.0) {
            *res = Some(fulfil())
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

mod private {
    use std::marker::{PhantomData, Unsize};
    use std::mem::MaybeUninit;

    use crate::marshal::CTypeBridge;
    use crate::ptr::{
        DowncastSafe, DowncastSafeInterface, IBase, ObjInterface, VTableInterfaceInfo,
        VTableObjectInfo,
    };
    use crate::{interface, DynObj, ObjArc, ObjBox, ObjWeak, ObjectId};

    use super::{Demand, IProvider};

    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, CTypeBridge)]
    pub enum RequestType {
        Object = 0,
        ObjectRef = 1,
        ObjectBox = 2,
        ObjectArc = 3,
        ObjectWeak = 4,
        Interface = 5,
        InterfaceBox = 6,
        InterfaceArc = 7,
        InterfaceWeak = 8,
    }

    interface! {
        #![interface_cfg(
            abi(explicit(abi = "C-unwind")),
        )]

        pub frozen interface IErased: marker IBase {
            fn request_type(&self) -> RequestType;
            fn markers(&self) -> Option<usize>;
            fn object_info(&self) -> Option<&VTableObjectInfo>;
            fn interface_info(&self) -> Option<&VTableInterfaceInfo>;
            fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>];
        }
    }

    pub trait IErasedExt<'a>: IErased + 'a {
        fn downcast_object<T>(&mut self) -> Option<&mut Option<T>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a;

        fn downcast_object_ref<T>(&mut self) -> Option<&mut Option<&'a T>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a;

        fn downcast_object_box<T>(&mut self) -> Option<&mut Option<ObjBox<T>>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a;

        fn downcast_object_arc<T>(&mut self) -> Option<&mut Option<ObjArc<T>>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a;

        fn downcast_object_weak<T>(&mut self) -> Option<&mut Option<ObjWeak<T>>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a;

        fn downcast_interface<T>(&mut self) -> Option<&mut Option<&'a DynObj<T>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a;

        fn downcast_interface_box<T>(&mut self) -> Option<&mut Option<ObjBox<DynObj<T>>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a;

        fn downcast_interface_arc<T>(&mut self) -> Option<&mut Option<ObjArc<DynObj<T>>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a;

        fn downcast_interface_weak<T>(&mut self) -> Option<&mut Option<ObjWeak<DynObj<T>>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a;
    }

    impl<'a, U: IErased + ?Sized + 'a> IErasedExt<'a> for U {
        fn downcast_object<T>(&mut self) -> Option<&mut Option<T>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
        {
            match self.request_type() {
                RequestType::Object => {
                    if self.object_info()?.is::<T>() {
                        let ptr =
                            unsafe { &mut *(self.result_pointer() as *mut _ as *mut Option<T>) };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_object_ref<T>(&mut self) -> Option<&mut Option<&'a T>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
        {
            match self.request_type() {
                RequestType::ObjectRef => {
                    if self.object_info()?.is::<T>() {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _ as *mut Option<&'a T>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_object_box<T>(&mut self) -> Option<&mut Option<ObjBox<T>>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
        {
            match self.request_type() {
                RequestType::ObjectBox => {
                    if self.object_info()?.is::<T>() {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _ as *mut Option<ObjBox<T>>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_object_arc<T>(&mut self) -> Option<&mut Option<ObjArc<T>>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
        {
            match self.request_type() {
                RequestType::ObjectArc => {
                    if self.object_info()?.is::<T>() {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _ as *mut Option<ObjArc<T>>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_object_weak<T>(&mut self) -> Option<&mut Option<ObjWeak<T>>>
        where
            T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
        {
            match self.request_type() {
                RequestType::ObjectWeak => {
                    if self.object_info()?.is::<T>() {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _ as *mut Option<ObjWeak<T>>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_interface<T>(&mut self) -> Option<&mut Option<&'a DynObj<T>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
        {
            match self.request_type() {
                RequestType::Interface => {
                    if self.interface_info()?.is_equal::<T>(self.markers()?) {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _ as *mut Option<&'a DynObj<T>>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_interface_box<T>(&mut self) -> Option<&mut Option<ObjBox<DynObj<T>>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
        {
            match self.request_type() {
                RequestType::InterfaceBox => {
                    if self.interface_info()?.is_equal::<T>(self.markers()?) {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Option<ObjBox<DynObj<T>>>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_interface_arc<T>(&mut self) -> Option<&mut Option<ObjArc<DynObj<T>>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
        {
            match self.request_type() {
                RequestType::InterfaceArc => {
                    if self.interface_info()?.is_equal::<T>(self.markers()?) {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Option<ObjArc<DynObj<T>>>)
                        };
                        Some(ptr)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn downcast_interface_weak<T>(&mut self) -> Option<&mut Option<ObjWeak<DynObj<T>>>>
        where
            T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
        {
            match self.request_type() {
                RequestType::InterfaceWeak => {
                    if self.interface_info()?.is_equal::<T>(self.markers()?) {
                        let ptr = unsafe {
                            &mut *(self.result_pointer() as *mut _
                                as *mut Option<ObjWeak<DynObj<T>>>)
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
    struct ObjRequest<'a> {
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::Object
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
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
    struct ObjRefRequest<'a> {
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjRefRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::ObjectRef
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
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
    struct ObjBoxRequest<'a> {
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjBoxRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::ObjectBox
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
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
    struct ObjArcRequest<'a> {
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjArcRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::ObjectArc
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
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
    struct ObjWeakRequest<'a> {
        info: VTableObjectInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ObjWeakRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::ObjectWeak
        }

        fn markers(&self) -> Option<usize> {
            Some(self.info.markers)
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
    struct InterfaceBoxRequest<'a> {
        markers: usize,
        info: VTableInterfaceInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for InterfaceBoxRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::InterfaceBox
        }

        fn markers(&self) -> Option<usize> {
            Some(self.markers)
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
    struct InterfaceArcRequest<'a> {
        markers: usize,
        info: VTableInterfaceInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for InterfaceArcRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::InterfaceArc
        }

        fn markers(&self) -> Option<usize> {
            Some(self.markers)
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
    struct InterfaceWeakRequest<'a> {
        markers: usize,
        info: VTableInterfaceInfo,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for InterfaceWeakRequest<'a> {
        fn request_type(&self) -> RequestType {
            RequestType::InterfaceWeak
        }

        fn markers(&self) -> Option<usize> {
            Some(self.markers)
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
    /// use fimo_ffi::ObjectId;
    /// use fimo_ffi::provider::{IProvider, Demand, request_object};
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
    /// struct Provider;
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object(|| A(true))
    ///               .provide_object(|| B(32));
    ///     }
    /// }
    ///
    /// let a = request_object::<A>(&Provider).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// let b = request_object::<B>(&Provider).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_object::<C>(&Provider).is_none());
    /// ```
    pub fn request_object<'a, T>(provider: &'a impl IProvider) -> Option<T>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = ObjRequest {
            info: VTableObjectInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an object reference from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// use fimo_ffi::ObjectId;
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
    /// struct Provider(A, B, C);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object_ref(&self.0)
    ///               .provide_object_ref(&self.1);
    ///     }
    /// }
    ///
    /// let p = Provider(A(true), B(32), C(0.0));
    ///
    /// let a = request_object_ref::<A>(&p).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// let b = request_object_ref::<B>(&p).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_object_ref::<C>(&p).is_none());
    /// ```
    pub fn request_object_ref<'a, T>(provider: &'a impl IProvider) -> Option<&'a T>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = ObjRefRequest {
            info: VTableObjectInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an object [`ObjBox`] from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// use fimo_ffi::{ObjBox, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_object_box};
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
    /// struct Provider;
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object_box(|| ObjBox::new(A(true)))
    ///               .provide_object_box(|| ObjBox::new(B(32)));
    ///     }
    /// }
    ///
    /// let a = request_object_box::<A>(&Provider).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// let b = request_object_box::<B>(&Provider).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_object_box::<C>(&Provider).is_none());
    /// ```
    pub fn request_object_box<'a, T>(provider: &'a impl IProvider) -> Option<ObjBox<T>>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = ObjBoxRequest {
            info: VTableObjectInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an object [`ObjArc`] from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// use fimo_ffi::{ObjArc, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_object_arc};
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
    /// struct Provider;
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object_arc(|| ObjArc::new(A(true)))
    ///               .provide_object_arc(|| ObjArc::new(B(32)));
    ///     }
    /// }
    ///
    /// let a = request_object_arc::<A>(&Provider).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// let b = request_object_arc::<B>(&Provider).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_object_arc::<C>(&Provider).is_none());
    /// ```
    pub fn request_object_arc<'a, T>(provider: &'a impl IProvider) -> Option<ObjArc<T>>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = ObjArcRequest {
            info: VTableObjectInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an object [`ObjWeak`] from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// use fimo_ffi::{ObjArc, ObjWeak, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_object_weak};
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
    /// struct Provider(ObjArc<A>, ObjArc<B>, ObjArc<C>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_object_weak(|| ObjArc::downgrade(&self.0))
    ///               .provide_object_weak(|| ObjArc::downgrade(&self.1));
    ///     }
    /// }
    ///
    /// let a = ObjArc::new(A(true));
    /// let b = ObjArc::new(B(32));
    /// let c = ObjArc::new(C(0.0));
    /// let p = Provider(a, b, c);
    ///
    /// let a = request_object_weak::<A>(&p).unwrap();
    /// assert_eq!(a.upgrade().unwrap().0, true);
    ///
    /// let b = request_object_weak::<B>(&p).unwrap();
    /// assert_eq!(b.upgrade().unwrap().0, 32);
    ///
    /// assert!(request_object_weak::<C>(&p).is_none());
    /// ```
    pub fn request_object_weak<'a, T>(provider: &'a impl IProvider) -> Option<ObjWeak<T>>
    where
        T: DowncastSafe + ObjectId + Unsize<dyn IBase + 'a> + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = ObjWeakRequest {
            info: VTableObjectInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an interface reference from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use std::marker::PhantomData;
    /// use fimo_ffi::{interface, ObjectId};
    /// use fimo_ffi::ptr::{coerce_obj, IBase};
    /// use fimo_ffi::provider::{IProvider, Demand, request_interface};
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
    /// struct Provider(A, B, C);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_interface::<dyn IA>(coerce_obj(&self.0))
    ///               .provide_interface::<dyn IB + Send + Sync>(coerce_obj(&self.1))
    ///               .provide_interface::<dyn IC + Unpin>(coerce_obj(&self.2));
    ///     }
    /// }
    ///
    /// let p = Provider(A(true), B(32), C(0.0, PhantomData));
    ///
    /// let a = request_interface::<dyn IA>(&p).unwrap();
    /// assert_eq!(a.val(), true);
    ///
    /// let b = request_interface::<dyn IB + Send + Sync>(&p).unwrap();
    /// assert_eq!(b.val(), 32);
    ///
    /// // Provider provides a `DynObj<dyn IB + Send + Sync>`
    /// assert!(request_interface::<dyn IB>(&p).is_none());
    /// assert!(request_interface::<dyn IB + Send>(&p).is_none());
    /// assert!(request_interface::<dyn IB + Sync>(&p).is_none());
    /// assert!(request_interface::<dyn IB + Unpin>(&p).is_none());
    /// assert!(request_interface::<dyn IB + Send + Sync>(&p).is_some());
    /// assert!(request_interface::<dyn IB + Send + Unpin>(&p).is_none());
    /// assert!(request_interface::<dyn IB + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface::<dyn IB + Send + Sync + Unpin>(&p).is_none());
    ///
    /// let c = request_interface::<dyn IC + Unpin>(&p).unwrap();
    /// assert_eq!(c.val(), 0.0);
    ///
    /// // Provider provides a `DynObj<dyn IC + Unpin>`
    /// assert!(request_interface::<dyn IC>(&p).is_none());
    /// assert!(request_interface::<dyn IC + Send>(&p).is_none());
    /// assert!(request_interface::<dyn IC + Sync>(&p).is_none());
    /// assert!(request_interface::<dyn IC + Unpin>(&p).is_some());
    /// assert!(request_interface::<dyn IC + Send + Sync>(&p).is_none());
    /// assert!(request_interface::<dyn IC + Send + Unpin>(&p).is_none());
    /// assert!(request_interface::<dyn IC + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface::<dyn IC + Send + Sync + Unpin>(&p).is_none());
    /// ```
    pub fn request_interface<'a, T>(provider: &'a impl IProvider) -> Option<&'a DynObj<T>>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = InterfaceRequest {
            markers: <T as ObjInterface>::MARKER_BOUNDS,
            info: VTableInterfaceInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an interface [`ObjBox`] from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use std::marker::PhantomData;
    /// use fimo_ffi::ptr::{coerce_obj, IBase};
    /// use fimo_ffi::{ObjBox, interface, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_interface_box};
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
    /// struct Provider(A, B, C);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_interface_box::<dyn IA>(|| ObjBox::coerce_obj(ObjBox::new(self.0)))
    ///               .provide_interface_box::<dyn IB + Send + Sync>(|| ObjBox::coerce_obj(ObjBox::new(self.1)))
    ///               .provide_interface_box::<dyn IC + Unpin>(|| ObjBox::coerce_obj(ObjBox::new(self.2)));
    ///     }
    /// }
    ///
    /// let p = Provider(A(true), B(32), C(0.0, PhantomData));
    ///
    /// let a = request_interface_box::<dyn IA>(&p).unwrap();
    /// assert_eq!(a.val(), true);
    ///
    /// let b = request_interface_box::<dyn IB + Send + Sync>(&p).unwrap();
    /// assert_eq!(b.val(), 32);
    ///
    /// // Provider provides a `DynObj<dyn IB + Send + Sync>`
    /// assert!(request_interface_box::<dyn IB>(&p).is_none());
    /// assert!(request_interface_box::<dyn IB + Send>(&p).is_none());
    /// assert!(request_interface_box::<dyn IB + Sync>(&p).is_none());
    /// assert!(request_interface_box::<dyn IB + Unpin>(&p).is_none());
    /// assert!(request_interface_box::<dyn IB + Send + Sync>(&p).is_some());
    /// assert!(request_interface_box::<dyn IB + Send + Unpin>(&p).is_none());
    /// assert!(request_interface_box::<dyn IB + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface_box::<dyn IB + Send + Sync + Unpin>(&p).is_none());
    ///
    /// let c = request_interface_box::<dyn IC + Unpin>(&p).unwrap();
    /// assert_eq!(c.val(), 0.0);
    ///
    /// // Provider provides a `DynObj<dyn IC + Unpin>`
    /// assert!(request_interface_box::<dyn IC>(&p).is_none());
    /// assert!(request_interface_box::<dyn IC + Send>(&p).is_none());
    /// assert!(request_interface_box::<dyn IC + Sync>(&p).is_none());
    /// assert!(request_interface_box::<dyn IC + Unpin>(&p).is_some());
    /// assert!(request_interface_box::<dyn IC + Send + Sync>(&p).is_none());
    /// assert!(request_interface_box::<dyn IC + Send + Unpin>(&p).is_none());
    /// assert!(request_interface_box::<dyn IC + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface_box::<dyn IC + Send + Sync + Unpin>(&p).is_none());
    /// ```
    pub fn request_interface_box<'a, T>(provider: &'a impl IProvider) -> Option<ObjBox<DynObj<T>>>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = InterfaceBoxRequest {
            markers: <T as ObjInterface>::MARKER_BOUNDS,
            info: VTableInterfaceInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an interface [`ObjArc`] from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use std::marker::PhantomData;
    /// use fimo_ffi::ptr::{coerce_obj, IBase};
    /// use fimo_ffi::{ObjArc, interface, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_interface_arc};
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
    /// struct Provider(ObjArc<A>, ObjArc<B>, ObjArc<C>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_interface_arc::<dyn IA>(|| ObjArc::coerce_obj(self.0.clone()))
    ///               .provide_interface_arc::<dyn IB + Send + Sync>(|| ObjArc::coerce_obj(self.1.clone()))
    ///               .provide_interface_arc::<dyn IC + Unpin>(|| ObjArc::coerce_obj(self.2.clone()));
    ///     }
    /// }
    ///
    /// let a = ObjArc::new(A(true));
    /// let b = ObjArc::new(B(32));
    /// let c = ObjArc::new(C(0.0, PhantomData));
    /// let p = Provider(a, b, c);
    ///
    /// let a = request_interface_arc::<dyn IA>(&p).unwrap();
    /// assert_eq!(a.val(), true);
    ///
    /// let b = request_interface_arc::<dyn IB + Send + Sync>(&p).unwrap();
    /// assert_eq!(b.val(), 32);
    ///
    /// // Provider provides a `DynObj<dyn IB + Send + Sync>`
    /// assert!(request_interface_arc::<dyn IB>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IB + Send>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IB + Sync>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IB + Unpin>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IB + Send + Sync>(&p).is_some());
    /// assert!(request_interface_arc::<dyn IB + Send + Unpin>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IB + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IB + Send + Sync + Unpin>(&p).is_none());
    ///
    /// let c = request_interface_arc::<dyn IC + Unpin>(&p).unwrap();
    /// assert_eq!(c.val(), 0.0);
    ///
    /// // Provider provides a `DynObj<dyn IC + Unpin>`
    /// assert!(request_interface_arc::<dyn IC>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IC + Send>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IC + Sync>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IC + Unpin>(&p).is_some());
    /// assert!(request_interface_arc::<dyn IC + Send + Sync>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IC + Send + Unpin>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IC + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface_arc::<dyn IC + Send + Sync + Unpin>(&p).is_none());
    /// ```
    pub fn request_interface_arc<'a, T>(provider: &'a impl IProvider) -> Option<ObjArc<DynObj<T>>>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = InterfaceArcRequest {
            markers: <T as ObjInterface>::MARKER_BOUNDS,
            info: VTableInterfaceInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }

    /// Requests an interface [`ObjWeak`] from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use std::marker::PhantomData;
    /// use fimo_ffi::ptr::{coerce_obj, IBase};
    /// use fimo_ffi::{ObjArc, ObjWeak, interface, ObjectId};
    /// use fimo_ffi::provider::{IProvider, Demand, request_interface_weak};
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
    /// struct Provider(ObjArc<A>, ObjArc<B>, ObjArc<C>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_interface_weak::<dyn IA>(|| ObjWeak::coerce_obj(ObjArc::downgrade(&self.0)))
    ///               .provide_interface_weak::<dyn IB + Send + Sync>(|| ObjWeak::coerce_obj(ObjArc::downgrade(&self.1)))
    ///               .provide_interface_weak::<dyn IC + Unpin>(|| ObjWeak::coerce_obj(ObjArc::downgrade(&self.2)));
    ///     }
    /// }
    ///
    /// let a = ObjArc::new(A(true));
    /// let b = ObjArc::new(B(32));
    /// let c = ObjArc::new(C(0.0, PhantomData));
    /// let p = Provider(a, b, c);
    ///
    /// let a = request_interface_weak::<dyn IA>(&p).unwrap();
    /// assert_eq!(a.upgrade().unwrap().val(), true);
    ///
    /// let b = request_interface_weak::<dyn IB + Send + Sync>(&p).unwrap();
    /// assert_eq!(b.upgrade().unwrap().val(), 32);
    ///
    /// // Provider provides a `DynObj<dyn IB + Send + Sync>`
    /// assert!(request_interface_weak::<dyn IB>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IB + Send>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IB + Sync>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IB + Unpin>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IB + Send + Sync>(&p).is_some());
    /// assert!(request_interface_weak::<dyn IB + Send + Unpin>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IB + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IB + Send + Sync + Unpin>(&p).is_none());
    ///
    /// let c = request_interface_weak::<dyn IC + Unpin>(&p).unwrap();
    /// assert_eq!(c.upgrade().unwrap().val(), 0.0);
    ///
    /// // Provider provides a `DynObj<dyn IC + Unpin>`
    /// assert!(request_interface_weak::<dyn IC>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IC + Send>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IC + Sync>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IC + Unpin>(&p).is_some());
    /// assert!(request_interface_weak::<dyn IC + Send + Sync>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IC + Send + Unpin>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IC + Sync + Unpin>(&p).is_none());
    /// assert!(request_interface_weak::<dyn IC + Send + Sync + Unpin>(&p).is_none());
    /// ```
    pub fn request_interface_weak<'a, T>(provider: &'a impl IProvider) -> Option<ObjWeak<DynObj<T>>>
    where
        T: DowncastSafeInterface + Unsize<dyn IBase + 'a> + ?Sized + 'a,
    {
        let mut result = MaybeUninit::new(None);
        let mut request = InterfaceWeakRequest {
            markers: <T as ObjInterface>::MARKER_BOUNDS,
            info: VTableInterfaceInfo::new::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { result.assume_init() }
    }
}
