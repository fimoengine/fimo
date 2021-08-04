use crate::base_interface::{DataGuard, Locked};
use crate::KeyGenerator;
use emf_core_base_rs::ffi::collections::NonNullConst;
use emf_core_base_rs::ffi::library::library_loader::LibraryLoaderInterface;
use emf_core_base_rs::ffi::library::{
    InternalHandle, LibraryHandle, LibraryType, LoaderHandle, DEFAULT_HANDLE,
};
use emf_core_base_rs::ffi::CBaseFn;
use emf_core_base_rs::library::library_loader::{
    LibraryLoader, LibraryLoaderABICompat, LibraryLoaderAPI, UnknownLoader,
};
use emf_core_base_rs::library::{InternalLibrary, Library, Loader, Symbol};
use emf_core_base_rs::ownership::{
    BorrowMutable, ImmutableAccessIdentifier, MutableAccessIdentifier, Owned,
};
use emf_core_base_rs::Error;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_void, CStr};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::path::Path;
use std::pin::Pin;

const INVALID_LOADER: LoaderHandle = LoaderHandle { id: -1 };
const INVALID_INTERNAL_HANDLE: InternalHandle = InternalHandle { id: -1 };

#[derive(Debug)]
enum LibraryError {
    DuplicatedLibraryType { r#type: String },
    InvalidLibraryType { r#type: String },
    InvalidLibraryHandle { handle: LibraryHandle },
    InvalidLoaderHandle { handle: LoaderHandle },
    BufferOverflow { actual: usize, required: usize },
    RemovingDefaultHandle,
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LibraryError::DuplicatedLibraryType { r#type } => {
                write!(f, "Duplicated library type! {}", r#type)
            }
            LibraryError::InvalidLibraryType { r#type } => {
                write!(f, "Invalid library type! {}", r#type)
            }
            LibraryError::InvalidLibraryHandle { handle } => {
                write!(f, "Invalid library handle! {}", handle)
            }
            LibraryError::InvalidLoaderHandle { handle } => {
                write!(f, "Invalid loader handle! {}", handle)
            }
            LibraryError::BufferOverflow { actual, required } => {
                write!(
                    f,
                    "Buffer overflow! length: {}, required: {}",
                    actual, required
                )
            }
            LibraryError::RemovingDefaultHandle => {
                write!(f, "Default loader handle can not be unloaded!")
            }
        }
    }
}

impl std::error::Error for LibraryError {}

/// Implementation of the library api.
#[derive(Debug)]
pub struct LibraryAPI<'i> {
    lib_type_to_loader: HashMap<String, LoaderHandle>,

    loaders: HashMap<LoaderHandle, LibraryLoaderInterface>,
    loader_to_lib_type: HashMap<LoaderHandle, String>,
    loader_to_libraries: HashMap<LoaderHandle, HashSet<LibraryHandle>>,

    library_to_loader: HashMap<LibraryHandle, LoaderHandle>,
    libraries: HashMap<LibraryHandle, InternalHandle>,

    loader_gen: KeyGenerator<LoaderHandle, fn(&LoaderHandle) -> LoaderHandle>,
    library_gen: KeyGenerator<LibraryHandle, fn(&LibraryHandle) -> LibraryHandle>,

    phantom: PhantomData<fn() -> &'i ()>,
}

impl Default for LibraryAPI<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'i> LibraryAPI<'i> {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        let first_loader = LoaderHandle { id: 0 };
        let loader_generator = |old: &LoaderHandle| LoaderHandle { id: old.id + 1 };

        let first_library = LibraryHandle { id: 0 };
        let library_generator = |old: &LibraryHandle| LibraryHandle { id: old.id + 1 };

        Self {
            lib_type_to_loader: Default::default(),
            loaders: Default::default(),
            loader_to_lib_type: Default::default(),
            loader_to_libraries: Default::default(),
            library_to_loader: Default::default(),
            libraries: Default::default(),
            loader_gen: KeyGenerator::new(first_loader, loader_generator),
            library_gen: KeyGenerator::new(first_library, library_generator),
            phantom: PhantomData,
        }
    }

    /// Resets the api.
    #[inline]
    pub fn reset(&mut self) {
        let mut reloaded = Self::new();
        std::mem::swap(&mut reloaded, self);
        drop(reloaded);
    }

