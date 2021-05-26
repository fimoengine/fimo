use crate::DependencyError;
use emf_core_base_rs::ownership::Owned;
use emf_core_base_rs::Error as Err;

/// Possible error states
#[non_exhaustive]
#[derive(Debug)]
pub enum Error<LError> {
    /// Error originating from the api.
    APIError(Err<Owned>),
    /// A dependency error.
    DependencyError(DependencyError),
    /// Error originating from a loader.
    LoaderError(LoaderError<LError>),
}

impl<LError> From<Err<Owned>> for Error<LError> {
    fn from(err: Err<Owned>) -> Self {
        Error::APIError(err)
    }
}

impl<LError> From<DependencyError> for Error<LError> {
    fn from(err: DependencyError) -> Self {
        Error::DependencyError(err)
    }
}

impl<LError> From<LoaderError<LError>> for Error<LError> {
    fn from(err: LoaderError<LError>) -> Self {
        Error::LoaderError(err)
    }
}

/// Loader error states.
#[derive(Debug)]
pub struct LoaderError<LError>(pub(crate) LError);

impl<LError> From<LError> for LoaderError<LError> {
    fn from(err: LError) -> Self {
        Self { 0: err }
    }
}
