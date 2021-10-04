//! Error type.
use crate::{ErrorInfo, NonNullConst, Optional, TypeWrapper};
use std::fmt::{Debug, Display, Formatter};
use std::ptr::NonNull;

/// Opaque structure representing an error.
#[repr(C)]
#[derive(Debug)]
pub struct ErrorData {
    _dummy: [u8; 0],
}

/// Function pointer to the internal drop function for an `Error`.
pub type CleanupFn = TypeWrapper<unsafe extern "C-unwind" fn(Option<NonNull<ErrorData>>)>;

/// Function pointer to the internal source function for an `Error` and `ErrorRef`.
pub type SourceFn =
    TypeWrapper<unsafe extern "C-unwind" fn(Option<NonNullConst<ErrorData>>) -> Optional<ErrorRef>>;

/// Function pointer to the internal display function for an `Error` and `ErrorRef`.
pub type DisplayInfoFn =
    TypeWrapper<unsafe extern "C-unwind" fn(Option<NonNullConst<ErrorData>>) -> ErrorInfo>;

/// Function pointer to the internal debug function for an `Error` and `ErrorRef`.
pub type DebugInfoFn =
    TypeWrapper<unsafe extern "C-unwind" fn(Option<NonNullConst<ErrorData>>) -> ErrorInfo>;

/// Error vtable.
#[repr(C)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ErrorVTable {
    /// Cleanup function pointer.
    pub cleanup_fn: CleanupFn,
    /// Source function pointer.
    pub source_fn: SourceFn,
    /// Display function pointer.
    pub display_info_fn: DisplayInfoFn,
    /// Debug function pointer.
    pub debug_info_fn: DebugInfoFn,
}

/// Unowned error value.
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ErrorRef {
    data: Option<NonNullConst<ErrorData>>,
    vtable: NonNullConst<ErrorVTable>,
}

impl ErrorRef {
    /// Consumes the `ErrorRef` and turns it into a pair of raw pointers.
    ///
    /// The result can be turned back into the `ErrorRef` with [ErrorRef::from_raw].
    pub fn into_raw(self) -> (Option<NonNullConst<ErrorData>>, NonNullConst<ErrorVTable>) {
        (self.data, self.vtable)
    }

    /// Creates a new `ErrorRef` from a data and vtable pointer.
    ///
    /// # Safety
    ///
    /// The data pointer must originate from a call to [ErrorRef::into_raw]
    /// or from a function in the vtable.
    pub unsafe fn from_raw(
        data: Option<NonNullConst<ErrorData>>,
        vtable: NonNullConst<ErrorVTable>,
    ) -> Self {
        Self { data, vtable }
    }

    /// Lower-level source, if it exists.
    ///
    /// # Safety
    ///
    /// The resulting error may not outlive self.
    #[inline]
    pub unsafe fn source(&self) -> Optional<ErrorRef> {
        (self.vtable.as_ref().source_fn)(self.data)
    }

    /// Display error info.
    #[inline]
    pub fn display_info(&self) -> ErrorInfo {
        unsafe { (self.vtable.as_ref().display_info_fn)(self.data) }
    }

    /// Display error info.
    #[inline]
    pub fn debug_info(&self) -> ErrorInfo {
        unsafe { (self.vtable.as_ref().debug_info_fn)(self.data) }
    }
}

impl Display for ErrorRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.display_info(), f)
    }
}

impl Debug for ErrorRef {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.debug_info(), f)
    }
}

/// Owned error value.
#[repr(C)]
#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Error {
    internal: ErrorRef,
}

unsafe impl Send for Error {}

impl Error {
    /// Consumes the `ErrorRef` and turns it into a pair of raw pointers.
    ///
    /// The result can be turned back into the `ErrorRef` with [ErrorRef::from_raw]
    /// to avoid a memory leakage.
    pub fn into_raw(self) -> (Option<NonNullConst<ErrorData>>, NonNullConst<ErrorVTable>) {
        self.internal.into_raw()
    }

