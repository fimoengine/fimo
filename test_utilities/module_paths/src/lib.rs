use std::error::Error;
use std::path::PathBuf;

pub fn core_module_path() -> Result<PathBuf, Box<dyn Error>> {
    let artifact_dir = PathBuf::from(std::env::current_exe()?.parent().unwrap().parent().unwrap());

    let core_path = if cfg!(target_os = "windows") {
        artifact_dir.join("fimo_core.dll").canonicalize()?
    } else if cfg!(target_os = "linux") {
        artifact_dir.join("libfimo_core.so").canonicalize()?
    } else if cfg!(target_os = "macos") {
        artifact_dir.join("libfimo_core.dylib").canonicalize()?
    } else {
        unimplemented!()
    };

    Ok(core_path)
}

#[cfg(feature = "tasks_module")]
pub fn tasks_module_path() -> Result<PathBuf, Box<dyn Error>> {
    let artifact_dir = PathBuf::from(std::env::current_exe()?.parent().unwrap().parent().unwrap());

    let tasks_path = if cfg!(target_os = "windows") {
        artifact_dir.join("fimo_tasks.dll").canonicalize()?
    } else if cfg!(target_os = "linux") {
        artifact_dir.join("libfimo_tasks.so").canonicalize()?
    } else if cfg!(target_os = "macos") {
        artifact_dir.join("libfimo_tasks.dylib").canonicalize()?
    } else {
        unimplemented!()
    };

    Ok(tasks_path)
}

#[cfg(feature = "actix_module")]
pub fn actix_module_path() -> Result<PathBuf, Box<dyn Error>> {
    let artifact_dir = PathBuf::from(std::env::current_exe()?.parent().unwrap().parent().unwrap());

    let core_path = if cfg!(target_os = "windows") {
        artifact_dir.join("fimo_actix.dll").canonicalize()?
    } else if cfg!(target_os = "linux") {
        artifact_dir.join("libfimo_actix.so").canonicalize()?
    } else if cfg!(target_os = "macos") {
        artifact_dir.join("libfimo_actix.dylib").canonicalize()?
    } else {
        unimplemented!()
    };

    Ok(core_path)
}
