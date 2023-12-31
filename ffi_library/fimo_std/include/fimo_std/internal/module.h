#ifndef FIMO_INTERNAL_MODULE_H
#define FIMO_INTERNAL_MODULE_H

#include <fimo_std/module.h>

#include <hashmap/hashmap.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
#define FIMO_INTERNAL_EXPORT __declspec(dllexport)
#else
#define FIMO_INTERNAL_EXPORT __attribute__((visibility("default")))
#endif

typedef struct FimoInternalModuleCtx {
    struct hashmap* symbols;
    struct hashmap* modules;
} FimoInternalModuleCtx;

/**
 * Inspector function for the internal iterator of exported modules.
 *
 * @param arg0 export declaration
 * @param arg1 user defined data
 *
 * @return `true`, if the iteration should continue.
 */
typedef bool (*FimoInternalModuleInspector)(const FimoModuleExport*, void*);

/**
 * Iterates over the modules exported by the current binary.
 *
 * @param inspector inspection function.
 * @param data user defined data to pass to the inspector.
 */
FIMO_INTERNAL_EXPORT
void fimo_internal_module_export_iterator(FimoInternalModuleInspector inspector, void* data);

/**
 * Reads a module parameter with dependency read access.
 *
 * Reads the value of a module parameter with dependency read
 * access. The operation fails, if the parameter does not exist,
 * or if the parameter does not allow reading with a dependency
 * access. The caller must ensure that `value` points to an
 * instance of the same datatype as the parameter in question.
 *
 * @param module module of the caller
 * @param value pointer where to store the value
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_internal_module_param_get_dependency(void* context, const FimoModule* module,
    void* value, const char* module_name, const char* param);

/**
 * Setter for a module parameter.
 *
 * If the setter produces an error, the parameter won't be modified.
 *
 * @param module module of the caller
 * @param value value to write
 * @param param parameter to write
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_internal_module_param_set_private(void* context, const FimoModule* module,
    const void* value, FimoModuleParam* param);

/**
 * Getter for a module parameter.
 *
 * @param module module of the caller
 * @param value buffer where to store the parameter
 * @param param parameter to load
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_internal_module_param_get_private(void* context, const FimoModule* module,
    void* value, const FimoModuleParam* param);

#ifdef __cplusplus
}
#endif

#endif // FIMO_INTERNAL_MODULE_H
