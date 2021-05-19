//! A utility crate for bootstrapping emf modules.
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    broken_intra_doc_links
)]
use emf_core_base_rs::module::{Loader, Module, ModuleAPI};
use emf_core_base_rs::ownership::BorrowMutable;
use emf_core_base_rs::version::Version;
use emf_core_base_rs::CBaseAccess;
use std::path::Path;

mod core_loader;
mod dependency_list;
mod error;
mod manifest;

pub use core_loader::{
    CoreLoader, NativeLoader, NativeLoaderError, ValidationError as LoaderValidationError,
};
pub use dependency_list::{DependencyError, DependencyList, NodeType};
pub use error::{Error, LoaderError};
pub use manifest::{
    get_manifest_path, parse_manifest, parse_manifest_from_file, ModuleManifest, ValidationError,
};

/// Callback closure.
pub type ModuleCallback<'a> = dyn Fn(Module<'a, BorrowMutable<'a>>);

/// Configuration of the interface.
#[derive(Debug)]
pub struct CoreCFG<'l, 'a, L: CoreLoader<'a> = NativeLoader<'a>> {
    loader: &'l mut L,
    library: Option<L::Library>,
    dependencies: DependencyList<'a>,
    target_version: Version,
}

impl<'l, 'a, L: CoreLoader<'a>> Drop for CoreCFG<'l, 'a, L> {
    fn drop(&mut self) {
        let library = self.library.take();
        self.loader.unload_library(library.unwrap());
    }
}

impl<'l, 'a, L: CoreLoader<'a>> CoreCFG<'l, 'a, L> {
    /// Construct a new instance.
    ///
    /// The instance will load the core module from `core_path`.
    pub fn new(
        loader: &'l mut L,
        core_path: &Path,
        target_version: &Version,
        exports: &[(&str, Version)],
    ) -> Result<Self, Error<L::Error>> {
        let library = loader
            .load_library(core_path)
            .map_err(Into::<LoaderError<_>>::into)?;

        Ok(CoreCFG {
            loader,
            library: Some(library),
            dependencies: DependencyList::new(core_path, target_version, exports)?,
            target_version: *target_version,
        })
    }

    /// Adds a module to the list of dependencies.
    ///
    /// The loading and initialization of the module can be configured with optional closures.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fimo_bootstrap_rs::{CoreCFG, NativeLoader};
    /// use std::path::Path;
    /// # use fimo_bootstrap_rs::Error;
    /// use fimo_version_rs::new_short;
    ///
    /// # let _ = || -> Result<(), Error<_>> {
    /// let mut cfg = <CoreCFG>::new(
    ///                 &mut Default::default(),
    ///                 Path::new("./path/to/library"),
    ///                 &new_short(0, 1, 0),
    ///                 &[]
    ///             )?
    ///             .require_module(
    ///                 Path::new("./module_1"),
    ///                 &[("export_1", new_short(0, 1, 0))],
    ///                 None,
    ///                 None
    ///             )?
    ///             .require_module(
    ///                 Path::new("./module_2"),
    ///                 &[],
    ///                 None,
    ///                 Some(Box::new(|_m| println!("Do something"))),
    ///             );
    /// # Ok(())
    /// # };
    /// ```
    pub fn require_module(
        &mut self,
        module_path: &Path,
        exports: &[(&str, Version)],
        load_fn: Option<Box<ModuleCallback<'a>>>,
        init_fn: Option<Box<ModuleCallback<'a>>>,
    ) -> Result<&mut Self, Error<L::Error>> {
        self.dependencies
            .add_module(module_path, exports, load_fn, init_fn)
            .map(|_v| self)
            .map_err(From::from)
    }

    /// Initializes the interface.
    ///
    /// Calls `f` after initializing all dependencies.
    pub fn initialize<I, T>(
        &mut self,
        f: impl FnOnce(&L::Interface, &L::Extensions) -> T,
    ) -> Result<T, Error<L::Error>> {
        let (deps, dep_order) = self.dependencies.generate_dependency_order()?;
        let mut module = self
            .loader
            .fetch_module_handle(self.library.as_mut().unwrap())
            .map_err(Into::<LoaderError<_>>::into)?;
        let (interface, instance, extensions) = self
            .loader
            .initialize_interface(&mut module, &self.target_version)
            .map_err(Into::<LoaderError<_>>::into)?;

        let mut modules = Vec::new();

        let result = interface.lock(|i| -> Result<_, Error<_>> {
            // Initialize modules
            for op in &dep_order {
                match op {
                    &NodeType::Load(idx) => {
                        let loader: Loader<'_, BorrowMutable<'_>> = unsafe {
                            Loader::new(
                                ModuleAPI::get_loader_handle_from_type(i, &deps[idx].loader)?
                                    .as_handle(),
                            )
                        };
                        let mut module = ModuleAPI::add_module(i, &loader, &deps[idx].module_path)?;

                        ModuleAPI::load(i, &mut module).map(|_| {
                            if let Some(func) = &deps[idx].load_fn {
                                // Safety: func can not leak the module
                                func(unsafe { Module::new(module.as_handle()) })
                            }
                            modules.push(module);
                        })?;
                    }
                    &NodeType::Initialize(idx) => {
                        ModuleAPI::initialize(i, &mut modules[idx]).map(|_| {
                            if let Some(func) = &deps[idx].init_fn {
                                // Safety: func can not leak the module
                                func(unsafe { Module::new(modules[idx].as_handle()) })
                            }
                        })?;
                    }
                    NodeType::Root => {}
                }
            }

            Ok(())
        });

        if let Err(e) = result {
            self.loader
                .terminate_interface(module, instance)
                .map_err(Into::<LoaderError<_>>::into)?;
            return Err(e);
        }

        // Enter function
        let f_result = f(&interface, &extensions);

        let result = interface.lock(|i| -> Result<_, Error<_>> {
            // Unload modules
            for mut module in modules {
                // Ignore errors.
                let _ = ModuleAPI::terminate(i, &mut module);
                let _ = ModuleAPI::unload(i, &mut module);
                ModuleAPI::remove_module(i, module)?;
            }

            Ok(())
        });

        self.loader
            .terminate_interface(module, instance)
            .map_err(Into::<LoaderError<_>>::into)?;

        if let Err(e) = result {
            Err(e)
        } else {
            Ok(f_result)
        }
    }
}