    /// Registers a new loader.
    ///
    /// The loader can load libraries of the type `lib_type`.
    /// The loader must outlive the binding to the interface.
    ///
    /// # Failure
    ///
    /// The function fails if the library type already exists.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn register_loader<LT, T>(
        &mut self,
        loader: Pin<&'i LT>,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'i, Owned>, Error<Owned>>
    where
        T: LibraryLoaderAPI<'i> + LibraryLoaderABICompat,
        LibraryLoader<T, Owned>: From<&'i LT>,
    {
        let lib_type = lib_type.as_ref();
        let loader: LibraryLoader<T, Owned> = From::from(loader.get_ref());

        if self.lib_type_to_loader.contains_key(lib_type) {
            Err(Error::from(LibraryError::DuplicatedLibraryType {
                r#type: lib_type.to_string(),
            }))
        } else {
            let key = self.loader_gen.next_key();
            self.lib_type_to_loader.insert(lib_type.to_string(), key);

            self.loaders.insert(key, loader.to_raw());
            self.loader_to_lib_type.insert(key, lib_type.to_string());
            self.loader_to_libraries.insert(key, Default::default());

            Ok(unsafe { Loader::new(key) })
        }
    }

    /// Unregisters an existing loader.
    ///
    /// # Failure
    ///
    /// The function fails if `loader` is invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    #[inline]
    pub fn unregister_loader(&mut self, loader: Loader<'_, Owned>) -> Result<(), Error<Owned>> {
        let handle = loader.as_handle();
        // The default handle may not be unregistered.
        if handle == DEFAULT_HANDLE {
            return Err(Error::from(LibraryError::RemovingDefaultHandle));
        }

        // Check if the loader is actually registered.
        if !self.loaders.contains_key(&handle) {
            return Err(Error::from(LibraryError::InvalidLoaderHandle { handle }));
        }

        // Unload loaded libraries
        for lib in self.loader_to_libraries[&handle].clone() {
            self.unload_library(unsafe { Library::new(lib) })?;
        }

        // Remove entries
        self.loaders.remove(&handle);
        self.loader_to_libraries.remove(&handle);
        let lib_type = self.loader_to_lib_type.remove(&handle).unwrap();

        // Remove lib type
        self.lib_type_to_loader.remove(&lib_type);

        // Free the key.
        self.loader_gen.free_key(handle);

        Ok(())
    }

