//! Implementation of path utilities.
use crate::{
    marshal::CTypeBridge,
    span::{ConstSpanPtr, MutSpanPtr},
    ConstSpan, MutSpan,
};
use std::path::Path;

// FIXME: Relies on an implementation detail of the standard library.
unsafe impl<'a> const CTypeBridge for &'a Path {
    type Type = ConstSpan<'a, u8>;

    fn marshal(self) -> Self::Type {
        unsafe {
            let bytes: &[u8] = std::mem::transmute(self);
            bytes.into()
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let bytes: &[u8] = x.into();
        std::mem::transmute(bytes)
    }
}

// FIXME: Relies on an implementation detail of the standard library.
unsafe impl<'a> const CTypeBridge for &'a mut Path {
    type Type = MutSpan<'a, u8>;

    fn marshal(self) -> Self::Type {
        unsafe {
            let bytes: &mut [u8] = std::mem::transmute(self);
            bytes.into()
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let bytes: &mut [u8] = x.into();
        std::mem::transmute(bytes)
    }
}

// FIXME: Relies on an implementation detail of the standard library.
unsafe impl const CTypeBridge for *const Path {
    type Type = ConstSpanPtr<u8>;

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn marshal(self) -> Self::Type {
        unsafe {
            let bytes: *const [u8] = std::mem::transmute(self);
            bytes.into()
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let bytes: *const [u8] = x.into();
        std::mem::transmute(bytes)
    }
}

// FIXME: Relies on an implementation detail of the standard library.
unsafe impl const CTypeBridge for *mut Path {
    type Type = MutSpanPtr<u8>;

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn marshal(self) -> Self::Type {
        unsafe {
            let bytes: *mut [u8] = std::mem::transmute(self);
            bytes.into()
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let bytes: *mut [u8] = x.into();
        std::mem::transmute(bytes)
    }
}
