//! Implementation of the [`IProvider`] interface.

use crate::{interface, marshal::CTypeBridge, ptr::IBase, DynObj, Optional};
use std::fmt::Debug;

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

pub use private::{request_ref, request_value};

/// Helper object for providing data by type.
#[repr(transparent)]
pub struct Demand<'a>(DynObj<dyn private::IErased + 'a>);

impl<'a> Demand<'a> {
    /// Provide a value.
    pub fn provide_value<T>(&mut self, fulfil: impl FnOnce() -> T) -> &mut Demand<'a>
    where
        T: CTypeBridge + 'static,
    {
        self.provide_val_impl(fulfil, private::IErasedExt::downcast_value::<T>)
    }

    /// Provide a reference.
    pub fn provide_ref<T>(&mut self, value: &'a T) -> &mut Demand<'a>
    where
        &'a T: CTypeBridge,
        T: ?Sized + 'static,
    {
        self.provide_val_impl(|| value, private::IErasedExt::downcast_ref::<T>)
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

mod private {
    use std::marker::PhantomData;
    use std::mem::MaybeUninit;

    use crate::marshal::CTypeBridge;
    use crate::ptr::IBase;
    use crate::type_id::StableTypeId;
    use crate::{interface, Object, Optional};

    use super::{Demand, IProvider};

    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, CTypeBridge)]
    pub enum RequestType {
        Value = 0,
        Ref = 1,
    }

    interface! {
        #![interface_cfg(
            abi(explicit(abi = "C-unwind")),
        )]

