use crate::KeyGenerator;
use emf_core_base_rs::ffi::collections::NonNullConst;
use emf_core_base_rs::ffi::library::library_loader::{LibraryLoaderInterface, NativeLibraryHandle};
use emf_core_base_rs::ffi::library::InternalHandle as InternalLibraryHandle;
use emf_core_base_rs::library::library_loader::{
    LibraryLoader, NativeLoader as NativeLibraryLoader,
};
use emf_core_base_rs::ownership::Owned;
use emf_core_base_rs::Error;
#[cfg(unix)]
use libloading::os::unix::*;
#[cfg(windows)]
use libloading::os::windows::*;
use parking_lot::Mutex;
use std::any::Any;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
#[cfg(windows)]
use std::os::windows::raw::HANDLE;
use std::path::PathBuf;
use std::ptr::NonNull;

#[derive(Debug)]
enum NativeLoaderError {
    InvalidLibraryHandle {
        handle: InternalLibraryHandle,
    },
    LibraryLoadError {
        library: PathBuf,
        error: libloading::Error,
    },
    LibraryUnloadError {
        handle: InternalLibraryHandle,
        error: libloading::Error,
    },
    LibrarySymbolError {
        handle: InternalLibraryHandle,
        error: libloading::Error,
    },
    UnknownError {
        error: Box<dyn Any + Send + 'static>,
    },
}

impl Display for NativeLoaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeLoaderError::InvalidLibraryHandle { handle } => {
                write!(f, "Invalid library handle! handle: {}", handle)
            }
            NativeLoaderError::LibraryLoadError { library, error: _ } => {
                write!(
                    f,
                    "Error while trying to load a library! library: {}",
                    library.display()
                )
            }
            NativeLoaderError::LibraryUnloadError { handle, error: _ } => {
                write!(
                    f,
                    "Error while trying to unload a library! handle: {}",
                    handle
                )
            }
            NativeLoaderError::LibrarySymbolError { handle, error: _ } => {
                write!(
                    f,
                    "Error while trying to load a symbol from a library! handle: {}",
                    handle
                )
            }
            NativeLoaderError::UnknownError { error: _ } => {
                write!(f, "Unknown error occurred!")
            }
        }
    }
}

impl std::error::Error for NativeLoaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NativeLoaderError::InvalidLibraryHandle { handle: _ } => None,
            NativeLoaderError::LibraryLoadError { library: _, error } => Some(error),
            NativeLoaderError::LibraryUnloadError { handle: _, error } => Some(error),
            NativeLoaderError::LibrarySymbolError { handle: _, error } => Some(error),
            NativeLoaderError::UnknownError { error: _ } => None,
        }
    }
}

/// Internal loader.
#[derive(Debug)]
pub struct NativeLoaderInternal {
    libraries: HashMap<InternalLibraryHandle, Library>,
    library_gen:
        KeyGenerator<InternalLibraryHandle, fn(&InternalLibraryHandle) -> InternalLibraryHandle>,
}

impl Default for NativeLoaderInternal {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeLoaderInternal {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        let first_library = InternalLibraryHandle { id: 0 };
        let library_generator =
            |old: &InternalLibraryHandle| InternalLibraryHandle { id: old.id + 1 };

        Self {
            libraries: Default::default(),
            library_gen: KeyGenerator::new(first_library, library_generator),
        }
    }

    /// Loads a library.
    #[inline]
    #[cfg(unix)]
    pub fn load_library(
        &mut self,
        path: impl AsRef<OsStr>,
        flags: i32,
    ) -> Result<InternalLibraryHandle, Error<Owned>> {
        let lib = match unsafe { Library::open(Some(&path), flags) } {
            Ok(lib) => lib,
            Err(e) => {
                return Err(Error::from(NativeLoaderError::LibraryLoadError {
                    library: PathBuf::from(path.as_ref().to_os_string()),
                    error: e,
                }))
            }
        };

        let handle = self.library_gen.next_key();
        self.libraries.insert(handle, lib);
        Ok(handle)
    }

    /// Loads a library.
    #[inline]
    #[cfg(windows)]
    pub fn load_library(
        &mut self,
        path: impl AsRef<OsStr>,
        _h_file: Option<NonNull<HANDLE>>,
        flags: u32,
    ) -> Result<InternalLibraryHandle, Error<Owned>> {
        let lib = match unsafe { Library::load_with_flags(&path, flags as _) } {
            Ok(lib) => lib,
            Err(e) => {
                return Err(Error::from(NativeLoaderError::LibraryLoadError {
                    library: PathBuf::from(path.as_ref().to_os_string()),
                    error: e,
                }))
            }
        };

        let handle = self.library_gen.next_key();
        self.libraries.insert(handle, lib);
        Ok(handle)
    }

