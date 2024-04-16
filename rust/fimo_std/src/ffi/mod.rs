//! FFI helpers.

/// Used to transfer ownership to and from a ffi interface.
///
/// The ownership of a type is transferred by calling [`Self::into_ffi`] and
/// is transferred back by calling [`Self::from_ffi`].
pub trait FFITransferable<FfiType: Sized> {
    /// Transfers the ownership from a Rust type to a ffi type.
    fn into_ffi(self) -> FfiType;

    /// Assumes ownership of a ffi type.
    ///
    /// # Safety
    ///
    /// The caller must ensure to have the ownership of the ffi type.
    unsafe fn from_ffi(ffi: FfiType) -> Self;
}

/// Used to share ownership with and from a ffi interface.
///
/// The ownership of a type is shared by calling [`Self::share_to_ffi`] and
/// is borrowed by calling [`Self::borrow_from_ffi`].
pub trait FFISharable<FfiType: Sized> {
    type BorrowedView<'a>: 'a;

    /// Shares the value of a Rust type with a ffi type.
    fn share_to_ffi(&self) -> FfiType;

    /// Borrows the ownership of a ffi type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all invariants of the type are conserved.
    unsafe fn borrow_from_ffi<'a>(ffi: FfiType) -> Self::BorrowedView<'a>;
}