        pub frozen interface IErased: marker IBase {
            fn type_id(&self) -> StableTypeId;
            fn request_type(&self) -> RequestType;
            fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>];
        }
    }

    pub trait IErasedExt<'a>: IErased + 'a {
        fn downcast_value<T>(&mut self) -> Option<&mut Optional<<T as CTypeBridge>::Type>>
        where
            T: CTypeBridge + 'static;

        fn downcast_ref<T>(&mut self) -> Option<&mut Optional<<&'a T as CTypeBridge>::Type>>
        where
            &'a T: CTypeBridge,
            T: ?Sized + 'static;
    }

    impl<'a, U: IErased + ?Sized + 'a> IErasedExt<'a> for U {
        fn downcast_value<T>(&mut self) -> Option<&mut Optional<<T as CTypeBridge>::Type>>
        where
            T: CTypeBridge + 'static,
        {
            match self.request_type() {
                RequestType::Value => {
                    if self.type_id() == StableTypeId::of::<T>() {
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

        fn downcast_ref<T>(&mut self) -> Option<&mut Optional<<&'a T as CTypeBridge>::Type>>
        where
            &'a T: CTypeBridge,
            T: ?Sized + 'static,
        {
            match self.request_type() {
                RequestType::Ref => {
                    if self.type_id() == StableTypeId::of::<T>() {
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

    #[derive(Object)]
    #[interfaces(IErased)]
    struct ValueRequest<'a> {
        id: StableTypeId,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for ValueRequest<'a> {
        fn type_id(&self) -> StableTypeId {
            self.id
        }

        fn request_type(&self) -> RequestType {
            RequestType::Value
        }

        fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>] {
            self.result_ptr
        }
    }

    #[derive(Object)]
    #[interfaces(IErased)]
    struct RefRequest<'a> {
        id: StableTypeId,
        result_ptr: *mut [MaybeUninit<u8>],
        _phantom: PhantomData<&'a mut ()>,
    }

    impl<'a> IErased for RefRequest<'a> {
        fn type_id(&self) -> StableTypeId {
            self.id
        }

        fn request_type(&self) -> RequestType {
            RequestType::Ref
        }

        fn result_pointer(&mut self) -> *mut [MaybeUninit<u8>] {
            self.result_ptr
        }
    }

    /// Requests a value from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use fimo_ffi::ptr::IBase;
    /// use fimo_ffi::marshal::CTypeBridge;
    /// use fimo_ffi::provider::{IProvider, Demand, request_value};
    /// use fimo_ffi::{ObjBox, ObjArc, Object, interface, DynObj};
    ///
    /// #[derive(CTypeBridge, Object)]
    /// struct A(bool);
    ///
    /// #[derive(CTypeBridge, Object)]
    /// struct B(usize);
    ///
    /// #[derive(CTypeBridge, Object)]
    /// #[interfaces(IC)]
    /// struct C(f32);
    ///
    /// impl IC for C {
    ///     fn val(&self) -> f32 {
    ///         self.0
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
    /// struct Provider(ObjArc<DynObj<dyn IC + Send>>);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_value(|| A(true))
    ///               .provide_value(|| ObjBox::new(B(32)))
    ///               .provide_value(|| self.0.clone());
    ///     }
    /// }
    ///
    /// let c = ObjArc::coerce_obj(ObjArc::new(C(1.7)));
    /// let p = Provider(c);
    ///
    /// let a = request_value::<A>(&p).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// assert!(request_value::<ObjBox<A>>(&p).is_none());
    /// assert!(request_value::<ObjArc<A>>(&p).is_none());
    ///
    /// let b = request_value::<ObjBox<B>>(&p).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_value::<C>(&p).is_none());
    ///
    /// let c = request_value::<ObjArc<DynObj<dyn IC + Send>>>(&p).unwrap();
    /// assert_eq!(c.val(), 1.7);
    ///
    /// // Provider provides a `ObjArc<DynObj<dyn IC + Send>>`
    /// assert!(request_value::<ObjArc<DynObj<dyn IC>>>(&p).is_none());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Send>>>(&p).is_some());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Sync>>>(&p).is_none());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Unpin>>>(&p).is_none());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Send + Sync>>>(&p).is_none());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Send + Unpin>>>(&p).is_none());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Sync + Unpin>>>(&p).is_none());
    /// assert!(request_value::<ObjArc<DynObj<dyn IC + Send + Sync + Unpin>>>(&p).is_none());
    /// ```
    pub fn request_value<'a, T>(provider: &'a impl IProvider) -> Option<T>
    where
        T: CTypeBridge + 'static,
    {
        let mut result = MaybeUninit::new(Optional::None);
        let mut request = ValueRequest {
            id: StableTypeId::of::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { Option::demarshal(result.assume_init()) }
    }

    /// Requests a reference from the [`IProvider`].
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(const_trait_impl)]
    ///
    /// use fimo_ffi::ptr::{IBase, coerce_obj};
    /// use fimo_ffi::provider::{IProvider, Demand, request_ref};
    /// use fimo_ffi::{ObjBox, ObjArc, Object, interface, DynObj};
    ///
    /// #[derive(Object)]
    /// struct A(bool);
    ///
    /// #[derive(Object)]
    /// struct B(usize);
    ///
    /// #[derive(Object)]
    /// #[interfaces(IC)]
    /// struct C(f32);
    ///
    /// impl IC for C {
    ///     fn val(&self) -> f32 {
    ///         self.0
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
    /// struct Provider(A, ObjBox<B>, C);
    ///
    /// impl IProvider for Provider {
    ///     fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
    ///         demand.provide_ref(&self.0)
    ///               .provide_ref(&self.1)
    ///               .provide_ref::<DynObj<dyn IC + Send>>(coerce_obj(&self.2));
    ///     }
    /// }
    ///
    /// let a = A(true);
    /// let b = ObjBox::new(B(32));
    /// let c = C(1.7);
    /// let p = Provider(a, b, c);
    ///
    /// let a = request_ref::<A>(&p).unwrap();
    /// assert_eq!(a.0, true);
    ///
    /// assert!(request_ref::<ObjBox<A>>(&p).is_none());
    /// assert!(request_ref::<ObjArc<A>>(&p).is_none());
    ///
    /// let b = request_ref::<ObjBox<B>>(&p).unwrap();
    /// assert_eq!(b.0, 32);
    ///
    /// assert!(request_ref::<C>(&p).is_none());
    ///
    /// let c = request_ref::<DynObj<dyn IC + Send>>(&p).unwrap();
    /// assert_eq!(c.val(), 1.7);
    ///
    /// // Provider provides a `&DynObj<dyn IC + Send>`
    /// assert!(request_ref::<DynObj<dyn IC>>(&p).is_none());
    /// assert!(request_ref::<DynObj<dyn IC + Send>>(&p).is_some());
    /// assert!(request_ref::<DynObj<dyn IC + Sync>>(&p).is_none());
    /// assert!(request_ref::<DynObj<dyn IC + Unpin>>(&p).is_none());
    /// assert!(request_ref::<DynObj<dyn IC + Send + Sync>>(&p).is_none());
    /// assert!(request_ref::<DynObj<dyn IC + Send + Unpin>>(&p).is_none());
    /// assert!(request_ref::<DynObj<dyn IC + Sync + Unpin>>(&p).is_none());
    /// assert!(request_ref::<DynObj<dyn IC + Send + Sync + Unpin>>(&p).is_none());
    /// ```
    pub fn request_ref<'a, T>(provider: &'a impl IProvider) -> Option<&'a T>
    where
        &'a T: CTypeBridge,
        T: ?Sized + 'static,
    {
        let mut result: MaybeUninit<Optional<<&'a T as CTypeBridge>::Type>> =
            MaybeUninit::new(Optional::None);
        let mut request = RefRequest {
            id: StableTypeId::of::<T>(),
            result_ptr: result.as_bytes_mut(),
            _phantom: PhantomData,
        };
        let demand = crate::ptr::coerce_obj_mut::<_, dyn IErased + 'a>(&mut request);
        let demand = unsafe { &mut *(demand as *mut _ as *mut Demand<'a>) };
        provider.provide(demand);

        unsafe { Option::demarshal(result.assume_init()) }
    }
}
