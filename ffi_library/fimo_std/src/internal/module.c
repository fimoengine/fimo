#include <fimo_std/internal/module.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/internal/tracing.h>

#include <stdatomic.h>

#ifdef _WIN32
__declspec(allocate("fi_mod$a")) const FimoModuleExport* fimo_internal_modules_section_start = NULL;
__declspec(allocate("fi_mod$z")) const FimoModuleExport* fimo_internal_modules_section_end = NULL;

#define FIMO_INTERNAL_MODULES_SECTION_START fimo_internal_modules_section_start
#define FIMO_INTERNAL_MODULES_SECTION_END fimo_internal_modules_section_end
#else
// Allocate a dummy module to force the creation of the section symbols.
const FimoModuleExport* fimo_internal_modules_dummy_module __attribute__((section(FIMO_MODULE_SECTION))) = NULL;
extern const FimoModuleExport* __start_fimo_module;
extern const FimoModuleExport* __stop_fimo_module;

#define FIMO_INTERNAL_MODULES_SECTION_START __start_fimo_module
#define FIMO_INTERNAL_MODULES_SECTION_END __stop_fimo_module
#endif

typedef struct FimoInternalModuleParam {
    const FimoModule* owner;
    FimoModuleParamType type;
    FimoModuleParamAccess read;
    FimoModuleParamAccess write;
    union {
        _Atomic FimoU8 u8;
        _Atomic FimoU16 u16;
        _Atomic FimoU32 u32;
        _Atomic FimoU64 u64;
        _Atomic FimoI8 i8;
        _Atomic FimoI16 i16;
        _Atomic FimoI32 i32;
        _Atomic FimoI64 i64;
    } value;
} FimoInternalModuleParam;

void fimo_internal_module_export_iterator(FimoInternalModuleInspector inspector, void* data)
{
    if (!inspector) {
        return;
    }

    const FimoModuleExport** start = &FIMO_INTERNAL_MODULES_SECTION_START;
    const FimoModuleExport** end = &FIMO_INTERNAL_MODULES_SECTION_END;
    for (const FimoModuleExport** it = start; it != end; it++) {
        // Skip empty module declarations.
        if (!(*it)) {
            continue;
        }

        // Pass the module to the inspection function.
        if (!inspector(*it, data)) {
            break;
        }
    }
}

FIMO_MUST_USE FimoError fimo_internal_module_param_set_private(void* context, const FimoModule* module,
    const void* value, FimoModuleParam* param)
{
    if (!context || !module || !value || !param) {
        if (context) {
            FIMO_INTERNAL_TRACING_EMIT_ERROR(context, __func__, "module",
                "invalid null parameter, module='%p', value='%p', param='%p'",
                module, value, param)
        }
        return FIMO_EINVAL;
    }

    FimoInternalContext* ctx = (FimoInternalContext*)context;
    FimoInternalModuleParam* p = (FimoInternalModuleParam*)param;
    FIMO_INTERNAL_TRACING_EMIT_TRACE(ctx, __func__, "module",
        "module='%p', param='%p', owner='%d', read='%d', write='%d', type='%d'",
        module, p, p->owner, p->read, p->write, p->type)

    if (p->owner != module) {
        FIMO_INTERNAL_TRACING_EMIT_ERROR(ctx, __func__, "module",
            "module parameter owner mismatch, owner='%s', caller='%s'",
            p->owner->module_info->name, module->module_info->name)
    }

    switch (p->type) {
    case FIMO_MODULE_PARAM_TYPE_U8:
        atomic_store(&p->value.u8, *((FimoU8*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_U16:
        atomic_store(&p->value.u16, *((FimoU16*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_U32:
        atomic_store(&p->value.u32, *((FimoU32*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_U64:
        atomic_store(&p->value.u64, *((FimoU64*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_I8:
        atomic_store(&p->value.i8, *((FimoI8*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_I16:
        atomic_store(&p->value.i16, *((FimoI16*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_I32:
        atomic_store(&p->value.i32, *((FimoI32*)value));
        break;
    case FIMO_MODULE_PARAM_TYPE_I64:
        atomic_store(&p->value.i64, *((FimoI64*)value));
        break;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE FimoError fimo_internal_module_param_get_private(void* context, const FimoModule* module,
    void* value, const FimoModuleParam* param)
{
    if (!context || !module || !value || !param) {
        if (context) {
            FIMO_INTERNAL_TRACING_EMIT_ERROR(context, __func__, "module",
                "invalid null parameter, module='%p', value='%p', param='%p'",
                module, value, param)
        }
        return FIMO_EINVAL;
    }

    FimoInternalContext* ctx = (FimoInternalContext*)context;
    const FimoInternalModuleParam* p = (const FimoInternalModuleParam*)param;
    FIMO_INTERNAL_TRACING_EMIT_TRACE(ctx, __func__, "module",
        "module='%p', param='%p', owner='%d', read='%d', write='%d', type='%d'",
        module, p, p->owner, p->read, p->write, p->type)

    if (p->owner != module) {
        FIMO_INTERNAL_TRACING_EMIT_ERROR(ctx, __func__, "module",
            "module parameter owner mismatch, owner='%s', caller='%s'",
            p->owner->module_info->name, module->module_info->name)
    }

    switch (p->type) {
    case FIMO_MODULE_PARAM_TYPE_U8:
        *((FimoU8*)value) = atomic_load(&p->value.u8);
        break;
    case FIMO_MODULE_PARAM_TYPE_U16:
        *((FimoU16*)value) = atomic_load(&p->value.u16);
        break;
    case FIMO_MODULE_PARAM_TYPE_U32:
        *((FimoU32*)value) = atomic_load(&p->value.u32);
        break;
    case FIMO_MODULE_PARAM_TYPE_U64:
        *((FimoU64*)value) = atomic_load(&p->value.u64);
        break;
    case FIMO_MODULE_PARAM_TYPE_I8:
        *((FimoI8*)value) = atomic_load(&p->value.i8);
        break;
    case FIMO_MODULE_PARAM_TYPE_I16:
        *((FimoI16*)value) = atomic_load(&p->value.i16);
        break;
    case FIMO_MODULE_PARAM_TYPE_I32:
        *((FimoI32*)value) = atomic_load(&p->value.i32);
        break;
    case FIMO_MODULE_PARAM_TYPE_I64:
        *((FimoI64*)value) = atomic_load(&p->value.i64);
        break;
    }

    return FIMO_EOK;
}