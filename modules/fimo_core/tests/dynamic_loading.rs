use fimo_core_interface as ci;
use fimo_module_core as module;
use fimo_module_core::ModuleLoader;
use std::alloc::System;
use std::error::Error;
use std::path::PathBuf;

#[global_allocator]
static A: System = System;

#[test]
#[cfg(feature = "rust_module")]
fn load_dynamic() -> Result<(), Box<dyn Error>> {
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

    let module_loader = module::rust_loader::RustLoader::new();
    let core_module = unsafe { module_loader.load_module_library(core_path.as_path())? };

    println!(
        "Core info: {}, Path: {}",
        core_module.get_module_info(),
        core_module.get_module_path().display()
    );

    let core_instance = unsafe { ci::rust::cast_instance(core_module.create_instance()?)? };

    println!(
        "Available interfaces: {:?}",
        core_instance.get_available_interfaces()
    );

    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    println!("Core interface: {}", core_descriptor);
    println!(
        "Core dependencies: {:?}",
        core_instance.get_interface_dependencies(core_descriptor)?
    );

    let core = unsafe { ci::rust::cast_interface(core_instance.get_interface(core_descriptor)?)? };
    print!("Core version: {}", core.lock().get_interface_version());

    Ok(())
}
