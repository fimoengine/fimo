/// Implementation of the module api.
#[derive(Debug)]
pub struct ModuleAPI {}

impl Default for ModuleAPI {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleAPI {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}