    /// Creates a new `ErrorRef` from a data and vtable pointer.
    ///
    /// # Safety
    ///
    /// The data pointer must originate from a call to [ErrorRef::into_raw]
    /// or from a function in the vtable.
    pub unsafe fn from_raw(
        data: Option<NonNullConst<ErrorData>>,
        vtable: NonNullConst<ErrorVTable>,
    ) -> Self {
        Self {
            internal: ErrorRef::from_raw(data, vtable),
        }
    }

    /// Lower-level source, if it exists.
    ///
    /// # Safety
    ///
    /// The resulting error may not outlive self.
    #[inline]
    pub unsafe fn source(&self) -> Optional<ErrorRef> {
        self.internal.source()
    }

    /// Display error info.
    #[inline]
    pub fn display_info(&self) -> ErrorInfo {
        self.internal.display_info()
    }

    /// Display error info.
    #[inline]
    pub fn debug_info(&self) -> ErrorInfo {
        self.internal.debug_info()
    }
}

impl Drop for Error {
    fn drop(&mut self) {
        unsafe {
            (self.internal.vtable.as_ref().cleanup_fn)(self.internal.data.map(|v| v.into_mut()))
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.internal, f)
    }
}

impl Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.internal, f)
    }
}

impl<T> From<T> for Error
where
    T: std::error::Error + Send,
{
    fn from(error: T) -> Self {
        Self::from(Box::new(box_error::OwnedError::new(error)))
    }
}

impl From<Error> for Box<dyn std::error::Error> {
    fn from(error: Error) -> Self {
        Box::new(rust_error::RustError::new(error))
    }
}

/// Types with available error vtables.
pub trait AsErrorVTable {
    /// VTable for the type.
    const VTABLE: ErrorVTable;

    /// Cleanup function.
    ///
    /// # Safety
    ///
    /// Passes the ffi boundary.
    unsafe extern "C-unwind" fn cleanup_fn(data: Option<NonNull<ErrorData>>);

    /// Source function.
    ///
    /// # Safety
    ///
    /// Passes the ffi boundary.
    unsafe extern "C-unwind" fn source_fn(
        data: Option<NonNullConst<ErrorData>>,
    ) -> Optional<ErrorRef>;

    /// Display function.
    ///
    /// # Safety
    ///
    /// Passes the ffi boundary.
    unsafe extern "C-unwind" fn display_info_fn(data: Option<NonNullConst<ErrorData>>)
        -> ErrorInfo;

    /// Debug function.
    ///
    /// # Safety
    ///
    /// Passes the ffi boundary.
    unsafe extern "C-unwind" fn debug_info_fn(data: Option<NonNullConst<ErrorData>>) -> ErrorInfo;
}

pub(crate) mod rust_error {
    use std::error::Error;
    use std::fmt::{Debug, Display, Formatter};

    pub struct RustError {
        error: crate::Error,
        source: Option<Box<RustErrorRef>>,
    }

    impl RustError {
        pub fn new(error: crate::Error) -> Self {
            let source = unsafe { error.source() }
                .into_rust()
                .map(|source| Box::new(RustErrorRef::new(source)));

            Self { error, source }
        }
    }

    impl Debug for RustError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(&self.error, f)
        }
    }

    impl Display for RustError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Display::fmt(&self.error, f)
        }
    }

    impl Error for RustError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match &self.source {
                None => None,
                Some(source) => Some(source),
            }
        }
    }

    pub struct RustErrorRef {
        error: crate::error::ErrorRef,
        source: Option<Box<RustErrorRef>>,
    }

    impl RustErrorRef {
        pub fn new(error: crate::error::ErrorRef) -> Self {
            let source = unsafe { error.source() }
                .into_rust()
                .map(|source| Box::new(RustErrorRef::new(source)));

            Self { error, source }
        }
    }

    impl Debug for RustErrorRef {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(&self.error, f)
        }
    }

    impl Display for Box<RustErrorRef> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Display::fmt(&self.error, f)
        }
    }

    impl Error for Box<RustErrorRef> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match &self.source {
                None => None,
                Some(source) => Some(source),
            }
        }
    }
}

