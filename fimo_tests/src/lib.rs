use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use fimo_ffi::DynObj;
use fimo_module::context::{AppContext, IContext};

/// Builder for a test context.
pub struct ContextBuilder {
    actix: bool,
    core: bool,
    logging: bool,
    tasks: bool,
    paths: HashMap<String, PathBuf>,
}

impl ContextBuilder {
    /// Constructs a new instance.
    pub fn new() -> Self {
        let module_paths = std::env::var_os("FIMO_MODULES_PATHS")
            .expect("FIMO_MODULES_PATHS env variable not set");
        let module_paths = module_paths
            .into_string()
            .expect("FIMO_MODULES_PATHS does not contain valid UTF-8");
        let paths = serde_json::from_str(&module_paths)
            .expect("FIMO_MODULES_PATHS is an invalid JSON string");

        Self {
            actix: false,
            core: false,
            logging: false,
            tasks: false,
            paths,
        }
    }

    /// Adds the `actix` module.
    pub fn with_actix(mut self) -> Self {
        self.actix = true;
        self
    }

    /// Adds the `core` module.
    pub fn with_core(mut self) -> Self {
        self.core = true;
        self
    }

    /// Adds the `logging` module.
    pub fn with_logging(mut self) -> Self {
        self.logging = true;
        self
    }

    /// Adds the `tasks` module.
    pub fn with_tasks(mut self) -> Self {
        self.tasks = true;
        self
    }

    /// Fetches the path to the module root.
    pub fn module_path(&self, name: &str) -> &Path {
        self.paths
            .get(name)
            .unwrap_or_else(|| panic!("Could not find path for module {name:?}"))
    }

    /// Builds and enters the context.
    pub fn build<R>(self, f: impl FnOnce(&DynObj<dyn IContext + '_>) -> R) -> R {
        AppContext::new().enter(move |context| {
            if self.core {
                context
                    .load_module(self.module_path("core"), &[])
                    .expect("Could not load the core module");
            }

            if self.logging {
                context
                    .load_module(self.module_path("logging"), &[])
                    .expect("Could not load the logging module");
            }

            if self.actix {
                context
                    .load_module(self.module_path("actix"), &[])
                    .expect("Could not load the actix module");
            }

            if self.tasks {
                context
                    .load_module(self.module_path("tasks"), &[])
                    .expect("Could not load the tasks module");
            }

            f(context)
        })
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
