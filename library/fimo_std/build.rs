use std::{env, path::PathBuf};

fn main() {
    let library = cmake::Config::new("ffi")
        .define("FIMO_MACRO_HELPER_FUNCTIONS", "ON")
        .build();
    println!("cargo:rustc-link-search=native={}/lib", library.display());
    println!("cargo:rustc-link-lib=static=fimo_std");
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::builder()
        .header("wrapper.h")
        .clang_arg("-Iffi/include")
        .clang_arg("-std=c17")
        .clang_arg("-DFIMO_MACRO_HELPER_FUNCTIONS=TRUE")
        .use_core()
        .newtype_enum("Fimo.*")
        .generate_cstr(true)
        .enable_function_attribute_detection()
        .allowlist_item("fimo_.*")
        .allowlist_item("FIMO_.*")
        .allowlist_item("Fimo.*")
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