    /// Fetches the interface of a library loader.
    ///
    /// # Failure
    ///
    /// The function fails if `loader` is invalid.
    ///
    /// # Return
    ///
    /// Interface on success, error otherwise.
    #[inline]
    pub fn get_loader_interface<'loader, O, T>(
        &self,
        loader: &Loader<'loader, O>,
    ) -> Result<LibraryLoader<T, O>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
        T: LibraryLoaderAPI<'loader> + LibraryLoaderABICompat,
    {
        let handle = loader.as_handle();
        if let Some(interface) = self.loaders.get(&handle) {
            Ok(unsafe { LibraryLoader::from_raw(*interface) })
        } else {
            Err(Error::from(LibraryError::InvalidLoaderHandle { handle }))
        }
    }

    /// Fetches the loader handle associated with the library type.
    ///
    /// # Failure
    ///
    /// The function fails if `lib_type` is not registered.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn get_loader_handle_from_type(
        &self,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'i, BorrowMutable<'_>>, Error<Owned>> {
        if let Some(loader) = self.lib_type_to_loader.get(lib_type.as_ref()) {
            Ok(unsafe { Loader::new(*loader) })
        } else {
            Err(Error::from(LibraryError::InvalidLibraryType {
                r#type: lib_type.as_ref().to_string(),
            }))
        }
    }

    /// Fetches the loader handle linked with the library handle.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    pub fn get_loader_handle_from_library<'library, O>(
        &self,
        library: &Library<'library, O>,
    ) -> Result<Loader<'library, BorrowMutable<'i>>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        let handle = library.as_handle();
        if let Some(loader) = self.library_to_loader.get(&handle) {
            Ok(unsafe { Loader::new(*loader) })
        } else {
            Err(Error::from(LibraryError::InvalidLibraryHandle { handle }))
        }
    }

    /// Fetches the number of registered loaders.
    ///
    /// # Return
    ///
    /// Number of registered loaders.
    #[inline]
    pub fn get_num_loaders(&self) -> usize {
        self.loaders.len()
    }

    /// Checks if a the library handle is valid.
    ///
    /// # Return
    ///
    /// [true] if the handle is valid, [false] otherwise.
    #[inline]
    pub fn library_exists<'library, O>(&self, library: &Library<'library, O>) -> bool
    where
        O: ImmutableAccessIdentifier,
    {
        self.libraries.contains_key(&library.as_handle())
    }

    /// Checks if a library type exists.
    ///
    /// # Return
    ///
    /// [true] if the type exists, [false] otherwise.
    #[inline]
    pub fn type_exists(&self, lib_type: impl AsRef<str>) -> bool {
        self.lib_type_to_loader.contains_key(lib_type.as_ref())
    }

    /// Copies the strings of the registered library types into a buffer.
    ///
    /// # Failure
    ///
    /// The function fails if `buffer.as_ref().len() < get_num_loaders()`.
    ///
    /// # Return
    ///
    /// Number of written types on success, error otherwise.
    #[inline]
    pub fn get_library_types(
        &self,
        mut buffer: impl AsMut<[LibraryType]>,
    ) -> Result<usize, Error<Owned>> {
        let buffer = buffer.as_mut();

        if buffer.len() < self.get_num_loaders() {
            Err(Error::from(LibraryError::BufferOverflow {
                actual: buffer.len(),
                required: self.get_num_loaders(),
            }))
        } else {
            for lib_type in buffer.iter_mut().zip(self.lib_type_to_loader.iter()) {
                *lib_type.0 = LibraryType::from(lib_type.1 .0.as_str())
            }

            Ok(self.get_num_loaders())
        }
    }

    /// Creates a new unlinked library handle.
    ///
    /// # Return
    ///
    /// Library handle.
    ///
    /// # Safety
    ///
    /// The handle must be linked before use.
    #[inline]
    pub unsafe fn create_library_handle(&mut self) -> Library<'i, Owned> {
        let handle = self.library_gen.next_key();
        self.library_to_loader.insert(handle, INVALID_LOADER);
        self.libraries.insert(handle, INVALID_INTERNAL_HANDLE);

        Library::new(handle)
    }

    /// Removes an existing library handle.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    ///
    /// # Safety
    ///
    /// Removing the handle does not unload the library.
    #[inline]
    pub unsafe fn remove_library_handle(
        &mut self,
        library: Library<'_, Owned>,
    ) -> Result<(), Error<Owned>> {
        let handle = library.as_handle();
        if let Some(loader) = self.library_to_loader.remove(&handle) {
            self.libraries.remove(&handle);

            // Remove from loader if it was linked.
            if let Some(library) = self.loader_to_libraries.get_mut(&loader) {
                library.remove(&handle);
            }

            Ok(())
        } else {
            Err(Error::from(LibraryError::InvalidLibraryHandle { handle }))
        }
    }

    /// Links a library handle to an internal library handle.
    ///
    /// Overrides the internal link of the library handle by setting
    /// it to the new library loader and internal handle.
    ///
    /// # Failure
    ///
    /// The function fails if `library` or `loader` are invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    ///
    /// # Safety
    ///
    /// Incorrect usage can lead to dangling handles or use-after-free errors.
    #[inline]
    pub unsafe fn link_library<'library, 'loader, O, LO, IO>(
        &mut self,
        library: &Library<'library, O>,
        loader: &Loader<'loader, LO>,
        internal: &InternalLibrary<IO>,
    ) -> Result<(), Error<Owned>>
    where
        'loader: 'library,
        O: MutableAccessIdentifier,
        LO: ImmutableAccessIdentifier,
        IO: ImmutableAccessIdentifier,
    {
        let library = library.as_handle();
        let loader = loader.as_handle();
        let internal = internal.as_handle();

        if !self.libraries.contains_key(&library) {
            return Err(Error::from(LibraryError::InvalidLibraryHandle {
                handle: library,
            }));
        }

        if !self.loaders.contains_key(&loader) {
            return Err(Error::from(LibraryError::InvalidLoaderHandle {
                handle: loader,
            }));
        }

        // Remove old link
        let old_loader = self.library_to_loader[&library];
        if old_loader != INVALID_LOADER {
            self.loader_to_libraries
                .get_mut(&old_loader)
                .unwrap()
                .remove(&library);
        }

        // Add new link
        self.libraries.insert(library, internal);
        self.library_to_loader.insert(library, loader);
        self.loader_to_libraries
            .get_mut(&loader)
            .unwrap()
            .insert(library);

        Ok(())
    }

    /// Fetches the internal handle linked with the library handle.
    ///
    /// # Failure
    ///
    /// The function fails if `handle` is invalid.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn get_internal_library_handle<'library, O>(
        &self,
        library: &Library<'library, O>,
    ) -> Result<InternalLibrary<O>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        if let Some(internal) = self.libraries.get(&library.as_handle()) {
            Ok(unsafe { InternalLibrary::new(*internal) })
        } else {
            Err(Error::from(LibraryError::InvalidLibraryHandle {
                handle: library.as_handle(),
            }))
        }
    }

    /// Loads a library. The resulting handle is unique.
    ///
    /// # Failure
    ///
    /// The function fails if `loader` or `path` is invalid or
    /// the type of the library can not be loaded with the loader.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn load_library<O>(
        &mut self,
        loader: &Loader<'i, O>,
        path: impl AsRef<Path>,
    ) -> Result<Library<'i, Owned>, Error<Owned>>
    where
        O: MutableAccessIdentifier + ImmutableAccessIdentifier,
    {
        let mut library_loader: LibraryLoader<UnknownLoader<'_>, _> =
            self.get_loader_interface(loader)?;
        let internal = unsafe { library_loader.load(&path) }?;
        let library = unsafe { self.create_library_handle() };
        unsafe { self.link_library(&library, &loader, &internal)? };

        Ok(library)
    }

    /// Unloads a library.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    #[inline]
    pub fn unload_library(&mut self, library: Library<'_, Owned>) -> Result<(), Error<Owned>> {
        let loader = self.get_loader_handle_from_library(&library)?;
        let internal = self.get_internal_library_handle(&library)?;
        let mut library_loader: LibraryLoader<UnknownLoader<'_>, _> =
            self.get_loader_interface(&loader)?;

        unsafe {
            // Remove the library.
            self.remove_library_handle(library)?;

            // Unload the library.
            library_loader.unload(internal)
        }
    }

    /// Fetches a data symbol from a library.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid or library does not contain `symbol`.
    ///
    /// # Note
    ///
    /// Some platforms may differentiate between a `function-pointer` and a `data-pointer`.
    /// See [LibraryAPI::get_function_symbol()] for fetching a function.
    ///
    /// # Return
    ///
    /// Symbol on success, error otherwise.
    #[inline]
    pub fn get_data_symbol<'library, 'handle, O, U>(
        &self,
        library: &'handle Library<'library, O>,
        symbol: impl AsRef<CStr>,
        caster: impl FnOnce(NonNullConst<c_void>) -> &'library U,
    ) -> Result<Symbol<'handle, &'library U>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        let loader = self.get_loader_handle_from_library(&library)?;
        let internal = self.get_internal_library_handle(&library)?;
        let library_loader: LibraryLoader<UnknownLoader<'_>, _> =
            self.get_loader_interface(&loader)?;

        unsafe { library_loader.get_data_symbol(&internal, &symbol, caster) }
    }

    /// Fetches a function symbol from a library.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid or library does not contain `symbol`.
    ///
    /// # Note
    ///
    /// Some platforms may differentiate between a `function-pointer` and a `data-pointer`.
    /// See [LibraryAPI::get_data_symbol()] for fetching some data.
    ///
    /// # Return
    ///
    /// Symbol on success, error otherwise.
    #[inline]
    pub fn get_function_symbol<'library, 'handle, O, U>(
        &self,
        library: &'handle Library<'library, O>,
        symbol: impl AsRef<CStr>,
        caster: impl FnOnce(CBaseFn) -> U,
    ) -> Result<Symbol<'handle, U>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        let loader = self.get_loader_handle_from_library(&library)?;
        let internal = self.get_internal_library_handle(&library)?;
        let library_loader: LibraryLoader<UnknownLoader<'_>, _> =
            self.get_loader_interface(&loader)?;

        unsafe { library_loader.get_function_symbol(&internal, &symbol, caster) }
    }
}