    /// Unloads a library.
    #[inline]
    pub fn unload_library(&mut self, library: InternalLibraryHandle) -> Result<(), Error<Owned>> {
        if let Some(lib) = self.libraries.remove(&library) {
            match lib.close() {
                Ok(_) => Ok(()),
                Err(error) => Err(Error::from(NativeLoaderError::LibraryUnloadError {
                    handle: library,
                    error,
                })),
            }
        } else {
            Err(Error::from(NativeLoaderError::InvalidLibraryHandle {
                handle: library,
            }))
        }
    }

    /// Fetches a symbol from a library.
    #[inline]
    pub fn get_library_symbol<T>(
        &self,
        library: InternalLibraryHandle,
        symbol: &[u8],
    ) -> Result<Symbol<T>, Error<Owned>> {
        if let Some(lib) = self.libraries.get(&library) {
            match unsafe { lib.get(symbol) } {
                Ok(symbol) => Ok(symbol),
                Err(error) => Err(Error::from(NativeLoaderError::LibrarySymbolError {
                    handle: library,
                    error,
                })),
            }
        } else {
            Err(Error::from(NativeLoaderError::InvalidLibraryHandle {
                handle: library,
            }))
        }
    }

    /// Fetches the native library handle of a library.
    #[inline]
    pub fn get_native_library_handle(
        &mut self,
        library: InternalLibraryHandle,
    ) -> Result<NativeLibraryHandle, Error<Owned>> {
        if let Some(library) = self.libraries.get_mut(&library) {
            // Copy the internal handle.
            let mut tmp: Library = unsafe { std::mem::zeroed() };
            std::mem::swap(library, &mut tmp);

            let raw = tmp.into_raw();
            let mut tmp = unsafe { Library::from_raw(raw) };
            std::mem::swap(library, &mut tmp);
            std::mem::forget(tmp);

            Ok(raw as NativeLibraryHandle)
        } else {
            Err(Error::from(NativeLoaderError::InvalidLibraryHandle {
                handle: library,
            }))
        }
    }
}

/// Native loader.
#[derive(Debug)]
pub struct NativeLoader(Mutex<NativeLoaderInternal>);

impl NativeLoader {
    /// Constructs a new instance.
    pub fn new() -> Self {
        Self {
            0: Mutex::new(NativeLoaderInternal::new()),
        }
    }
}

impl Default for NativeLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for NativeLoader {
    type Target = Mutex<NativeLoaderInternal>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NativeLoader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> From<&'a NativeLoader> for LibraryLoader<NativeLibraryLoader<'a>, Owned> {
    fn from(val: &'a NativeLoader) -> Self {
        unsafe {
            LibraryLoader::from_raw(LibraryLoaderInterface {
                loader: Some(NonNull::from(val).cast()),
                vtable: NonNullConst::from(&library_loader::VTABLE),
            })
        }
    }
}

mod library_loader {
    use crate::base_interface::native_loader::{NativeLoader, NativeLoaderError};
    use emf_core_base_rs::ffi::collections::{NonNullConst, Result};
    use emf_core_base_rs::ffi::errors::Error;
    use emf_core_base_rs::ffi::library::library_loader::{
        LibraryLoader, LibraryLoaderVTable, NativeLibraryHandle, NativeLibraryLoaderVTable,
    };
    use emf_core_base_rs::ffi::library::{InternalHandle, OSPathString, Symbol, SymbolName};
    use emf_core_base_rs::ffi::{CBaseFn, TypeWrapper};
    use std::ffi::c_void;
    #[cfg(windows)]
    use std::os::windows::raw::HANDLE;
    use std::ptr::NonNull;

    pub const VTABLE: LibraryLoaderVTable = LibraryLoaderVTable {
        load_fn: TypeWrapper(load),
        unload_fn: TypeWrapper(unload),
        get_data_symbol_fn: TypeWrapper(get_data_symbol),
        get_function_symbol_fn: TypeWrapper(get_fn_symbol),
        get_extended_vtable_fn: TypeWrapper(get_extended_vtable),
    };

    const EXTENDED_VTABLE: NativeLibraryLoaderVTable = NativeLibraryLoaderVTable {
        loader_vtable: unsafe { NonNullConst::new_unchecked(&VTABLE) },
        load_ext_fn: TypeWrapper(load_extended),
        get_native_handle_fn: TypeWrapper(get_native_handle),
    };

