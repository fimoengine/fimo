/// Implementation of the library api.
#[derive(Debug)]
pub struct LibraryAPI {}

impl Default for LibraryAPI {
    fn default() -> Self {
        Self::new()
    }
}

impl LibraryAPI {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}
