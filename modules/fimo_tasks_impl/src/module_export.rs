use crate::{context::ContextImpl, Runtime};
use fimo_std::{
    error::Error,
    export_module,
    module::{ConstructorModule, LoadingSet, Module, ModuleConstructor, PreModule},
};

export_module! {
    mod TasksModule {
        name: "fimo_tasks_impl",
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
        exports: {
            context: fimo_tasks::symbols::fimo_tasks::Context = &ContextImpl::ffi_context(),
        },
        dyn_exports: {},
        state: Runtime,
        constructor: TasksModuleConstructor,
    }
}

struct TasksModuleConstructor;

impl<'m> ModuleConstructor<TasksModule<'m>> for TasksModuleConstructor {
    fn construct<'a>(
        module: ConstructorModule<'a, TasksModule<'m>>,
        _set: LoadingSet<'_>,
    ) -> Result<&'a mut <TasksModule<'m> as Module>::Data, Error> {
        fimo_std::panic::set_panic_hook();
        let module = module.unwrap()?;

        let runtime = Box::new(Runtime::new(module)?);
        Ok(Box::leak(runtime))
    }

    fn destroy(
        module: PreModule<'_, TasksModule<'m>>,
        data: &mut <TasksModule<'m> as Module>::Data,
    ) {
        // Safety: We make sure to not reuse the reference.
        let mut runtime = unsafe {
            let d = data;
            Box::from_raw(d)
        };

        runtime.shutdown(module);
        drop(runtime);
    }
}