pub(crate) mod box_error {
    use crate::{
        error::{AsErrorVTable, Error as Err, ErrorData, ErrorRef, ErrorVTable},
        ErrorInfo, NonNullConst, Optional, TypeWrapper,
    };
    use std::error::Error;
    use std::ptr::NonNull;

    #[derive(Debug)]
    pub struct InternalError {
        error: NonNullConst<dyn Error + 'static>,
        source: Option<Box<InternalError>>,
    }

    impl InternalError {
        #[inline]
        pub fn new(error: &(dyn Error + 'static)) -> Self {
            let source = error.source();
            Self {
                error: NonNullConst::from(error),
                source: source.map(|e| Box::new(Self::new(e))),
            }
        }

        #[inline]
        pub fn source(&self) -> &Option<Box<InternalError>> {
            &self.source
        }

        #[inline]
        #[allow(clippy::box_collection)]
        pub fn display_info(&self) -> Box<String> {
            Box::new(format!("{}", unsafe { self.error.as_ref() }))
        }

        #[inline]
        #[allow(clippy::box_collection)]
        pub fn debug_info(&self) -> Box<String> {
            Box::new(format!("{:?}", unsafe { self.error.as_ref() }))
        }
    }

    impl From<&InternalError> for ErrorRef {
        fn from(error: &InternalError) -> Self {
            Self {
                data: Some(NonNullConst::from(error).cast()),
                vtable: NonNullConst::from(&<&InternalError>::VTABLE),
            }
        }
    }

    impl AsErrorVTable for &InternalError {
        const VTABLE: ErrorVTable = ErrorVTable {
            cleanup_fn: TypeWrapper(Self::cleanup_fn),
            source_fn: TypeWrapper(Self::source_fn),
            display_info_fn: TypeWrapper(Self::display_info_fn),
            debug_info_fn: TypeWrapper(Self::debug_info_fn),
        };

        unsafe extern "C-unwind" fn cleanup_fn(_data: Option<NonNull<ErrorData>>) {}

        unsafe extern "C-unwind" fn source_fn(
            data: Option<NonNullConst<ErrorData>>,
        ) -> Optional<ErrorRef> {
            data.unwrap()
                .cast::<InternalError>()
                .as_ref()
                .source()
                .as_ref()
                .map_or(Optional::None, |error| {
                    Optional::Some(ErrorRef::from(error.as_ref()))
                })
        }

        unsafe extern "C-unwind" fn display_info_fn(
            data: Option<NonNullConst<ErrorData>>,
        ) -> ErrorInfo {
            ErrorInfo::from(
                data.unwrap()
                    .cast::<InternalError>()
                    .as_ref()
                    .display_info(),
            )
        }

        unsafe extern "C-unwind" fn debug_info_fn(
            data: Option<NonNullConst<ErrorData>>,
        ) -> ErrorInfo {
            ErrorInfo::from(data.unwrap().cast::<InternalError>().as_ref().debug_info())
        }
    }

    #[derive(Debug)]
    pub struct OwnedError<T: Error + Send> {
        error: Box<T>,
        source: Option<Box<InternalError>>,
    }

    unsafe impl<T: Error + Send> Send for OwnedError<T> {}

    impl<T: Error + Send> OwnedError<T> {
        #[inline]
        pub fn new(error: T) -> Self {
            let mut err = Self {
                error: Box::new(error),
                source: None,
            };

            err.source = err.error.source().map(|e| Box::new(InternalError::new(e)));
            err
        }

        #[inline]
        pub fn source(&self) -> &Option<Box<InternalError>> {
            &self.source
        }

        #[inline]
        #[allow(clippy::box_collection)]
        pub fn display_info(&self) -> Box<String> {
            Box::new(format!("{}", &self.error))
        }

