#ifndef FIMO_CONTEXT_H
#define FIMO_CONTEXT_H

#include <fimo_std/error.h>
#include <fimo_std/impl/context_version_.h>
#include <fimo_std/utils.h>
#include <fimo_std/version.h>
#include "error.h"

#ifdef __cplusplus
extern "C" {
#endif

/// Id of the fimo std interface types.
typedef enum FimoConfigId : FimoI32 {
    FIMO_CONFIG_ID_TRACING,
    FIMO_CONFIG_ID_MODULES,
} FimoConfigId;

/// Head of a config instance for some subsystem.
typedef struct FimoConfigHead {
    FimoConfigId id;
} FimoConfigHead;

/// Handle to the global functions implemented by the context.
///
/// Is not intended to be instantiated outside of the current module, as it may gain additional
/// fields without being considered a breaking change.
typedef struct FimoContextHandle FimoContextHandle;

/// Base VTable of the context.
///
/// Changing this definition is a breaking change.
typedef struct FimoCoreVTable {
    /// Deinitializes the global context.
    ///
    /// May block until all resources owned by the context are shut down.
    void (*deinit)();
    /// Checks whether the context has an error stored for the current thread.
    bool (*has_error_result)();
    /// Replaces the thread local result stored in the context with a new one.
    ///
    /// The old result is returned.
    FimoResult (*replace_result)(FimoResult new_result);
} FimoCoreVTable;

/// Initializes a new context with the given options.
///
/// If `options` is `NULL`, the context is initialized with the default options, otherwise
/// `options` must be an array terminated with a `NULL` element. The initialized context is written
/// to `context`. In case of an error, this function cleans up the configuration options.
///
/// Only one context may be initialized at any given moment.
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_context_init(const FimoConfigHead **options, const FimoContextHandle **context);

#ifdef __cplusplus
}
#endif

#endif // FIMO_CONTEXT_H
