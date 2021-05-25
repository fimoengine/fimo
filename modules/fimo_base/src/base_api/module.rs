use std::marker::PhantomData;

/// Implementation of the module api.
#[derive(Debug)]
pub struct ModuleAPI<'i> {
    phantom: PhantomData<fn() -> &'i ()>,
}

impl Default for ModuleAPI<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'i> ModuleAPI<'i> {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
