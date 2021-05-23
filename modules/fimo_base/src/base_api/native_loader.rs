use crate::KeyGenerator;
use emf_core_base_rs::ffi::library::InternalHandle as InternalLibraryHandle;
#[cfg(unix)]
use libloading::os::unix::*;
#[cfg(windows)]
use libloading::os::windows::*;
use std::collections::HashMap;

/// Native loader.
#[derive(Debug)]
pub struct NativeLoader {
    libraries: HashMap<InternalLibraryHandle, Library>,
    library_gen:
        KeyGenerator<InternalLibraryHandle, fn(&InternalLibraryHandle) -> InternalLibraryHandle>,
}

impl Default for NativeLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeLoader {
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
}