impl<'a, 'i> DataGuard<'a, LibraryAPI<'i>, Locked> {
    /// Registers a new loader.
    ///
    /// The loader can load libraries of the type `lib_type`.
    /// The loader must outlive the binding to the interface.
    ///
    /// # Failure
    ///
    /// The function fails if the library type already exists.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn register_loader<LT, T>(
        &mut self,
        loader: Pin<&'i LT>,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'i, Owned>, Error<Owned>>
    where
        T: LibraryLoaderAPI<'i> + LibraryLoaderABICompat,
        LibraryLoader<T, Owned>: From<&'i LT>,
    {
        self.data.register_loader(loader, lib_type)
    }

    /// Unregisters an existing loader.
    ///
    /// # Failure
    ///
    /// The function fails if `loader` is invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    #[inline]
    pub fn unregister_loader(&mut self, loader: Loader<'_, Owned>) -> Result<(), Error<Owned>> {
        self.data.unregister_loader(loader)
    }

    /// Fetches the interface of a library loader.
    ///
    /// # Failure
    ///
    /// The function fails if `loader` is invalid.
    ///
    /// # Return
    ///
    /// Interface on success, error otherwise.
    #[inline]
    pub fn get_loader_interface<'loader, O, T>(
        &self,
        loader: &Loader<'loader, O>,
    ) -> Result<LibraryLoader<T, O>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
        T: LibraryLoaderAPI<'loader> + LibraryLoaderABICompat,
    {
        self.data.get_loader_interface(loader)
    }

    /// Fetches the loader handle associated with the library type.
    ///
    /// # Failure
    ///
    /// The function fails if `lib_type` is not registered.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn get_loader_handle_from_type(
        &self,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'i, BorrowMutable<'_>>, Error<Owned>> {
        self.data.get_loader_handle_from_type(lib_type)
    }

    /// Fetches the loader handle linked with the library handle.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    pub fn get_loader_handle_from_library<'library, O>(
        &self,
        library: &Library<'library, O>,
    ) -> Result<Loader<'library, BorrowMutable<'_>>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        self.data.get_loader_handle_from_library(library)
    }

    /// Fetches the number of registered loaders.
    ///
    /// # Return
    ///
    /// Number of registered loaders.
    #[inline]
    pub fn get_num_loaders(&self) -> usize {
        self.data.get_num_loaders()
    }

    /// Checks if a the library handle is valid.
    ///
    /// # Return
    ///
    /// [true] if the handle is valid, [false] otherwise.
    #[inline]
    pub fn library_exists<'library, O>(&self, library: &Library<'library, O>) -> bool
    where
        O: ImmutableAccessIdentifier,
    {
        self.data.library_exists(library)
    }

    /// Checks if a library type exists.
    ///
    /// # Return
    ///
    /// [true] if the type exists, [false] otherwise.
    #[inline]
    pub fn type_exists(&self, lib_type: impl AsRef<str>) -> bool {
        self.data.type_exists(lib_type)
    }

    /// Copies the strings of the registered library types into a buffer.
    ///
    /// # Failure
    ///
    /// The function fails if `buffer.as_ref().len() < get_num_loaders()`.
    ///
    /// # Return
    ///
    /// Number of written types on success, error otherwise.
    #[inline]
    pub fn get_library_types(
        &self,
        buffer: impl AsMut<[LibraryType]>,
    ) -> Result<usize, Error<Owned>> {
        self.data.get_library_types(buffer)
    }

    /// Creates a new unlinked library handle.
    ///
    /// # Return
    ///
    /// Library handle.
    ///
    /// # Safety
    ///
    /// The handle must be linked before use.
    #[inline]
    pub unsafe fn create_library_handle(&mut self) -> Library<'i, Owned> {
        self.data.create_library_handle()
    }

    /// Removes an existing library handle.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    ///
    /// # Safety
    ///
    /// Removing the handle does not unload the library.
    #[inline]
    pub unsafe fn remove_library_handle(
        &mut self,
        library: Library<'_, Owned>,
    ) -> Result<(), Error<Owned>> {
        self.data.remove_library_handle(library)
    }

    /// Links a library handle to an internal library handle.
    ///
    /// Overrides the internal link of the library handle by setting
    /// it to the new library loader and internal handle.
    ///
    /// # Failure
    ///
    /// The function fails if `library` or `loader` are invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    ///
    /// # Safety
    ///
    /// Incorrect usage can lead to dangling handles or use-after-free errors.
    #[inline]
    pub unsafe fn link_library<'library, 'loader, O, LO, IO>(
        &mut self,
        library: &Library<'library, O>,
        loader: &Loader<'loader, LO>,
        internal: &InternalLibrary<IO>,
    ) -> Result<(), Error<Owned>>
    where
        'loader: 'library,
        O: MutableAccessIdentifier,
        LO: ImmutableAccessIdentifier,
        IO: ImmutableAccessIdentifier,
    {
        self.data.link_library(library, loader, internal)
    }

    /// Fetches the internal handle linked with the library handle.
    ///
    /// # Failure
    ///
    /// The function fails if `handle` is invalid.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn get_internal_library_handle<'library, O>(
        &self,
        library: &Library<'library, O>,
    ) -> Result<InternalLibrary<O>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        self.data.get_internal_library_handle(library)
    }

    /// Loads a library. The resulting handle is unique.
    ///
    /// # Failure
    ///
    /// The function fails if `loader` or `path` is invalid or
    /// the type of the library can not be loaded with the loader.
    ///
    /// # Return
    ///
    /// Handle on success, error otherwise.
    #[inline]
    pub fn load_library<O>(
        &mut self,
        loader: &Loader<'i, O>,
        path: impl AsRef<Path>,
    ) -> Result<Library<'i, Owned>, Error<Owned>>
    where
        O: MutableAccessIdentifier + ImmutableAccessIdentifier,
    {
        self.data.load_library(loader, path)
    }

    /// Unloads a library.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid.
    ///
    /// # Return
    ///
    /// Error on failure.
    #[inline]
    pub fn unload_library(&mut self, library: Library<'_, Owned>) -> Result<(), Error<Owned>> {
        self.data.unload_library(library)
    }

    /// Fetches a data symbol from a library.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid or library does not contain `symbol`.
    ///
    /// # Note
    ///
    /// Some platforms may differentiate between a `function-pointer` and a `data-pointer`.
    /// See [LibraryAPI::get_function_symbol()] for fetching a function.
    ///
    /// # Return
    ///
    /// Symbol on success, error otherwise.
    #[inline]
    pub fn get_data_symbol<'library, 'handle, O, U>(
        &self,
        library: &'handle Library<'library, O>,
        symbol: impl AsRef<CStr>,
        caster: impl FnOnce(NonNullConst<c_void>) -> &'library U,
    ) -> Result<Symbol<'handle, &'library U>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        self.data.get_data_symbol(library, symbol, caster)
    }

    /// Fetches a function symbol from a library.
    ///
    /// # Failure
    ///
    /// The function fails if `library` is invalid or library does not contain `symbol`.
    ///
    /// # Note
    ///
    /// Some platforms may differentiate between a `function-pointer` and a `data-pointer`.
    /// See [LibraryAPI::get_data_symbol()] for fetching some data.
    ///
    /// # Return
    ///
    /// Symbol on success, error otherwise.
    #[inline]
    pub fn get_function_symbol<'library, 'handle, O, U>(
        &self,
        library: &'handle Library<'library, O>,
        symbol: impl AsRef<CStr>,
        caster: impl FnOnce(CBaseFn) -> U,
    ) -> Result<Symbol<'handle, U>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        self.data.get_function_symbol(library, symbol, caster)
    }
}

