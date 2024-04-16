use std::{env, path::PathBuf};

fn main() {
    let library = cmake::Config::new("ffi")
        .configure_arg("-DFIMO_TEST_BINDINGS:BOOL=OFF")
        .configure_arg("-DFIMO_INSTALL_BINDINGS:BOOL=ON")
        .build();
    println!("cargo:rustc-link-search=native={}/lib", library.display());
    println!("cargo:rustc-link-lib=static=fimo_std");
    println!("cargo:rerun-if-changed=wrapper.h");

    #[cfg(windows)]
    println!("cargo:rustc-link-lib=dylib=Pathcch");

    let bindings = bindgen::builder()
        .header("wrapper.h")
        .clang_arg("-Iffi/include")
        .clang_arg("-std=c17")
        .clang_arg("-DFIMO_STD_BINDGEN")
        .use_core()
        .newtype_enum("Fimo.*")
        .generate_cstr(true)
        .derive_partialeq(true)
        .derive_eq(true)
        .derive_partialord(true)
        .derive_ord(true)
        .derive_hash(true)
        .enable_function_attribute_detection()
        .allowlist_item("fimo_.*")
        .allowlist_item("FIMO_.*")
        .allowlist_item("Fimo.*")
        .blocklist_type("FimoModuleRawSymbol")
        .wrap_unsafe_ops(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(DoxygenCallback))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[derive(Debug)]
struct DoxygenCallback;

impl bindgen::callbacks::ParseCallbacks for DoxygenCallback {
    fn process_comment(&self, comment: &str) -> Option<String> {
        Some(doxygen_rs::transform(comment))
    }
}
