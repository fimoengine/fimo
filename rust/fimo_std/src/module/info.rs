use crate::{
    context::ContextView,
    error::AnyError,
    ffi::{ConstCStr, OpaqueHandle, VTablePtr, View, Viewable},
    version::Version,
};
use std::{
    ffi::CStr,
    fmt::{Debug, Formatter},
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    pin::Pin,
};

/// Virtual function table of an [`Info`].
///
/// Adding fields to the vtable is not a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct InfoVTable {
    acquire: unsafe extern "C" fn(info: Pin<&InfoView<'_>>),
    release: unsafe extern "C" fn(info: Pin<&InfoView<'_>>),
    mark_unloadable: unsafe extern "C" fn(info: Pin<&InfoView<'_>>),
    is_loaded: unsafe extern "C" fn(info: Pin<&InfoView<'_>>) -> bool,
    try_ref_instance_strong: unsafe extern "C" fn(info: Pin<&InfoView<'_>>) -> bool,
    unref_instance_strong: unsafe extern "C" fn(info: Pin<&InfoView<'_>>) -> bool,
}

/// Borrowed info of a module instance.
#[repr(C)]
pub struct InfoView<'a> {
    pub next: Option<OpaqueHandle<dyn Send + Sync + 'a>>,
    pub name: ConstCStr,
    pub description: Option<ConstCStr>,
    pub author: Option<ConstCStr>,
    pub license: Option<ConstCStr>,
    pub module_path: Option<ConstCStr>,
    pub vtable: VTablePtr<InfoVTable>,
    // Using PhantomPinned directly makes it not FFI-Safe.
    pub _phantom: PhantomData<PhantomPinned>,
}

impl InfoView<'_> {
    /// Returns the name of the instance.
    pub const fn name(self: Pin<&Self>) -> &CStr {
        unsafe { Pin::into_inner_unchecked(self).name.as_ref() }
    }

    /// Returns the optional description of the instance.
    pub const fn description(self: Pin<&Self>) -> Option<&CStr> {
        unsafe {
            match Pin::into_inner_unchecked(self).description {
                None => None,
                Some(x) => Some(x.as_ref()),
            }
        }
    }

    /// Returns the optional author of the instance.
    pub const fn author(self: Pin<&Self>) -> Option<&CStr> {
        unsafe {
            match Pin::into_inner_unchecked(self).author {
                None => None,
                Some(x) => Some(x.as_ref()),
            }
        }
    }

    /// Returns the optional license of the instance.
    pub const fn license(self: Pin<&Self>) -> Option<&CStr> {
        unsafe {
            match Pin::into_inner_unchecked(self).license {
                None => None,
                Some(x) => Some(x.as_ref()),
            }
        }
    }

    /// Returns the optional path to the binary containing the instance.
    pub const fn module_path(self: Pin<&Self>) -> Option<&str> {
        unsafe {
            match Pin::into_inner_unchecked(self).module_path {
                None => None,
                Some(x) => Some(std::str::from_utf8_unchecked(x.as_ref().to_bytes())),
            }
        }
    }

    /// Promotes the `InfoView` to a [`Info`], by increasing the reference count.
    pub fn to_info(self: Pin<&Self>) -> Info {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let f = this.vtable.acquire;
            f(self);
            Info(std::mem::transmute::<
                Pin<&Self>,
                Pin<&'static InfoView<'static>>,
            >(self))
        }
    }

    /// Signals that the owning instance may be unloaded.
    ///
    /// The instance will be unloaded once it is no longer actively used by another instance.
    pub fn mark_unloadable(self: Pin<&Self>) {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let f = this.vtable.mark_unloadable;
            f(self);
        }
    }

    /// Returns whether the owning module instance is still loaded.
    pub fn is_loaded(self: Pin<&Self>) -> bool {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let f = this.vtable.is_loaded;
            f(self)
        }
    }

    /// Tries to increase the strong reference count of the module instance.
    ///
    /// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
    /// between modules, without registering the dependency with the subsystem.
    pub fn try_ref_instance_strong(self: Pin<&Self>) -> bool {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let f = this.vtable.try_ref_instance_strong;
            f(self)
        }
    }

    /// Decreases the strong reference count of the module instance.
    ///
    /// # Safety
    ///
    /// May only be called after a call to [`InfoView::try_ref_instance_strong`].
    pub unsafe fn unref_instance_strong(self: Pin<&Self>) {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let f = this.vtable.unref_instance_strong;
            f(self);
        }
    }
}

impl View for Pin<&InfoView<'_>> {}

impl Debug for InfoView<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("InfoView")
                .field("next", &self.next)
                .field("name", &Pin::new_unchecked(self).name())
                .field("description", &Pin::new_unchecked(self).description())
                .field("author", &Pin::new_unchecked(self).author())
                .field("license", &Pin::new_unchecked(self).license())
                .field("module_path", &Pin::new_unchecked(self).module_path())
                .field("vtable", &self.vtable)
                .finish()
        }
    }
}

/// Owned info of a module instance.
#[derive(Debug)]
#[repr(transparent)]
pub struct Info(Pin<&'static InfoView<'static>>);

impl Info {
    /// Searches for a module by its name.
    ///
    /// Queries a module by its unique name. The returned `Info` instance will have its reference
    /// count increased.
    pub fn find_by_name(
        ctx: impl Viewable<ContextView<'_>>,
        name: &CStr,
    ) -> Result<Self, AnyError> {
        let ctx = ctx.view();
        let mut out = MaybeUninit::uninit();
        let f = ctx.vtable.module_v0.find_instance_by_name;
        unsafe {
            f(ctx.handle, ConstCStr::new(name), &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Searches for a module by a symbol it exports.
    ///
    /// Queries the module that exported the specified symbol. The returned `Info` instance will
    /// have its reference count increased.
    pub fn find_by_symbol_raw(
        ctx: impl Viewable<ContextView<'_>>,
        name: &CStr,
        namespace: &CStr,
        version: Version,
    ) -> Result<Self, AnyError> {
        let ctx = ctx.view();
        let mut out = MaybeUninit::uninit();
        let f = ctx.vtable.module_v0.find_instance_by_symbol;
        unsafe {
            let name = ConstCStr::new(name);
            let namespace = ConstCStr::new(namespace);
            f(ctx.handle, name, namespace, version, &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }
}

impl Clone for Info {
    fn clone(&self) -> Self {
        self.view().to_info()
    }
}

impl<'a> Viewable<Pin<&'a InfoView<'a>>> for &'a Info {
    fn view(self) -> Pin<&'a InfoView<'a>> {
        self.0
    }
}

impl Drop for Info {
    fn drop(&mut self) {
        let view = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(view);
            let f = inner.vtable.release;
            f(view);
        }
    }
}