        #[inline]
        #[allow(clippy::box_collection)]
        pub fn debug_info(&self) -> Box<String> {
            Box::new(format!("{:?}", &self.error))
        }
    }

    impl<T: Error + Send> From<Box<OwnedError<T>>> for Err {
        fn from(error: Box<OwnedError<T>>) -> Self {
            Self {
                internal: ErrorRef {
                    data: Some(NonNullConst::from(Box::leak(error)).cast()),
                    vtable: NonNullConst::from(&<Box<OwnedError<T>>>::VTABLE),
                },
            }
        }
    }

    impl<T: Error + Send> AsErrorVTable for Box<OwnedError<T>> {
        const VTABLE: ErrorVTable = ErrorVTable {
            cleanup_fn: TypeWrapper(Self::cleanup_fn),
            source_fn: TypeWrapper(Self::source_fn),
            display_info_fn: TypeWrapper(Self::display_info_fn),
            debug_info_fn: TypeWrapper(Self::debug_info_fn),
        };

        unsafe extern "C-unwind" fn cleanup_fn(data: Option<NonNull<ErrorData>>) {
            drop(Box::<OwnedError<T>>::from_raw(
                data.unwrap().cast().as_ptr(),
            ))
        }

        unsafe extern "C-unwind" fn source_fn(
            data: Option<NonNullConst<ErrorData>>,
        ) -> Optional<ErrorRef> {
            data.unwrap()
                .cast::<OwnedError<T>>()
                .as_ref()
                .source()
                .as_ref()
                .map_or(Optional::None, |error| {
                    Optional::Some(ErrorRef::from(error.as_ref()))
                })
        }

        unsafe extern "C-unwind" fn display_info_fn(
            data: Option<NonNullConst<ErrorData>>,
        ) -> ErrorInfo {
            ErrorInfo::from(
                data.unwrap()
                    .cast::<OwnedError<T>>()
                    .as_ref()
                    .display_info(),
            )
        }

        unsafe extern "C-unwind" fn debug_info_fn(
            data: Option<NonNullConst<ErrorData>>,
        ) -> ErrorInfo {
            ErrorInfo::from(data.unwrap().cast::<OwnedError<T>>().as_ref().debug_info())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error as Err, Optional};
    use std::error::Error;
    use std::fmt::{Debug, Display, Formatter};

    #[derive(Copy, Clone)]
    struct MyError {
        internal: MyInternalError,
    }

    impl Debug for MyError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("my error debug!")
        }
    }

    impl Display for MyError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("my error display!")
        }
    }

    impl Error for MyError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.internal)
        }
    }

    #[derive(Copy, Clone)]
    struct MyInternalError {
        error: &'static str,
    }

    impl Debug for MyInternalError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Display::fmt(self.error, f)
        }
    }

    impl Display for MyInternalError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Display::fmt(self.error, f)
        }
    }

    impl Error for MyInternalError {}

    #[test]
    fn custom_error() {
        let my_error = MyError {
            internal: MyInternalError { error: "my error!" },
        };

        let error = Err::from(my_error);

        let error_dis_info = error.display_info();
        let error_dbg_info = error.debug_info();

        let error_dis = error_dis_info.as_ref();
        let error_dbg = error_dbg_info.as_ref();

        assert_eq!(format!("{}", &my_error), error_dis);
        assert_eq!(format!("{:?}", &my_error), error_dbg);

        let source = unsafe { error.source() }.unwrap();

        let source_dis_info = source.display_info();
        let source_dbg_info = source.debug_info();

        let source_dis = source_dis_info.as_ref();
        let source_dbg = source_dbg_info.as_ref();

        assert_eq!(format!("{}", &my_error.internal), source_dis);
        assert_eq!(format!("{:?}", &my_error.internal), source_dbg);

        assert_eq!(unsafe { source.source() }, Optional::None)
    }
}
