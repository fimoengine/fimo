use fimo_ffi::ObjArc;
use fimo_module::Error;
use fimo_tasks_int::rust::IFimoTasks;
use module_loading::ModuleDatabase;
use std::alloc::System;

#[global_allocator]
static A: System = System;

#[cfg(test)]
mod runtime;
#[cfg(test)]
mod sync;
#[cfg(test)]
mod tasks;

fn initialize() -> Result<ObjArc<IFimoTasks>, Error> {
    let db = ModuleDatabase::new()?;
    let (i, handle) = db.new_interface()?;
    std::mem::forget(handle);
    Ok(i)
}
