use std::{env, path::PathBuf};

fn main() {
    let bindings = bindgen::builder()
        .header("wrapper.h")
        .clang_arg("-Iffi/fimo_std/include")
        .clang_arg("-Iffi/fimo_tasks/include")
        .clang_arg("-std=c17")
        .clang_arg("-DFIMO_STD_BINDGEN")
        .use_core()
        .newtype_enum("FiTasks.*")
        .generate_cstr(true)
        .derive_partialeq(true)
        .derive_eq(true)
        .derive_partialord(true)
        .derive_ord(true)
        .derive_hash(true)
        .enable_function_attribute_detection()
        .allowlist_item("fi_tasks_.*")
        .allowlist_item("FI_TASKS_.*")
        .allowlist_item("FiTasks.*")
        .blocklist_item("fimo.*")
        .blocklist_item("FIMO_.*")
        .blocklist_item("Fimo.*")
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
