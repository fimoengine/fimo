use fimo_tasks_interface::rust::FimoTasks;
use module_loading::{get_core_interface, get_tasks_interface};
use std::alloc::System;
use std::error::Error;
use std::sync::Arc;

#[global_allocator]
static A: System = System;

#[cfg(test)]
mod runtime;
#[cfg(test)]
mod sync;
#[cfg(test)]
mod tasks;

fn initialize() -> Result<Arc<dyn FimoTasks>, Box<dyn Error>> {
    let (core_instance, core_interface) = get_core_interface()?;
    get_tasks_interface(&core_instance, &core_interface)
}