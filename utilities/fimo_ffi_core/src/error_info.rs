//! Error info type.
use crate::{ConstSpan, NonNullConst, TypeWrapper};
use std::fmt::{Debug, Display, Formatter};
use std::ptr::NonNull;

/// `UTF-8` error string.
pub type ErrorString = ConstSpan<u8>;

/// Opaque structure representing an error info.
#[repr(C)]
#[derive(Debug)]
pub struct ErrorInfoData {
    _dummy: [u8; 0],
}

/// Function pointer to the internal drop function for an `ErrorInfo`.
pub type CleanupFn = TypeWrapper<unsafe extern "C-unwind" fn(Option<NonNull<ErrorInfoData>>)>;

/// Function pointer to the internal clone function for an `ErrorInfo`.
pub type CloneFn = TypeWrapper<
    unsafe extern "C-unwind" fn(
        Option<NonNullConst<ErrorInfoData>>,
    ) -> Option<NonNull<ErrorInfoData>>,
>;

/// Function pointer to the internal as_str function for an `ErrorInfo`.
pub type AsStrFn =
    TypeWrapper<unsafe extern "C-unwind" fn(Option<NonNullConst<ErrorInfoData>>) -> ErrorString>;

/// Error vtable.
#[repr(C)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ErrorInfoVTable {
    /// Cleanup function pointer.
    pub cleanup_fn: CleanupFn,
    /// Clone function pointer.
    pub clone_fn: CloneFn,
    /// AsStr function pointer.
    pub as_str_fn: AsStrFn,
}

/// Error info.
#[repr(C)]
#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ErrorInfo {
    data: Option<NonNull<ErrorInfoData>>,
    vtable: NonNullConst<ErrorInfoVTable>,
}

impl ErrorInfo {
    /// Consumes the `ErrorInfo` and turns it into a pair of raw pointers.
    ///
    /// The result can be turned back into the `ErrorInfo` with [ErrorInfo::from_raw]
    /// to avoid a memory leakage.
    pub fn into_raw(
        self,
    ) -> (
        Option<NonNull<ErrorInfoData>>,
        NonNullConst<ErrorInfoVTable>,
    ) {
        let pointers = (self.data, self.vtable);
        std::mem::forget(self);
        pointers
    }

    /// Creates a new `ErrorInfo` from a data and vtable pointer.
    ///
    /// # Safety
    ///
    /// The data pointer must originate from a call to [ErrorInfo::into_raw]
    /// or from a function in the vtable.
    pub unsafe fn from_raw(
        data: Option<NonNull<ErrorInfoData>>,
        vtable: NonNullConst<ErrorInfoVTable>,
    ) -> Self {
        Self { data, vtable }
    }

    /// Fetches the error string.
    #[inline]
    pub fn as_str(&self) -> ErrorString {
        unsafe { (self.vtable.as_ref().as_str_fn)(self.data.map(From::from)) }
    }
}

unsafe impl Send for ErrorInfo {}

impl Drop for ErrorInfo {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.vtable.as_ref().cleanup_fn)(self.data) }
    }
}

impl Clone for ErrorInfo {
    #[inline]
    fn clone(&self) -> Self {
        let data = unsafe { (self.vtable.as_ref().clone_fn)(self.data.map(From::from)) };
        Self {
            data,
            vtable: self.vtable,
        }
    }
}

impl AsRef<str> for ErrorInfo {
    #[inline]
    fn as_ref(&self) -> &str {
        let data = self.as_str();
        if data.is_empty() {
            ""
        } else {
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(data.as_ptr(), data.len()))
            }
        }
    }
}

impl Display for ErrorInfo {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl Debug for ErrorInfo {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl<T> From<Box<T>> for ErrorInfo
where
    Box<T>: AsErrorInfoVTable,
{
    fn from(v: Box<T>) -> Self {
        Self {
            data: Some(NonNull::from(Box::leak(v)).cast()),
            vtable: NonNullConst::from(&<Box<T>>::VTABLE),
        }
    }
}

/// Types with available error info vtables.
pub trait AsErrorInfoVTable {
    /// VTable for the type.
    const VTABLE: ErrorInfoVTable;

    /// Cleanup function.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from the same vtable.
    unsafe extern "C-unwind" fn cleanup_fn(ptr: Option<NonNull<ErrorInfoData>>);

    /// Clone function.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from the same vtable.
    unsafe extern "C-unwind" fn clone_fn(
        ptr: Option<NonNullConst<ErrorInfoData>>,
    ) -> Option<NonNull<ErrorInfoData>>;

    /// As-str function.
    ///
    /// # Safety
    ///
    /// This function requires that `ptr` originates from the same vtable.
    #[allow(clippy::wrong_self_convention)]
    unsafe extern "C-unwind" fn as_str_fn(ptr: Option<NonNullConst<ErrorInfoData>>) -> ErrorString;
}

impl<T> AsErrorInfoVTable for Box<T>
where
    T: AsRef<str> + Clone + Send,
{
    const VTABLE: ErrorInfoVTable = ErrorInfoVTable {
        cleanup_fn: TypeWrapper(Self::cleanup_fn),
        clone_fn: TypeWrapper(Self::clone_fn),
        as_str_fn: TypeWrapper(Self::as_str_fn),
    };

    unsafe extern "C-unwind" fn cleanup_fn(ptr: Option<NonNull<ErrorInfoData>>) {
        drop(Box::<T>::from_raw(ptr.unwrap().cast().as_ptr()))
    }

    unsafe extern "C-unwind" fn clone_fn(
        ptr: Option<NonNullConst<ErrorInfoData>>,
    ) -> Option<NonNull<ErrorInfoData>> {
        let new: Box<T> = Box::new(ptr.unwrap().cast::<T>().as_ref().clone());
        Some(NonNull::from(Box::leak(new)).cast())
    }

    unsafe extern "C-unwind" fn as_str_fn(ptr: Option<NonNullConst<ErrorInfoData>>) -> ErrorString {
        ErrorString::from(ptr.unwrap().cast::<T>().as_ref().as_ref())
    }
}

#[cfg(test)]
mod tests {
    use crate::ErrorInfo;

    #[test]
    fn box_error() {
        let error_str = Box::new("my error message");
        let error_info = ErrorInfo::from(error_str.clone());
        let error_info_clone = error_info.clone();

        assert_eq!(*error_info.as_ref(), **error_str);
        assert_eq!(*error_info_clone.as_ref(), **error_str);
    }
}
