//! This crate is derived from [`cargo-xtask`](https://github.com/matklad/cargo-xtask).

mod cli;
mod task_harness;

use std::io;
use thiserror::Error;

use crate::cli::Cli;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("cargo command failed")]
    CargoCommandFailed,
    #[error("could not determine repository root")]
    CouldNotDetermineRepositoryRoot,
    #[error("invalid task provided: {0}")]
    InvalidTaskProvided(String),
}

pub type TaskResult<T> = Result<T, TaskError>;

fn main() -> TaskResult<()> {
    Cli::parse_and_run()
}
