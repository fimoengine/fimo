#ifndef FIMO_TASKS_META_PACKAGE_H
#define FIMO_TASKS_META_PACKAGE_H

#include <fimo_std/impl/context_version_.h>

#include <fimo_tasks_meta/command_buffer.h>
#include <fimo_tasks_meta/futex.h>
#include <fimo_tasks_meta/pool.h>
#include <fimo_tasks_meta/task.h>
#include <fimo_tasks_meta/task_local.h>

/// Namespace for all symbols of the package.
#define FIMO_TASKS_META_SYMBOL_NAMESPACE "fimo-tasks"

#define FIMO_TASKS_META_SYMBOL_VERSION_MAJOR FIMO_CONTEXT_VERSION_MAJOR
#define FIMO_TASKS_META_SYMBOL_VERSION_MINOR FIMO_CONTEXT_VERSION_MINOR
#define FIMO_TASKS_META_SYMBOL_VERSION_PATCH FIMO_CONTEXT_VERSION_PATCH
#define FIMO_TASKS_META_SYMBOL_VERSION_PRE FIMO_CONTEXT_VERSION_PRE
#define FIMO_TASKS_META_SYMBOL_VERSION_PRE_LEN FIMO_CONTEXT_VERSION_PRE_LEN
#define FIMO_TASKS_META_SYMBOL_VERSION_BUILD FIMO_CONTEXT_VERSION_BUILD
#define FIMO_TASKS_META_SYMBOL_VERSION_BUILD_LEN FIMO_CONTEXT_VERSION_BUILD_LEN

#endif // FIMO_TASKS_META_PACKAGE_H
