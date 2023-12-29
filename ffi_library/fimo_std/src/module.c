#include <fimo_std/module.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/internal/module.h>

FIMO_MUST_USE FimoError fimo_module_lock(FimoContext context)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_lock(context.data);
}

FIMO_MUST_USE FimoError fimo_module_unlock(FimoContext context)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_unlock(context.data);
}

FIMO_MUST_USE FimoError fimo_module_pseudo_module_new(FimoContext context,
    const FimoModule** module)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_pseudo_module_new(context.data, module);
}

FIMO_MUST_USE FimoError fimo_module_pseudo_module_destroy(const FimoModule* module)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_pseudo_module_destroy(module->context.data, module);
}

FIMO_MUST_USE FimoError fimo_module_set_new(FimoContext context,
    FimoModuleLoadingSet* module_set)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_set_new(context.data, module_set);
}

FIMO_MUST_USE FimoError fimo_module_set_has_module(FimoContext context,
    FimoModuleLoadingSet module_set, const char* name, bool* has_module)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_set_has_module(context.data, module_set, name, has_module);
}

FIMO_MUST_USE FimoError fimo_module_set_has_symbol(FimoContext context,
    FimoModuleLoadingSet module_set, const char* name, const char* ns,
    FimoVersion version, bool* has_symbol)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_set_has_symbol(context.data, module_set, name, ns, version, has_symbol);
}

FIMO_MUST_USE FimoError fimo_module_set_append(FimoContext context,
    FimoModuleLoadingSet module_set, const char* module_path,
    FimoModuleLoadingFilter filter, void* filter_data,
    FimoModuleLoadingSuccessCallback on_success,
    FimoModuleLoadingErrorCallback on_error,
    FimoModuleLoadingCleanupCallback on_cleanup, void* user_data)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_set_append(context.data, module_set, module_path, filter,
        filter_data, on_success, on_error, on_cleanup, user_data, fimo_internal_module_export_iterator);
}

FIMO_MUST_USE FimoError fimo_module_set_dismiss(FimoContext context,
    FimoModuleLoadingSet module_set)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_set_dismiss(context.data, module_set);
}

FIMO_MUST_USE FimoError fimo_module_set_finish(FimoContext context,
    FimoModuleLoadingSet module_set)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_set_finish(context.data, module_set);
}

FIMO_MUST_USE FimoError fimo_module_find_by_name(FimoContext context, const char* name,
    const FimoModuleInfo** module)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_find_by_name(context.data, name, module);
}

FIMO_MUST_USE FimoError fimo_module_find_by_symbol(FimoContext context, const char* name,
    const char* ns, FimoVersion version, const FimoModuleInfo** module)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_find_by_symbol(context.data, name, ns, version, module);
}

FIMO_MUST_USE FimoError fimo_module_namespace_exists(FimoContext context, const char* ns,
    bool* exists)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_namespace_exists(context.data, ns, exists);
}

FIMO_MUST_USE FimoError fimo_module_namespace_include(const FimoModule* module,
    const char* ns)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_namespace_exclude(module->context.data, module, ns);
}

FIMO_MUST_USE FimoError fimo_module_namespace_exclude(const FimoModule* module,
    const char* ns)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_namespace_exclude(module->context.data, module, ns);
}

FIMO_MUST_USE FimoError fimo_module_namespace_included(const FimoModule* module,
    const char* ns, bool* is_included, bool* is_static)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_namespace_included(module->context.data, module, ns, is_included, is_static);
}

FIMO_MUST_USE FimoError fimo_module_acquire_dependency(const FimoModule* module,
    const FimoModuleInfo* dependency)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_acquire_dependency(module->context.data, module, dependency);
}

FIMO_MUST_USE FimoError fimo_module_relinquish_dependency(const FimoModule* module,
    const FimoModuleInfo* dependency)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_relinquish_dependency(module->context.data, module, dependency);
}

FIMO_MUST_USE FimoError fimo_module_has_dependency(const FimoModule* module,
    const FimoModuleInfo* other, bool* has_dependency, bool* is_static)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_has_dependency(module->context.data, module, other, has_dependency, is_static);
}

FIMO_MUST_USE FimoError fimo_module_load_symbol(const FimoModule* module, const char* name,
    const char* ns, FimoVersion version, const void** symbol)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_load_symbol(module->context.data, module, name, ns, version, symbol);
}

FIMO_MUST_USE FimoError fimo_module_unload(FimoContext context, const FimoModuleInfo* module)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_unload(context.data, module);
}

FIMO_MUST_USE FimoError fimo_module_param_query(FimoContext context, const char* module_name,
    const char* param, FimoModuleParamType* type, FimoModuleParamAccess* read,
    FimoModuleParamAccess* write)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_param_query(context.data, module_name, param, type, read, write);
}

FIMO_MUST_USE FimoError fimo_module_param_set_public(FimoContext context, const void* value,
    const char* module_name, const char* param)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_param_set_public(context.data, value, module_name, param);
}

FIMO_MUST_USE FimoError fimo_module_param_get_public(FimoContext context, void* value,
    const char* module_name, const char* param)
{
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)context.vtable;
    return vtable->module_param_get_public(context.data, value, module_name, param);
}

FIMO_MUST_USE FimoError fimo_module_param_set_dependency(const FimoModule* module, const void* value,
    const char* module_name, const char* param)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_param_set_dependency(module->context.data, module, value, module_name, param);
}

FIMO_MUST_USE FimoError fimo_module_param_get_dependency(const FimoModule* module, void* value,
    const char* module_name, const char* param)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_param_get_dependency(module->context.data, module, value, module_name, param);
}

FIMO_MUST_USE FimoError fimo_module_param_set_private(const FimoModule* module, const void* value,
    FimoModuleParam* param)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_param_set_private(module->context.data, module, value, param);
}

FIMO_MUST_USE FimoError fimo_module_param_get_private(const FimoModule* module, void* value,
    const FimoModuleParam* param)
{
    if (!module) {
        return FIMO_EINVAL;
    }
    const FimoInternalContextVTable* vtable = (const FimoInternalContextVTable*)module->context.vtable;
    return vtable->module_param_get_private(module->context.data, module, value, param);
}
