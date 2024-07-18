use crate::context::ContextImpl;
use fimo_std::export_module;

export_module! {
    mod Module {
        name: "fimo_tasks",
        description: "Threading subsystem of the Fimo Engine",
        author: "Fimo",
        license: "MIT License and Apache License, Version 2.0",
        parameters: {},
        resources: {},
        namespaces: [],
        imports: {},
        exports: {},
        dyn_exports: {
            context: ContextImpl,
        },
    }
}
