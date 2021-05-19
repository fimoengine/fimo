use crate::base_api::{DataGuard, Locked};
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
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_void, CStr};
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;

#[derive(Default, Ord, PartialOrd, Eq, PartialEq)]
struct KeyGenerator<T, Gen: Fn(&T) -> T> {
    next_key: T,
    freed_keys: Vec<T>,
    generator: Gen,
}

impl<T: Debug, Gen: Fn(&T) -> T> Debug for KeyGenerator<T, Gen> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyGenerator")
            .field("next_key", &self.next_key)
            .field("freed_keys", &self.freed_keys)
            .finish()
    }
}

impl<T, Gen: Fn(&T) -> T> KeyGenerator<T, Gen> {
    /// Constructs an instance
    pub fn new(key: T, generator: Gen) -> Self {
        Self {
            next_key: key,
            freed_keys: vec![],
            generator,
        }
    }

    /// Fetches the next key.
    pub fn next_key(&mut self) -> T {
        if self.freed_keys.is_empty() {
            let mut next = (self.generator)(&self.next_key);
            std::mem::swap(&mut self.next_key, &mut next);
            next
        } else {
            self.freed_keys.pop().unwrap()
        }
    }

    /// Frees a key.
    pub fn free_key(&mut self, key: T) {
        self.freed_keys.push(key)
    }
}

const INVALID_LOADER: LoaderHandle = LoaderHandle { id: -1 };
const INVALID_INTERNAL_HANDLE: InternalHandle = InternalHandle { id: -1 };

#[derive(Debug)]
enum LibraryError {
    DuplicatedLibraryType { r#type: String },
    InvalidLibraryType { r#type: String },
    InvalidLibraryHandle { handle: LibraryHandle },
    InvalidLoaderHandle { handle: LoaderHandle },
    BufferOverflow { actual: usize, required: usize },
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
        }
    }
}

impl std::error::Error for LibraryError {}

/// Implementation of the library api.
#[derive(Debug)]
pub struct LibraryAPI {
    lib_type_to_loader: BTreeMap<String, LoaderHandle>,

    loaders: BTreeMap<LoaderHandle, NonNullConst<LibraryLoaderInterface>>,
    loader_to_lib_type: BTreeMap<LoaderHandle, String>,
    loader_to_libraries: BTreeMap<LoaderHandle, BTreeSet<LibraryHandle>>,

    library_to_loader: BTreeMap<LibraryHandle, LoaderHandle>,
    libraries: BTreeMap<LibraryHandle, InternalHandle>,

    loader_gen: KeyGenerator<LoaderHandle, fn(&LoaderHandle) -> LoaderHandle>,
    library_gen: KeyGenerator<LibraryHandle, fn(&LibraryHandle) -> LibraryHandle>,
}

impl Default for LibraryAPI {
    fn default() -> Self {
        Self::new()
    }
}

impl LibraryAPI {
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
        }
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
    pub fn register_loader<'loader, LT, T>(
        &mut self,
        loader: &'loader LT,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'static, Owned>, Error<Owned>>
    where
        T: LibraryLoaderAPI<'static>,
        LibraryLoader<T, Owned>: From<&'loader LT>,
    {
        let lib_type = lib_type.as_ref();
        let loader: LibraryLoader<T, Owned> = From::from(loader);

        if self.lib_type_to_loader.contains_key(lib_type) {
            Err(Error::from(Box::new(LibraryError::DuplicatedLibraryType {
                r#type: lib_type.to_string(),
            })))
        } else {
            let key = self.loader_gen.next_key();
            self.lib_type_to_loader.insert(lib_type.to_string(), key);

            self.loaders.insert(key, loader.to_interface());
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
        if handle == DEFAULT_HANDLE || !self.loaders.contains_key(&handle) {
            return Err(Error::from(Box::new(LibraryError::InvalidLoaderHandle {
                handle,
            })));
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
            Ok(unsafe { LibraryLoader::from_interface(*interface) })
        } else {
            Err(Error::from(Box::new(LibraryError::InvalidLoaderHandle {
                handle,
            })))
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
    pub fn get_loader_handle_from_type<'api>(
        &self,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'static, BorrowMutable<'api>>, Error<Owned>> {
        if let Some(loader) = self.lib_type_to_loader.get(lib_type.as_ref()) {
            Ok(unsafe { Loader::new(*loader) })
        } else {
            Err(Error::from(Box::new(LibraryError::InvalidLibraryType {
                r#type: lib_type.as_ref().to_string(),
            })))
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
    pub fn get_loader_handle_from_library<'api, 'library, O>(
        &self,
        library: &Library<'library, O>,
    ) -> Result<Loader<'library, BorrowMutable<'api>>, Error<Owned>>
    where
        O: ImmutableAccessIdentifier,
    {
        let handle = library.as_handle();
        if let Some(loader) = self.library_to_loader.get(&handle) {
            Ok(unsafe { Loader::new(*loader) })
        } else {
            Err(Error::from(Box::new(LibraryError::InvalidLibraryHandle {
                handle,
            })))
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
            Err(Error::from(Box::new(LibraryError::BufferOverflow {
                actual: buffer.len(),
                required: self.get_num_loaders(),
            })))
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
    pub unsafe fn create_library_handle(&mut self) -> Library<'static, Owned> {
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
            self.loader_to_libraries
                .get_mut(&loader)
                .unwrap()
                .remove(&handle);

            Ok(())
        } else {
            Err(Error::from(Box::new(LibraryError::InvalidLibraryHandle {
                handle,
            })))
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
            return Err(Error::from(Box::new(LibraryError::InvalidLibraryHandle {
                handle: library,
            })));
        }

        if !self.loaders.contains_key(&loader) {
            return Err(Error::from(Box::new(LibraryError::InvalidLoaderHandle {
                handle: loader,
            })));
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
            Err(Error::from(Box::new(LibraryError::InvalidLibraryHandle {
                handle: library.as_handle(),
            })))
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
        loader: &Loader<'static, O>,
        path: impl AsRef<Path>,
    ) -> Result<Library<'static, Owned>, Error<Owned>>
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

impl<'a> DataGuard<'a, LibraryAPI, Locked> {
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
    pub fn register_loader<'loader, LT, T>(
        &mut self,
        loader: &'loader LT,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'static, Owned>, Error<Owned>>
    where
        T: LibraryLoaderAPI<'static>,
        LibraryLoader<T, Owned>: From<&'loader LT>,
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
    pub fn get_loader_handle_from_type<'api>(
        &self,
        lib_type: impl AsRef<str>,
    ) -> Result<Loader<'static, BorrowMutable<'api>>, Error<Owned>> {
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
    pub fn get_loader_handle_from_library<'api, 'library, O>(
        &self,
        library: &Library<'library, O>,
    ) -> Result<Loader<'library, BorrowMutable<'api>>, Error<Owned>>
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
    pub unsafe fn create_library_handle(&mut self) -> Library<'static, Owned> {
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
        loader: &Loader<'static, O>,
        path: impl AsRef<Path>,
    ) -> Result<Library<'static, Owned>, Error<Owned>>
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
