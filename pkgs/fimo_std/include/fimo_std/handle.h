#ifndef FIMO_HANDLE_H
#define FIMO_HANDLE_H

#include <fimo_std/context.h>
#include <fimo_std/modules.h>
#include <fimo_std/tasks.h>
#include <fimo_std/tracing.h>

struct FimoContextHandle {
    /// Returns the version of the initialized context.
    ///
    /// May differ from the one specified during compilation.
    FimoVersion (*get_version)();
    FimoCoreVTable core_v0;
    FimoTracingVTable tracing_v0;
    FimoModulesVTable modules_v0;
    FimoTasksVTable tasks_v0;
};

#endif // FIMO_HANDLE_H
