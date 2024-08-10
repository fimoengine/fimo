#include <fimo_std/module.h>
#include <fimo_std/vtable.h>

#if defined(FIMO_STD_BUILD_SHARED) && defined(FIMO_STD_EXPORT_SYMBOLS)
FIMO_EXPORT
void fimo_impl_module_info_acquire(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    info->acquire(info);
}

FIMO_EXPORT
void fimo_impl_module_info_release(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    info->release(info);
}

FIMO_EXPORT
bool fimo_impl_module_info_is_loaded(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    return info->is_loaded(info);
}

FIMO_EXPORT
FimoResult fimo_impl_module_info_lock_unload(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    return info->lock_unload(info);
}

FIMO_EXPORT
void fimo_impl_module_info_unlock_unload(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    info->unlock_unload(info);
}
#endif

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_pseudo_module_new(const FimoContext context, const FimoModule **module) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.pseudo_module_new(context.data, module);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_pseudo_module_destroy(const FimoModule *module, FimoContext *module_context) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.pseudo_module_destroy(module->context.data, module, module_context);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_new(const FimoContext context, FimoModuleLoadingSet **module_set) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_new(context.data, module_set);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_has_module(const FimoContext context, FimoModuleLoadingSet *module_set, const char *name,
                                      bool *has_module) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_has_module(context.data, module_set, name, has_module);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_has_symbol(const FimoContext context, FimoModuleLoadingSet *module_set, const char *name,
                                      const char *ns, const FimoVersion version, bool *has_symbol) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_has_symbol(context.data, module_set, name, ns, version, has_symbol);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_append_callback(const FimoContext context, FimoModuleLoadingSet *module_set,
                                           const char *module_name, const FimoModuleLoadingSuccessCallback on_success,
                                           const FimoModuleLoadingErrorCallback on_error, void *user_data) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_append_callback(context.data, module_set, module_name, on_success, on_error,
                                                 user_data);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_append_freestanding_module(const FimoModule *module, FimoModuleLoadingSet *module_set,
                                                      const FimoModuleExport *module_export) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.set_append_freestanding_module(module->context.data, module, module_set, module_export);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_append_modules(const FimoContext context, FimoModuleLoadingSet *module_set,
                                          const char *module_path, const FimoModuleLoadingFilter filter,
                                          void *filter_data) {
    void (*iterator)(bool (*)(const FimoModuleExport *, void *), void *) = fimo_impl_module_export_iterator;
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_append_modules(context.data, module_set, module_path, filter, filter_data,
                                                fimo_impl_module_export_iterator, *(const void **)&iterator);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_dismiss(const FimoContext context, FimoModuleLoadingSet *module_set) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_dismiss(context.data, module_set);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_finish(const FimoContext context, FimoModuleLoadingSet *module_set) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.set_finish(context.data, module_set);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_find_by_name(const FimoContext context, const char *name, const FimoModuleInfo **module) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.find_by_name(context.data, name, module);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_find_by_symbol(const FimoContext context, const char *name, const char *ns,
                                      const FimoVersion version, const FimoModuleInfo **module) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.find_by_symbol(context.data, name, ns, version, module);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_exists(const FimoContext context, const char *ns, bool *exists) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.namespace_exists(context.data, ns, exists);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_include(const FimoModule *module, const char *ns) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.namespace_include(module->context.data, module, ns);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_exclude(const FimoModule *module, const char *ns) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.namespace_exclude(module->context.data, module, ns);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_included(const FimoModule *module, const char *ns, bool *is_included,
                                          bool *is_static) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.namespace_included(module->context.data, module, ns, is_included, is_static);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_acquire_dependency(const FimoModule *module, const FimoModuleInfo *dependency) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.acquire_dependency(module->context.data, module, dependency);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_relinquish_dependency(const FimoModule *module, const FimoModuleInfo *dependency) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.relinquish_dependency(module->context.data, module, dependency);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_has_dependency(const FimoModule *module, const FimoModuleInfo *other, bool *has_dependency,
                                      bool *is_static) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.has_dependency(module->context.data, module, other, has_dependency, is_static);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_load_symbol(const FimoModule *module, const char *name, const char *ns,
                                   const FimoVersion version, const void **symbol) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.load_symbol(module->context.data, module, name, ns, version, symbol);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_unload(const FimoContext context, const FimoModuleInfo *module) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.unload(context.data, module);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_query(const FimoContext context, const char *module_name, const char *param,
                                   FimoModuleParamType *type, FimoModuleParamAccess *read,
                                   FimoModuleParamAccess *write) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.param_query(context.data, module_name, param, type, read, write);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_public(const FimoContext context, const void *value, const FimoModuleParamType type,
                                        const char *module_name, const char *param) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.param_set_public(context.data, value, type, module_name, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_public(const FimoContext context, void *value, FimoModuleParamType *type,
                                        const char *module_name, const char *param) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->module_v0.param_get_public(context.data, value, type, module_name, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_dependency(const FimoModule *module, const void *value, const FimoModuleParamType type,
                                            const char *module_name, const char *param) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.param_set_dependency(module->context.data, module, value, type, module_name, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_dependency(const FimoModule *module, void *value, FimoModuleParamType *type,
                                            const char *module_name, const char *param) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.param_get_dependency(module->context.data, module, value, type, module_name, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_private(const FimoModule *module, const void *value, const FimoModuleParamType type,
                                         FimoModuleParam *param) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.param_set_private(module->context.data, module, value, type, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_private(const FimoModule *module, void *value, FimoModuleParamType *type,
                                         const FimoModuleParam *param) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.param_get_private(module->context.data, module, value, type, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_inner(const FimoModule *module, const void *value, const FimoModuleParamType type,
                                       FimoModuleParamData *param) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.param_set_inner(module->context.data, module, value, type, param);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_inner(const FimoModule *module, void *value, FimoModuleParamType *type,
                                       const FimoModuleParamData *param) {
    if (module == NULL) {
        return FIMO_EINVAL;
    }
    const FimoContextVTable *vtable = module->context.vtable;
    return vtable->module_v0.param_get_inner(module->context.data, module, value, type, param);
}
