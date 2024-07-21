use crate::context::ContextImpl;
use fimo_std::export_module;

export_module! {
    mod TasksModule {
        name: "fimo_tasks",
        description: "Threading subsystem of the Fimo Engine",
        author: "Fimo",
        license: "MIT License and Apache License, Version 2.0",
        parameters: {
            default_stack_size: {
                default: u32(524288), // 512KB
                read_group: public,
                write_group: dependency,
            },
        },
        resources: {},
        namespaces: [],
        imports: {},
        exports: {},
        dyn_exports: {
            context: ContextImpl,
        },
    }
}