#[cfg(test)]
mod tests {
    use crate::base_interface::native_loader::NativeLoader;
    use crate::base_interface::LibraryAPI;
    use emf_core_base_rs::ffi::library::library_loader::LibraryLoaderInterface;
    use emf_core_base_rs::ffi::library::InternalHandle;
    use emf_core_base_rs::library::library_loader::{LibraryLoader, UnknownLoader};
    use emf_core_base_rs::library::{
        InternalLibrary, Library, Loader, Symbol, DEFAULT_HANDLE, NATIVE_LIBRARY_TYPE_NAME,
    };
    use emf_core_base_rs::ownership::Owned;
    use std::ffi::CString;
    use std::ops::{Deref, DerefMut};
    use std::pin::Pin;

    struct LibWrapper<'a>((LibraryAPI<'a>, Pin<&'a NativeLoader>));

    impl LibWrapper<'_> {
        pub fn new() -> Self {
            let loader = Pin::new(Box::leak(Box::new(NativeLoader::new()))).into_ref();
            let mut api = LibraryAPI::new();
            api.register_loader(loader, NATIVE_LIBRARY_TYPE_NAME)
                .unwrap();
            Self { 0: (api, loader) }
        }
    }

    impl Drop for LibWrapper<'_> {
        fn drop(&mut self) {
            self.0 .0.reset();

            let loader =
                unsafe { Box::<NativeLoader>::from_raw(self.0 .1.get_ref() as *const _ as *mut _) };
            drop(loader);
        }
    }

    impl<'a> Deref for LibWrapper<'a> {
        type Target = LibraryAPI<'a>;

        fn deref(&self) -> &Self::Target {
            &self.0 .0
        }
    }

    impl<'a> DerefMut for LibWrapper<'a> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0 .0
        }
    }

    struct TestLoader;
    const TEST_LOADER: &TestLoader = &TestLoader;
    const TEST_SYMBOL: usize = 123456;
    const TEST_SYMBOL_2: usize = 654321;
    fn test_fn() -> bool {
        true
    }

    impl From<&'static TestLoader> for LibraryLoader<UnknownLoader<'static>, Owned> {
        fn from(_: &'static TestLoader) -> Self {
            use emf_core_base_rs::ffi::collections::{NonNullConst, Result};
            use emf_core_base_rs::ffi::errors::Error;
            use emf_core_base_rs::ffi::library::library_loader::{
                LibraryLoader as LibLoader, LibraryLoaderVTable,
            };
            use emf_core_base_rs::ffi::library::{OSPathString, Symbol, SymbolName};
            use emf_core_base_rs::ffi::{CBaseFn, TypeWrapper};
            use std::ffi::c_void;
            use std::ptr::NonNull;

            unsafe extern "C-unwind" fn load(
                _: Option<NonNull<LibLoader>>,
                _: OSPathString,
            ) -> Result<InternalHandle, Error> {
                Result::Ok(InternalHandle { id: 0 })
            }

            unsafe extern "C-unwind" fn unload(
                _: Option<NonNull<LibLoader>>,
                _: InternalHandle,
            ) -> Result<i8, Error> {
                Result::Ok(0)
            }

            unsafe extern "C-unwind" fn get_data_symbol(
                _: Option<NonNull<LibLoader>>,
                h: InternalHandle,
                _: SymbolName,
            ) -> Result<Symbol<NonNullConst<c_void>>, Error> {
                if h.id == 0 {
                    Result::Ok(Symbol {
                        symbol: NonNullConst::from(&TEST_SYMBOL).cast(),
                    })
                } else {
                    Result::Ok(Symbol {
                        symbol: NonNullConst::from(&TEST_SYMBOL_2).cast(),
                    })
                }
            }

            #[allow(improper_ctypes_definitions)]
            unsafe extern "C-unwind" fn get_fn_symbol(
                _: Option<NonNull<LibLoader>>,
                _: InternalHandle,
                _: SymbolName,
            ) -> Result<Symbol<CBaseFn>, Error> {
                Result::Ok(Symbol {
                    symbol: std::mem::transmute(test_fn as fn() -> bool),
                })
            }

            unsafe extern "C-unwind" fn get_extended_vtable(
                _: Option<NonNull<LibLoader>>,
            ) -> NonNullConst<c_void> {
                NonNullConst::dangling()
            }

            const VTABLE: LibraryLoaderVTable = LibraryLoaderVTable {
                load_fn: TypeWrapper(load),
                unload_fn: TypeWrapper(unload),
                get_data_symbol_fn: TypeWrapper(get_data_symbol),
                get_function_symbol_fn: TypeWrapper(get_fn_symbol),
                get_extended_vtable_fn: TypeWrapper(get_extended_vtable),
            };

            unsafe {
                LibraryLoader::from_raw(LibraryLoaderInterface {
                    loader: None,
                    vtable: NonNullConst::from(&VTABLE),
                })
            }
        }
    }

    #[test]
    fn native_loader() {
        let mut lib = LibWrapper::new();
        assert!(lib
            .get_loader_interface::<_, UnknownLoader<'_>>(&DEFAULT_HANDLE)
            .is_ok());
        assert_eq!(
            lib.get_loader_handle_from_type(NATIVE_LIBRARY_TYPE_NAME)
                .unwrap()
                .as_handle(),
            DEFAULT_HANDLE.as_handle()
        );
        assert_eq!(lib.get_num_loaders(), 1);
        assert!(lib.type_exists(NATIVE_LIBRARY_TYPE_NAME));

        let mut buffer = vec![Default::default(); lib.get_num_loaders()];
        assert_eq!(
            lib.get_library_types(&mut buffer).unwrap(),
            lib.get_num_loaders()
        );
        assert_eq!(buffer, vec![From::from(NATIVE_LIBRARY_TYPE_NAME)]);

        let default = unsafe { Loader::new(DEFAULT_HANDLE.as_handle()) };
        assert!(lib.unregister_loader(default).is_err());
    }

    #[test]
    fn new_library() {
        let mut lib = LibWrapper::new();

        let library = unsafe { lib.create_library_handle() };
        assert!(lib.library_exists(&library));
        let library_copy = unsafe { Library::<Owned>::new(library.as_handle()) };
        unsafe {
            lib.remove_library_handle(library_copy).unwrap();
        }
        assert!(!lib.library_exists(&library));
    }

    #[test]
    fn loader_registration() {
        let mut lib = LibWrapper::new();
        let invalid = unsafe { Loader::new(super::INVALID_LOADER) };
        assert!(lib.unregister_loader(invalid).is_err());

        const TEST_LOADER_TYPE: &str = "TEST_LOADER";
        assert!(!lib.type_exists(TEST_LOADER_TYPE));

        let num_loaders = lib.get_num_loaders();

        let test_loader = lib
            .register_loader(Pin::new(TEST_LOADER), TEST_LOADER_TYPE)
            .unwrap();
        assert!(lib.type_exists(TEST_LOADER_TYPE));
        assert_eq!(
            lib.get_loader_handle_from_type(TEST_LOADER_TYPE)
                .unwrap()
                .as_handle(),
            test_loader.as_handle()
        );
        assert_eq!(lib.get_num_loaders(), num_loaders + 1);

        let library = lib.load_library(&test_loader, "").unwrap();

        lib.unregister_loader(test_loader).unwrap();
        assert_eq!(lib.get_num_loaders(), num_loaders);
        assert!(!lib.library_exists(&library));
    }

    #[test]
    fn library_loading() {
        let mut lib = LibWrapper::new();
        const TEST_LOADER_TYPE: &str = "TEST_LOADER";
        let test_loader = lib
            .register_loader(Pin::new(TEST_LOADER), TEST_LOADER_TYPE)
            .unwrap();

        let library = lib.load_library(&test_loader, "").unwrap();
        assert_eq!(
            lib.get_loader_handle_from_library(&library)
                .unwrap()
                .as_handle(),
            test_loader.as_handle()
        );
        assert!(lib.library_exists(&library));
        assert_eq!(
            lib.get_internal_library_handle(&library)
                .unwrap()
                .as_handle()
                .id,
            0
        );

        let data: Symbol<'_, &'static usize> = lib
            .get_data_symbol(&library, CString::new("").unwrap(), |s| unsafe {
                &*s.cast().as_ptr()
            })
            .unwrap();
        let func: Symbol<'_, fn() -> bool> = lib
            .get_function_symbol(&library, CString::new("").unwrap(), |s| unsafe {
                std::mem::transmute(s)
            })
            .unwrap();

        assert_eq!(*AsRef::<usize>::as_ref(&data), TEST_SYMBOL);
        assert_eq!(func.as_ref()(), test_fn());

        let library_clone = unsafe { Library::new(library.as_handle()) };
        lib.unload_library(library_clone).unwrap();
        assert!(!lib.library_exists(&library));
    }

    #[test]
    fn library_linking() {
        let mut lib = LibWrapper::new();
        const TEST_LOADER_TYPE: &str = "TEST_LOADER";
        let test_loader = lib
            .register_loader(Pin::new(TEST_LOADER), TEST_LOADER_TYPE)
            .unwrap();

        let library = unsafe { lib.create_library_handle() };
        unsafe {
            lib.link_library(
                &library,
                &test_loader,
                &InternalLibrary::<Owned>::new(InternalHandle { id: 1 }),
            )
            .unwrap();
        }

        let data: Symbol<'_, &'static usize> = lib
            .get_data_symbol(&library, CString::new("").unwrap(), |s| unsafe {
                &*s.cast().as_ptr()
            })
            .unwrap();

        assert_eq!(*AsRef::<usize>::as_ref(&data), TEST_SYMBOL_2);
        lib.unload_library(library).unwrap();
    }
}