    fn get_loader<'a>(loader: Option<NonNull<LibraryLoader>>) -> &'a NativeLoader {
        unsafe { &*loader.unwrap().cast::<NativeLoader>().as_ptr() }
    }

    #[cfg(unix)]
    unsafe extern "C-unwind" fn load(
        loader: Option<NonNull<LibraryLoader>>,
        path: OSPathString,
    ) -> Result<InternalHandle, Error> {
        match std::panic::catch_unwind(move || {
            use crate::base_interface::base_interface::utilities::os_path_to_path_buf;
            use libloading::os::unix::*;
            let path = os_path_to_path_buf(path);
            get_loader(loader)
                .lock()
                .load_library(path, RTLD_LAZY | RTLD_LOCAL)
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    #[cfg(windows)]
    unsafe extern "C-unwind" fn load(
        loader: Option<NonNull<LibraryLoader>>,
        path: OSPathString,
    ) -> Result<InternalHandle, Error> {
        match std::panic::catch_unwind(move || {
            use crate::base_interface::base_interface::utilities::os_path_to_path_buf;
            let path = os_path_to_path_buf(path);
            get_loader(loader)
                .lock()
                .load_library(path, None, 0)
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    unsafe extern "C-unwind" fn unload(
        loader: Option<NonNull<LibraryLoader>>,
        handle: InternalHandle,
    ) -> Result<i8, Error> {
        match std::panic::catch_unwind(move || {
            get_loader(loader)
                .lock()
                .unload_library(handle)
                .map_or_else(|e| Result::Err(e.into_inner()), |_v| Result::Ok(0))
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    unsafe extern "C-unwind" fn get_data_symbol(
        loader: Option<NonNull<LibraryLoader>>,
        handle: InternalHandle,
        symbol: SymbolName,
    ) -> Result<Symbol<NonNullConst<c_void>>, Error> {
        match std::panic::catch_unwind(move || {
            get_loader(loader)
                .lock()
                .get_library_symbol::<*const c_void>(handle, symbol.as_ref())
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| {
                        Result::Ok(Symbol {
                            symbol: NonNullConst::new(*v).unwrap(),
                        })
                    },
                )
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    #[allow(improper_ctypes_definitions)]
    unsafe extern "C-unwind" fn get_fn_symbol(
        loader: Option<NonNull<LibraryLoader>>,
        handle: InternalHandle,
        symbol: SymbolName,
    ) -> Result<Symbol<CBaseFn>, Error> {
        match std::panic::catch_unwind(move || {
            get_loader(loader)
                .lock()
                .get_library_symbol::<CBaseFn>(handle, symbol.as_ref())
                .map_or_else(
                    |e| Result::Err(e.into_inner()),
                    |v| Result::Ok(Symbol { symbol: *v }),
                )
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    unsafe extern "C-unwind" fn get_extended_vtable(
        _loader: Option<NonNull<LibraryLoader>>,
    ) -> NonNullConst<c_void> {
        NonNullConst::from(&EXTENDED_VTABLE).cast()
    }

    #[cfg(unix)]
    unsafe extern "C-unwind" fn load_extended(
        loader: Option<NonNull<LibraryLoader>>,
        path: OSPathString,
        flags: i32,
    ) -> Result<InternalHandle, Error> {
        match std::panic::catch_unwind(move || {
            use crate::base_interface::base_interface::utilities::os_path_to_path_buf;
            let path = os_path_to_path_buf(path);
            get_loader(loader)
                .lock()
                .load_library(path, flags)
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    #[cfg(windows)]
    unsafe extern "C-unwind" fn load_extended(
        loader: Option<NonNull<LibraryLoader>>,
        path: OSPathString,
        h_file: Option<NonNull<HANDLE>>,
        flags: u32,
    ) -> Result<InternalHandle, Error> {
        match std::panic::catch_unwind(move || {
            use crate::base_interface::base_interface::utilities::os_path_to_path_buf;
            let path = os_path_to_path_buf(path);
            get_loader(loader)
                .lock()
                .load_library(path, h_file, flags)
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }

    unsafe extern "C-unwind" fn get_native_handle(
        loader: Option<NonNull<LibraryLoader>>,
        handle: InternalHandle,
    ) -> Result<NativeLibraryHandle, Error> {
        match std::panic::catch_unwind(move || {
            get_loader(loader)
                .lock()
                .get_native_library_handle(handle)
                .map_or_else(|e| Result::Err(e.into_inner()), Result::Ok)
        }) {
            Ok(v) => v,
            Err(err) => Result::Err(Error::from(NativeLoaderError::UnknownError { error: err })),
        }
    }
}
