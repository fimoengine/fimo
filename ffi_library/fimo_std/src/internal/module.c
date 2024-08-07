#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <Windows.h>
#include <pathcch.h>
#else
#if __APPLE__
#define _DARWIN_C_SOURCE
#else
#define _GNU_SOURCE
#endif
#include <dlfcn.h>
#include <errno.h>
#endif

#include <fimo_std/internal/module.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/internal/tracing.h>
#include <fimo_std/memory.h>
#include <fimo_std/refcount.h>

#include <inttypes.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32) || defined(WIN32)
#include <malloc.h>
#elif __APPLE__
#include <malloc/malloc.h>
#elif __ANDROID__
#include <malloc.h>
#elif __linux__
#include <malloc.h>
#endif // defined(_WIN32) || defined(WIN32)

static void *malloc_(size_t size) { return fimo_malloc(size, NULL); }

static void *realloc_(void *ptr, size_t size) {
    if (ptr == NULL) {
        return fimo_malloc(size, NULL);
    }
    if (size == 0) {
        fimo_free(ptr);
        return NULL;
    }

    size_t old_size;
#if defined(_WIN32) || defined(WIN32)
    old_size = _aligned_msize(ptr, FIMO_MALLOC_ALIGNMENT, 0);
    if (old_size == (size_t)-1) {
        return NULL;
    }
#elif __APPLE__
    old_size = malloc_size(ptr);
#elif __ANDROID__
    old_size = malloc_usable_size(ptr);
#elif __linux__
    old_size = malloc_usable_size(ptr);
#else
    old_size = 0;
#endif
    if (old_size >= size) {
        return ptr;
    }

    FimoResult error = FIMO_EOK;
    void *new_ptr = fimo_malloc(size, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        fimo_result_release(error);
        return NULL;
    }

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(new_ptr, ptr, old_size);
    fimo_free(ptr);

    return new_ptr;
}

static void free_(void *ptr) { fimo_free(ptr); }

static uint64_t combine_hashes_(const uint64_t lhs, const uint64_t rhs) {
    const uint64_t hash = lhs ^ (rhs + 0x517cc1b727220a95 + (lhs << 6) + (lhs >> 2));
    return hash;
}

static FimoResult clone_string_(const char *str, char **cloned) {
    FIMO_DEBUG_ASSERT(cloned)
    if (str == NULL) {
        *cloned = NULL;
        return FIMO_EOK;
    }

    FimoUSize str_len = strlen(str) + 1;

    FimoResult error = FIMO_EOK;
    *cloned = fimo_malloc(str_len * sizeof(char), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.strcpy)
    // ReSharper disable once CppDeprecatedEntity
    strcpy(*cloned, str);
    FIMO_PRAGMA_MSVC(warning(pop))

    return FIMO_EOK;
}

#if _WIN32
static FimoResult path_utf8_to_wide_(const char *path, wchar_t **wide) {
    FIMO_DEBUG_ASSERT(path && wide)
    int wide_path_len = MultiByteToWideChar(CP_UTF8, 0, path, -1, NULL, 0);
    if (wide_path_len <= 0) {
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }

    FimoResult error;
    *wide = fimo_malloc(wide_path_len * sizeof(wchar_t), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    int wide_conv_res = MultiByteToWideChar(CP_UTF8, 0, path, -1, *wide, wide_path_len);
    if (wide_conv_res <= 0) {
        fimo_free(*wide);
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }

    return FIMO_EOK;
}

static FimoResult path_wide_to_utf8_(const wchar_t *wide, char **utf8_path) {
    FIMO_DEBUG_ASSERT(wide && utf8_path)
    int multi_byte_path_len = WideCharToMultiByte(CP_UTF8, 0, wide, -1, NULL, 0, NULL, NULL);
    if (multi_byte_path_len <= 0) {
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }

    FimoResult error;
    *utf8_path = fimo_malloc(sizeof(char) * multi_byte_path_len, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    int multi_byte_conv_res = WideCharToMultiByte(CP_UTF8, 0, wide, -1, *utf8_path, multi_byte_path_len, NULL, NULL);
    if (multi_byte_conv_res <= 0) {
        fimo_free(*utf8_path);
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }

    return FIMO_EOK;
}
#endif

static FimoResult path_get_parent_(const char *path, char **parent) {
    FIMO_DEBUG_ASSERT(path && parent)
    if (strcmp(path, "") == 0) {
        return FIMO_EINVAL;
    }

#if _WIN32
    wchar_t *wide;
    FimoResult error = path_utf8_to_wide_(path, &wide);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    wchar_t *canonical_path;
    HRESULT res = PathAllocCanonicalize(wide, PATHCCH_ALLOW_LONG_PATHS, &canonical_path);
    fimo_free(wide);
    if (res != S_OK) {
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(res);
    }
    FimoUSize canonical_len = wcslen(canonical_path);
    res = PathCchRemoveFileSpec(canonical_path, canonical_len);
    if (res != S_OK && res != S_FALSE) {
        LocalFree(canonical_path);
    }

    error = path_wide_to_utf8_(canonical_path, parent);
    LocalFree(canonical_path);
    return error;
#else
    FimoResult error;
    *parent = fimo_malloc(PATH_MAX, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    if (realpath(path, *parent) == NULL) {
        fimo_free(*parent);
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(errno);
    }

    FimoUSize path_len = strlen(*parent);
    for (FimoISize i = (FimoISize)path_len; i >= 0; i--) {
        if ((*parent)[i] == '/' || i == 0) {
            (*parent)[i] = '\0';
            break;
        }
    }

    return FIMO_EOK;
#endif
}

static FimoResult path_join(const char *path1, const char *path2, char **joined) {
    FIMO_DEBUG_ASSERT(path1 && path2 && joined)

#if _WIN32
    wchar_t *path1_w;
    FimoResult error = path_utf8_to_wide_(path1, &path1_w);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    wchar_t *path2_w;
    error = path_utf8_to_wide_(path2, &path2_w);
    if (FIMO_RESULT_IS_ERROR(error)) {
        fimo_free(path1_w);
        return error;
    }

    wchar_t *joined_w;
    HRESULT res = PathAllocCombine(path1_w, path2_w, PATHCCH_ALLOW_LONG_PATHS, &joined_w);
    fimo_free(path1_w);
    fimo_free(path2_w);
    if (res != S_OK) {
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(res);
    }

    error = path_wide_to_utf8_(joined_w, joined);
    LocalFree(joined_w);
    return error;
#else
    FimoUSize path1_len = strlen(path1);
    if (path1_len == 0) {
        FimoResult error = clone_string_(path2, joined);
        return error;
    }

    FimoUSize path2_len = strlen(path2);
    if (path2_len == 0) {
        FimoResult error = clone_string_(path1, joined);
        return error;
    }

    bool path1_has_backslash = path1[path1_len - 1] == '/';
    bool path2_has_backslash = path2[0] == '/';
    FimoUSize joined_len = path1_len + path2_len + 1;
    if (path1_has_backslash && path2_has_backslash) {
        joined_len--;
    }
    else if (!path1_has_backslash && !path2_has_backslash) {
        joined_len++;
    }

    FimoResult error;
    *joined = fimo_malloc(sizeof(char) * joined_len, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    char *joined_ = *joined;
    strcpy(joined_, path1);

    FimoUSize idx = path1_len;
    FimoUSize start_idx = 0;
    if (!path1_has_backslash && !path2_has_backslash) {
        joined_[path1_len] = '/';
        idx++;
    }
    else if (path1_has_backslash && path2_has_backslash) {
        start_idx = 1;
    }
    strcpy(&joined_[idx], &path2[start_idx]);
    return FIMO_EOK;
#endif
}

#define GLOBAL_NS ""

#define TO_CTX_(CTX) FIMO_CONTAINER_OF(CTX, FimoInternalContext, module)

#define TO_TRACING_CTX_(CTX) &(TO_CTX_(CTX))->tracing

#define TO_MODULE_CTX_(CTX) &((FimoInternalContext *)CTX)->module

#define ERROR__(CTX, ERROR, ERROR_VAR, NAME_VAR, DESC_VAR, FMT, ...)                                                   \
    {                                                                                                                  \
        FIMO_ASSERT_TYPE_EQ(CTX, FimoInternalModuleContext *)                                                          \
        FimoResult ERROR_VAR = ERROR;                                                                                  \
        FimoResultString NAME_VAR = fimo_result_error_name(ERROR_VAR);                                                 \
        FimoResultString DESC_VAR = fimo_result_error_description(ERROR_VAR);                                          \
        FIMO_INTERNAL_TRACING_EMIT_ERROR(TO_TRACING_CTX_(ctx), __func__, "module", FMT, __VA_ARGS__)                   \
        FIMO_INTERNAL_TRACING_EMIT_ERROR(TO_TRACING_CTX_(ctx), __func__, "module", "error='%s: %s'", NAME_VAR.str,     \
                                         DESC_VAR.str)                                                                 \
        fimo_result_string_release(NAME_VAR);                                                                          \
        fimo_result_string_release(DESC_VAR);                                                                          \
    }

#define ERROR_SIMPLE__(CTX, ERROR, ERROR_VAR, NAME_VAR, DESC_VAR, MSG)                                                 \
    {                                                                                                                  \
        FIMO_ASSERT_TYPE_EQ(CTX, FimoInternalModuleContext *)                                                          \
        FimoResult ERROR_VAR = ERROR;                                                                                  \
        FimoResultString NAME_VAR = fimo_result_error_name(ERROR_VAR);                                                 \
        FimoResultString DESC_VAR = fimo_result_error_description(ERROR_VAR);                                          \
        FIMO_INTERNAL_TRACING_EMIT_ERROR_SIMPLE(TO_TRACING_CTX_(ctx), __func__, "module", MSG)                         \
        FIMO_INTERNAL_TRACING_EMIT_ERROR(TO_TRACING_CTX_(ctx), __func__, "module", "error='%s: %s'", NAME_VAR.str,     \
                                         DESC_VAR.str)                                                                 \
        fimo_result_string_release(NAME_VAR);                                                                          \
        fimo_result_string_release(DESC_VAR);                                                                          \
    }

#define ERROR_(CTX, ERROR, FMT, ...)                                                                                   \
    ERROR__(CTX, ERROR, FIMO_VAR(_fimo_private_error), FIMO_VAR(_fimo_private_error_name),                             \
            FIMO_VAR(_fimo_private_error_desc), FMT, __VA_ARGS__)

#define ERROR_SIMPLE_(CTX, ERROR, MSG)                                                                                 \
    ERROR_SIMPLE__(CTX, ERROR, FIMO_VAR(_fimo_private_error), FIMO_VAR(_fimo_private_error_name),                      \
                   FIMO_VAR(_fimo_private_error_desc), MSG)

#define WARN_(CTX, FMT, ...)                                                                                           \
    FIMO_ASSERT_TYPE_EQ(CTX, FimoInternalModuleContext *)                                                              \
    FIMO_INTERNAL_TRACING_EMIT_WARN(TO_TRACING_CTX_(ctx), __func__, "module", FMT, __VA_ARGS__)

#define WARN_SIMPLE_(CTX, MSG)                                                                                         \
    FIMO_ASSERT_TYPE_EQ(CTX, FimoInternalModuleContext *)                                                              \
    FIMO_INTERNAL_TRACING_EMIT_WARN_SIMPLE(TO_TRACING_CTX_(ctx), __func__, "module", MSG)

#define TRACE_(CTX, FMT, ...)                                                                                          \
    FIMO_ASSERT_TYPE_EQ(CTX, FimoInternalModuleContext *)                                                              \
    FIMO_INTERNAL_TRACING_EMIT_TRACE(TO_TRACING_CTX_(ctx), __func__, "module", FMT, __VA_ARGS__)

#define TRACE_SIMPLE_(CTX, MSG)                                                                                        \
    FIMO_ASSERT_TYPE_EQ(CTX, FimoInternalModuleContext *)                                                              \
    FIMO_INTERNAL_TRACING_EMIT_TRACE_SIMPLE(TO_TRACING_CTX_(ctx), __func__, "module", MSG)

#define ERR_MUTEX_INIT_ FIMO_RESULT_FROM_STRING("mutex initialization failed")
#define ERR_MUTEX_LOCK_ FIMO_RESULT_FROM_STRING("mutex lock failed")
#define ERR_MUTEX_UNLOCK_ FIMO_RESULT_FROM_STRING("mutex unlock failed")
#define ERR_MOD_MAP_ALLOC_ FIMO_RESULT_FROM_STRING("module map allocation failed")
#define ERR_SYM_MAP_ALLOC_ FIMO_RESULT_FROM_STRING("symbol map allocation failed")
#define ERR_PARAM_MAP_ALLOC_ FIMO_RESULT_FROM_STRING("parameter map allocation failed")
#define ERR_NS_MAP_ALLOC_ FIMO_RESULT_FROM_STRING("namespace map allocation failed")
#define ERR_DEP_MAP_ALLOC_ FIMO_RESULT_FROM_STRING("dependency map allocation failed")
#define ERR_MOD_INFO_DETACHED_ FIMO_RESULT_FROM_STRING("module info is detached")
#define ERR_DUPLICATE_MOD_ FIMO_RESULT_FROM_STRING("duplicate module")
#define ERR_DUPLICATE_SYM_ FIMO_RESULT_FROM_STRING("duplicate symbol")
#define ERR_DUPLICATE_PARAM_ FIMO_RESULT_FROM_STRING("duplicate parameter")
#define ERR_DUPLICATE_NS_ FIMO_RESULT_FROM_STRING("duplicate namespace")
#define ERR_DUPLICATE_DEP_ FIMO_RESULT_FROM_STRING("duplicate dependency")
#define ERR_DUPLICATE_LINK_ FIMO_RESULT_FROM_STRING("duplicate link")
#define ERR_MISSING_MOD_ FIMO_RESULT_FROM_STRING("module not found")
#define ERR_MISSING_SYM_ FIMO_RESULT_FROM_STRING("symbol not found")
#define ERR_MISSING_NS_ FIMO_RESULT_FROM_STRING("namespace not found")
#define ERR_MISSING_PARAM_ FIMO_RESULT_FROM_STRING("parameter not found")
#define ERR_MISSING_LINK_ FIMO_RESULT_FROM_STRING("link found")
#define ERR_CYCLIC_DEPENDENCY_ FIMO_RESULT_FROM_STRING("cyclic dependency detected")
#define ERR_MOD_IN_USE_ FIMO_RESULT_FROM_STRING("module in use")
#define ERR_NS_IN_USE_ FIMO_RESULT_FROM_STRING("namespace in use")
#define ERR_IS_PSEUDO_ FIMO_RESULT_FROM_STRING("is a pseudo module")
#define ERR_IS_NOT_PSEUDO_ FIMO_RESULT_FROM_STRING("is not a pseudo module")
#define ERR_STATIC_LINK_ FIMO_RESULT_FROM_STRING("link is static")
#define ERR_STATIC_NS_ FIMO_RESULT_FROM_STRING("namespace is static")
#define ERR_IS_LOADING_ FIMO_RESULT_FROM_STRING("loading in process")
#define ERR_INVALID_EXPORT_ FIMO_RESULT_FROM_STRING("invalid export")
#define ERR_NS_INCLUDED_ FIMO_RESULT_FROM_STRING("namespace already included")
#define ERR_NS_NOT_INCLUDED_ FIMO_RESULT_FROM_STRING("namespace not included")
#define ERR_NOT_A_DEPENDENCY_ FIMO_RESULT_FROM_STRING("not a dependency")
#define ERR_NO_READ_PERMISSION_ FIMO_RESULT_FROM_STRING("no read permission")
#define ERR_NO_WRITE_PERMISSION_ FIMO_RESULT_FROM_STRING("no write permission")
#define ERR_PARAM_TYPE_ FIMO_RESULT_FROM_STRING("invalid parameter type")

///////////////////////////////////////////////////////////////////////
//// Forward declarations
///////////////////////////////////////////////////////////////////////

typedef uint64_t (*HashFn_)(const void *, uint64_t, uint64_t);
typedef int (*CmpFn_)(const void *, const void *, void *);
typedef void (*FreeFn_)(void *);

struct ModuleInfo_;
struct ModuleInfoInner_;
struct ModuleInfoSymbol_;
struct ModuleInfoParam_;
struct ModuleInfoNamespace_;
struct ModuleInfoDependency_;

static const struct ModuleInfo_ *module_info_from_module_(const FimoModule *module);
static const struct ModuleInfo_ *module_info_from_module_info_(const FimoModuleInfo *module_info);
static const struct ModuleInfo_ *module_info_from_inner_(struct ModuleInfoInner_ *inner);
static void module_info_acquire_(const struct ModuleInfo_ *info);
static void module_info_release_(const struct ModuleInfo_ *info, bool cleanup_export);

static struct ModuleInfoInner_ *module_info_lock_(const struct ModuleInfo_ *inner);
static void module_info_unlock_(struct ModuleInfoInner_ *inner);
static bool module_info_is_detached_(struct ModuleInfoInner_ *inner);
static void module_info_detach_(struct ModuleInfoInner_ *inner, bool cleanup_export);
static FimoResult module_info_prevent_unload_(struct ModuleInfoInner_ *inner);
static void module_info_allow_unload_(struct ModuleInfoInner_ *inner);
static bool module_info_can_unload_(struct ModuleInfoInner_ *inner);
static FimoResult module_info_set_symbol_(struct ModuleInfoInner_ *inner, const char *name, const char *ns,
                                          FimoVersion version, FimoModuleDynamicSymbolDestructor destructor,
                                          const void *symbol, const struct ModuleInfoSymbol_ **symbol_element);
static const struct ModuleInfoSymbol_ *module_info_get_symbol_(struct ModuleInfoInner_ *inner, const char *name,
                                                               const char *ns, FimoVersion version);
static FimoResult module_info_set_param_(struct ModuleInfoInner_ *inner, const char *name,
                                         const FimoModuleParam *param);
static const struct ModuleInfoParam_ *module_info_get_param_(struct ModuleInfoInner_ *inner, const char *name);
static FimoResult module_info_set_ns_(struct ModuleInfoInner_ *inner, const char *name, bool is_static);
static const struct ModuleInfoNamespace_ *module_info_get_ns_(struct ModuleInfoInner_ *inner, const char *name);
static void module_info_delete_ns_(struct ModuleInfoInner_ *inner, const char *name);
static FimoResult module_info_set_dependency_(struct ModuleInfoInner_ *inner, const FimoModuleInfo *info,
                                              bool is_static);
static const struct ModuleInfoDependency_ *module_info_get_dependency_(struct ModuleInfoInner_ *inner,
                                                                       const char *name);
static void module_info_delete_dependency_(struct ModuleInfoInner_ *inner, const char *name);
static bool module_info_next_symbol_(struct ModuleInfoInner_ *inner, FimoUSize *it,
                                     const struct ModuleInfoSymbol_ **item);
static bool module_info_next_ns_(struct ModuleInfoInner_ *inner, FimoUSize *it,
                                 const struct ModuleInfoNamespace_ **item);
static bool module_info_next_dependency_(struct ModuleInfoInner_ *inner, FimoUSize *it,
                                         const struct ModuleInfoDependency_ **item);

struct ModuleHandle_;

static FimoResult module_handle_new_local_(void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *),
                                           const void *binary_handle, struct ModuleHandle_ **element);
static FimoResult module_handle_new_plugin_(const char *path, struct ModuleHandle_ **element);
static void module_handle_acquire_(struct ModuleHandle_ *element);
static void module_handle_release_(struct ModuleHandle_ *element);

struct LoadingSetModule_;
struct LoadingSetSymbol_;

static FimoResult loading_set_new_(FimoModuleLoadingSet **set);
static void loading_set_free_(FimoModuleLoadingSet *set);
static void loading_set_lock_(FimoModuleLoadingSet *set);
static void loading_set_unlock_(FimoModuleLoadingSet *set);
static const struct LoadingSetModule_ *loading_set_get_module_(FimoModuleLoadingSet *set, const char *name);
static const struct LoadingSetSymbol_ *loading_set_get_symbol_(FimoModuleLoadingSet *set, const char *name,
                                                               const char *ns, FimoVersion version);

struct Module_;
struct Symbol_;
struct Namespace_;

static FimoResult ctx_init_(FimoInternalModuleContext *ctx);
static void ctx_deinit_(FimoInternalModuleContext *ctx);
static FimoResult ctx_lock_(FimoInternalModuleContext *ctx);
static FimoResult ctx_unlock_(FimoInternalModuleContext *ctx);
static FimoResult ctx_add_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner);
static FimoResult ctx_remove_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner);
static FimoResult ctx_link_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner,
                                   struct ModuleInfoInner_ *other_inner);
static FimoResult ctx_unlink_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner,
                                     struct ModuleInfoInner_ *other_inner);
static bool ctx_can_remove_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner);
static FimoResult ctx_cleanup_loose_modules(FimoInternalModuleContext *ctx);
static const struct Module_ *ctx_get_module_(FimoInternalModuleContext *ctx, const char *name);
static const struct Symbol_ *ctx_get_symbol_(FimoInternalModuleContext *ctx, const char *name, const char *ns);
static const struct Symbol_ *ctx_get_symbol_compatible_(FimoInternalModuleContext *ctx, const char *name,
                                                        const char *ns, FimoVersion version);
static const struct Namespace_ *ctx_get_ns_(FimoInternalModuleContext *ctx, const char *name);
static FimoResult ctx_ns_allocate_if_not_found_(FimoInternalModuleContext *ctx, const char *name);
static void ctx_ns_free_if_empty_(FimoInternalModuleContext *ctx, const char *name);
static FimoResult ctx_ns_acquire_(FimoInternalModuleContext *ctx, const char *name);
static void ctx_ns_release_(FimoInternalModuleContext *ctx, const char *name);
static FimoResult ctx_insert_symbol_(FimoInternalModuleContext *ctx, const char *name, const char *ns,
                                     FimoVersion version, const char *module);
static void ctx_remove_symbol_(FimoInternalModuleContext *ctx, const char *name, const char *ns);

static FimoResult fi_module_new_pseudo_(FimoInternalModuleContext *ctx, const char *name, FimoModule **element);
static FimoResult fi_module_new_from_export(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                            const FimoModuleExport *export, struct ModuleHandle_ *handle,
                                            FimoModule **element);
static void fi_module_free_(struct ModuleInfoInner_ *info_inner, FimoContext *context);

static void fi_module_info_acquire_(const FimoModuleInfo *info);
static void fi_module_info_release_(const FimoModuleInfo *info);
static bool fi_module_info_is_loaded(const FimoModuleInfo *info);
static FimoResult fi_module_info_lock_unload_(const FimoModuleInfo *info);
static void fi_module_info_unlock_unload(const FimoModuleInfo *info);

static void fi_module_export_cleanup_(const FimoModuleExport *export);
static bool fi_module_export_is_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx);

///////////////////////////////////////////////////////////////////////
//// Parameter
///////////////////////////////////////////////////////////////////////

struct ParamData_ {
    const FimoModule *owner;
    FimoModuleParamType type;
    union {
        _Atomic(FimoU8) u8;
        _Atomic(FimoU16) u16;
        _Atomic(FimoU32) u32;
        _Atomic(FimoU64) u64;
        _Atomic(FimoI8) i8;
        _Atomic(FimoI16) i16;
        _Atomic(FimoI32) i32;
        _Atomic(FimoI64) i64;
    } value;
};

static bool param_data_is_owner_(const struct ParamData_ *param, const FimoModule *module) {
    FIMO_DEBUG_ASSERT(param && module)
    return param->owner == module;
}

static bool param_data_type_matches_(const struct ParamData_ *param, FimoModuleParamType type) {
    FIMO_DEBUG_ASSERT(param)
    return param->type == type;
}

static void param_data_read_(const struct ParamData_ *param, void *value, FimoModuleParamType *type) {
    FIMO_DEBUG_ASSERT(param && value && type)
    *type = param->type;
    switch (param->type) {
        case FIMO_MODULE_PARAM_TYPE_U8:
            *((FimoU8 *)value) = atomic_load(&param->value.u8);
            break;
        case FIMO_MODULE_PARAM_TYPE_U16:
            *((FimoU16 *)value) = atomic_load(&param->value.u16);
            break;
        case FIMO_MODULE_PARAM_TYPE_U32:
            *((FimoU32 *)value) = atomic_load(&param->value.u32);
            break;
        case FIMO_MODULE_PARAM_TYPE_U64:
            *((FimoU64 *)value) = atomic_load(&param->value.u64);
            break;
        case FIMO_MODULE_PARAM_TYPE_I8:
            *((FimoI8 *)value) = atomic_load(&param->value.i8);
            break;
        case FIMO_MODULE_PARAM_TYPE_I16:
            *((FimoI16 *)value) = atomic_load(&param->value.i16);
            break;
        case FIMO_MODULE_PARAM_TYPE_I32:
            *((FimoI32 *)value) = atomic_load(&param->value.i32);
            break;
        case FIMO_MODULE_PARAM_TYPE_I64:
            *((FimoI64 *)value) = atomic_load(&param->value.i64);
            break;
    }
}

static void param_data_write_(struct ParamData_ *param, const void *value) {
    FIMO_DEBUG_ASSERT(param && value)
    switch (param->type) {
        case FIMO_MODULE_PARAM_TYPE_U8:
            atomic_store(&param->value.u8, *((FimoU8 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_U16:
            atomic_store(&param->value.u16, *((FimoU16 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_U32:
            atomic_store(&param->value.u32, *((FimoU32 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_U64:
            atomic_store(&param->value.u64, *((FimoU64 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_I8:
            atomic_store(&param->value.i8, *((FimoI8 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_I16:
            atomic_store(&param->value.i16, *((FimoI16 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_I32:
            atomic_store(&param->value.i32, *((FimoI32 *)value));
            break;
        case FIMO_MODULE_PARAM_TYPE_I64:
            atomic_store(&param->value.i64, *((FimoI64 *)value));
            break;
    }
}

// Heap only.
struct FimoModuleParam {
    FimoModuleParamAccess read;
    FimoModuleParamAccess write;
    FimoModuleParamSet value_setter;
    FimoModuleParamGet value_getter;
    struct ParamData_ data;
};

static FimoResult param_new_(const FimoModuleParamAccess read, const FimoModuleParamAccess write,
                             const FimoModuleParamSet setter, const FimoModuleParamGet getter,
                             const struct ParamData_ data, FimoModuleParam **element) {
    FIMO_DEBUG_ASSERT(setter && getter && data.owner && element)
    FimoResult error = FIMO_EOK;
    *element = fimo_malloc(sizeof(**element), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    **element = (FimoModuleParam){
            .read = read,
            .write = write,
            .value_setter = setter,
            .value_getter = getter,
            .data = data,
    };

    return FIMO_EOK;
}

static void param_free_(FimoModuleParam *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free(element);
}

static bool param_can_read_public(const FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(param)
    return param->read == FIMO_MODULE_PARAM_ACCESS_PUBLIC;
}

static bool param_can_read_dependency(const FimoModuleParam *param, struct ModuleInfoInner_ *caller) {
    FIMO_DEBUG_ASSERT(param && caller)
    const char *param_owner = param->data.owner->module_info->name;
    if (module_info_get_dependency_(caller, param_owner) == NULL) {
        return false;
    }

    return param->read <= FIMO_MODULE_PARAM_ACCESS_DEPENDENCY;
}

static bool param_can_read_private(const FimoModuleParam *param, const FimoModule *caller) {
    FIMO_DEBUG_ASSERT(param && caller)
    return param->data.owner == caller;
}

static bool param_can_write_public(const FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(param)
    return param->write == FIMO_MODULE_PARAM_ACCESS_PUBLIC;
}

static bool param_can_write_dependency(const FimoModuleParam *param, struct ModuleInfoInner_ *caller) {
    FIMO_DEBUG_ASSERT(param && caller)
    const char *param_owner = param->data.owner->module_info->name;
    if (module_info_get_dependency_(caller, param_owner) == NULL) {
        return false;
    }

    return param->write <= FIMO_MODULE_PARAM_ACCESS_DEPENDENCY;
}

static bool param_can_write_private(const FimoModuleParam *param, const FimoModule *caller) {
    FIMO_DEBUG_ASSERT(param && caller)
    return param->data.owner == caller;
}

static FimoResult param_read_(const FimoModuleParam *param, const FimoModule *owner, void *value,
                              FimoModuleParamType *type) {
    FIMO_DEBUG_ASSERT(param && owner && value && type)
    return param->value_getter(owner, value, type, (const FimoModuleParamData *)&param->data);
}

static FimoResult param_write_(FimoModuleParam *param, const FimoModule *owner, const void *value,
                               const FimoModuleParamType type) {
    FIMO_DEBUG_ASSERT(param && owner && value)
    return param->value_setter(owner, value, type, (FimoModuleParamData *)&param->data);
}

///////////////////////////////////////////////////////////////////////
//// Module Info Symbol
///////////////////////////////////////////////////////////////////////

struct ModuleInfoSymbol_ {
    const char *name;
    const char *ns;
    FimoVersion version;
    FimoModuleDynamicSymbolDestructor destructor;
    FimoModuleRawSymbol symbol;
};

static FimoResult module_info_symbol_new_(const char *name, const char *ns, const FimoVersion version,
                                          const FimoModuleDynamicSymbolDestructor destructor, const void *symbol,
                                          struct ModuleInfoSymbol_ *element) {
    FIMO_DEBUG_ASSERT(name && ns && symbol && element)
    char *name_ = NULL;
    FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto name_alloc;
    }

    char *ns_ = NULL;
    error = clone_string_(ns, &ns_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto ns_alloc;
    }

    *element = (struct ModuleInfoSymbol_){
            .name = name_,
            .ns = ns_,
            .version = version,
            .destructor = destructor,
            .symbol =
                    {
                            .data = symbol,
                            .lock = 0,
                    },
    };

    return FIMO_EOK;

ns_alloc:
    fimo_free(name_);
name_alloc:
    return error;
}

static void module_info_symbol_free_(struct ModuleInfoSymbol_ *element) {
    FIMO_DEBUG_ASSERT(element)
    FIMO_DEBUG_ASSERT_FALSE(fimo_impl_module_symbol_is_used(&element->symbol.lock))
    if (element->destructor && element->symbol.data) {
        element->destructor((void *)element->symbol.data);
    }

    fimo_free((char *)element->ns);
    fimo_free((char *)element->name);
    element->ns = NULL;
    element->name = NULL;
}

static uint64_t module_info_symbol_hash_(const struct ModuleInfoSymbol_ *item, const uint64_t seed0,
                                         const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    FimoUSize ns_len = strlen(item->ns);

    const uint64_t name_hash = hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
    const uint64_t ns_hash = hashmap_xxhash3(item->ns, ns_len * sizeof(char), seed0, seed1);
    return combine_hashes_(name_hash, ns_hash);
}

static int module_info_symbol_cmp_(const struct ModuleInfoSymbol_ *a, const struct ModuleInfoSymbol_ *b,
                                   const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    const int comp = strcmp(a->name, b->name);
    if (comp != 0) {
        return comp;
    }
    return strcmp(a->ns, b->ns);
}

///////////////////////////////////////////////////////////////////////
//// Module Info Param
///////////////////////////////////////////////////////////////////////

struct ModuleInfoParam_ {
    const char *name;
    const FimoModuleParam *param;
};

static FimoResult module_info_param_new_(const char *name, const FimoModuleParam *param,
                                         struct ModuleInfoParam_ *element) {
    FIMO_DEBUG_ASSERT(name && param && element)

    char *name_ = NULL;
    FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *element = (struct ModuleInfoParam_){
            .name = name_,
            .param = param,
    };

    return FIMO_EOK;
}

static void module_info_param_free_(struct ModuleInfoParam_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    param_free_((FimoModuleParam *)element->param);
    element->name = NULL;
}

static uint64_t module_info_param_hash_(const struct ModuleInfoParam_ *item, const uint64_t seed0,
                                        const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
}

static int module_info_param_cmp_(const struct ModuleInfoParam_ *a, const struct ModuleInfoParam_ *b,
                                  const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Module Info Dependency
///////////////////////////////////////////////////////////////////////

struct ModuleInfoDependency_ {
    const char *name;
    const FimoModuleInfo *info;
    bool is_static;
};

static FimoResult module_info_dependency_new_(const FimoModuleInfo *info, const bool is_static,
                                              struct ModuleInfoDependency_ *element) {
    FIMO_DEBUG_ASSERT(info && element)
    char *name_ = NULL;
    const FimoResult error = clone_string_(info->name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *element = (struct ModuleInfoDependency_){
            .name = name_,
            .info = info,
            .is_static = is_static,
    };

    return FIMO_EOK;
}

static void module_info_dependency_free_(struct ModuleInfoDependency_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    element->name = NULL;
}

static uint64_t module_info_dependency_hash_(const struct ModuleInfoDependency_ *item, const uint64_t seed0,
                                             const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
}

static int module_info_dependency_cmp_(const struct ModuleInfoDependency_ *a, const struct ModuleInfoDependency_ *b,
                                       const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Module Info Namespace
///////////////////////////////////////////////////////////////////////

struct ModuleInfoNamespace_ {
    const char *name;
    bool is_static;
};

static FimoResult module_info_namespace_new_(const char *name, const bool is_static,
                                             struct ModuleInfoNamespace_ *element) {
    FIMO_DEBUG_ASSERT(name && element)
    char *name_ = NULL;
    const FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *element = (struct ModuleInfoNamespace_){
            .name = name_,
            .is_static = is_static,
    };

    return FIMO_EOK;
}

static void module_info_namespace_free_(struct ModuleInfoNamespace_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    element->name = NULL;
}

static uint64_t module_info_namespace_hash_(const struct ModuleInfoNamespace_ *item, const uint64_t seed0,
                                            const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
}

static int module_info_namespace_cmp_(const struct ModuleInfoNamespace_ *a, const struct ModuleInfoNamespace_ *b,
                                      const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Module Info
///////////////////////////////////////////////////////////////////////

enum ModuleType_ {
    MODULE_TYPE_REGULAR_,
    MODULE_TYPE_PSEUDO_,
};

struct ModuleInfoInner_ {
    struct hashmap *symbols;
    struct hashmap *parameters;
    struct hashmap *namespaces;
    struct hashmap *dependencies;
    struct ModuleHandle_ *handle;
    const FimoModule *module;
    const FimoModuleExport *export;
    FimoUSize unload_lock_count;
    mtx_t mutex;
};

// Heap only.
struct ModuleInfo_ {
    FimoModuleInfo info;
    enum ModuleType_ type;
    struct ModuleInfoInner_ inner;
    FimoAtomicRefCount ref_count;
};

static FimoResult module_info_new_(const char *name, const char *description, const char *author, const char *license,
                                   const char *module_path, struct ModuleHandle_ *handle,
                                   const FimoModuleExport *export, enum ModuleType_ type, struct ModuleInfo_ **info) {
    FIMO_DEBUG_ASSERT(name && info && handle)
    FimoResult error = FIMO_EOK;
    char *name_ = NULL;
    error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto name_alloc;
    }

    char *description_ = NULL;
    error = clone_string_(description, &description_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto desc_alloc;
    }

    char *author_ = NULL;
    error = clone_string_(author, &author_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto author_alloc;
    }

    char *license_ = NULL;
    error = clone_string_(license, &license_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto license_alloc;
    }

    char *module_path_ = NULL;
    error = clone_string_(module_path, &module_path_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto module_path_alloc;
    }

    struct hashmap *symbols = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct ModuleInfoSymbol_), 0, 0, 0, (HashFn_)module_info_symbol_hash_,
            (CmpFn_)module_info_symbol_cmp_, (FreeFn_)module_info_symbol_free_, NULL);
    if (symbols == NULL) {
        error = ERR_SYM_MAP_ALLOC_;
        goto alloc_symbols;
    }

    struct hashmap *parameters = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct ModuleInfoParam_), 0, 0, 0, (HashFn_)module_info_param_hash_,
            (CmpFn_)module_info_param_cmp_, (FreeFn_)module_info_param_free_, NULL);
    if (parameters == NULL) {
        error = ERR_PARAM_MAP_ALLOC_;
        goto alloc_parameters;
    }

    struct hashmap *namespaces =
            hashmap_new_with_allocator(malloc_, realloc_, free_, sizeof(struct ModuleInfoNamespace_), 0, 0, 0,
                                       (HashFn_)module_info_namespace_hash_, (CmpFn_)module_info_namespace_cmp_,
                                       (FreeFn_)module_info_namespace_free_, NULL);
    if (namespaces == NULL) {
        error = ERR_NS_MAP_ALLOC_;
        goto alloc_namespaces;
    }

    struct hashmap *dependencies =
            hashmap_new_with_allocator(malloc_, realloc_, free_, sizeof(struct ModuleInfoDependency_), 0, 0, 0,
                                       (HashFn_)module_info_dependency_hash_, (CmpFn_)module_info_dependency_cmp_,
                                       (FreeFn_)module_info_dependency_free_, NULL);
    if (dependencies == NULL) {
        error = ERR_DEP_MAP_ALLOC_;
        goto alloc_dependencies;
    }

    *info = fimo_malloc(sizeof(**info), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto info_alloc;
    }

    **info = (struct ModuleInfo_){
            .info =
                    {
                            .type = FIMO_STRUCT_TYPE_MODULE_INFO,
                            .next = NULL,
                            .name = name_,
                            .description = description_,
                            .author = author_,
                            .license = license_,
                            .module_path = module_path_,
                            .acquire = fi_module_info_acquire_,
                            .release = fi_module_info_release_,
                            .is_loaded = fi_module_info_is_loaded,
                            .lock_unload = fi_module_info_lock_unload_,
                            .unlock_unload = fi_module_info_unlock_unload,
                    },
            .type = type,
            .inner =
                    {
                            .symbols = symbols,
                            .parameters = parameters,
                            .namespaces = namespaces,
                            .dependencies = dependencies,
                            .handle = handle,
                            .module = NULL,
                            .unload_lock_count = 0,
                            .export = export,
                    },
            .ref_count = FIMO_REFCOUNT_INIT,
    };

    if (mtx_init(&(*info)->inner.mutex, mtx_plain) != thrd_success) {
        error = ERR_MUTEX_INIT_;
        goto mutex_init;
    }

    return FIMO_EOK;

mutex_init:
    fimo_free(*info);
info_alloc:
    hashmap_free(dependencies);
alloc_dependencies:
    hashmap_free(namespaces);
alloc_namespaces:
    hashmap_free(parameters);
alloc_parameters:
    hashmap_free(symbols);
alloc_symbols:
    fimo_free(module_path_);
module_path_alloc:
    fimo_free(license_);
license_alloc:
    fimo_free(author_);
author_alloc:
    fimo_free(description_);
desc_alloc:
    fimo_free(name_);
name_alloc:
    return error;
}

static const struct ModuleInfo_ *module_info_from_module_(const FimoModule *module) {
    FIMO_DEBUG_ASSERT(module)
    return module_info_from_module_info_(module->module_info);
}

static const struct ModuleInfo_ *module_info_from_module_info_(const FimoModuleInfo *module_info) {
    FIMO_DEBUG_ASSERT(module_info)
    return FIMO_CONTAINER_OF_CONST(module_info, struct ModuleInfo_, info);
}

static const struct ModuleInfo_ *module_info_from_inner_(struct ModuleInfoInner_ *inner) {
    FIMO_DEBUG_ASSERT(inner)
    return FIMO_CONTAINER_OF_CONST(inner, struct ModuleInfo_, inner);
}

static void module_info_acquire_(const struct ModuleInfo_ *info) {
    FIMO_DEBUG_ASSERT(info)
    fimo_increase_strong_count_atomic((FimoAtomicRefCount *)&info->ref_count);
}

static void module_info_release_(const struct ModuleInfo_ *info, const bool cleanup_export) {
    FIMO_DEBUG_ASSERT(info)
    const bool can_destroy = fimo_decrease_strong_count_atomic((FimoAtomicRefCount *)&info->ref_count);
    if (!can_destroy) {
        return;
    }

    struct ModuleInfoInner_ *inner = module_info_lock_(info);
    if (!module_info_is_detached_(inner)) {
        module_info_detach_(inner, cleanup_export);
    }
    module_info_unlock_(inner);

    fimo_free((char *)info->info.module_path);
    fimo_free((char *)info->info.license);
    fimo_free((char *)info->info.author);
    fimo_free((char *)info->info.description);
    fimo_free((char *)info->info.name);
    mtx_destroy((mtx_t *)&info->inner.mutex);
    fimo_free((struct ModuleInfo_ *)info);
}

static struct ModuleInfoInner_ *module_info_lock_(const struct ModuleInfo_ *inner) {
    FIMO_DEBUG_ASSERT(inner)
    const int result = mtx_lock((mtx_t *)&inner->inner.mutex);
    FIMO_ASSERT(result == thrd_success)
    return (struct ModuleInfoInner_ *)&inner->inner;
}

static void module_info_unlock_(struct ModuleInfoInner_ *inner) {
    FIMO_DEBUG_ASSERT(inner)
    const int result = mtx_unlock(&inner->mutex);
    FIMO_DEBUG_ASSERT(result == thrd_success);
    (void)result;
}

static bool module_info_is_detached_(struct ModuleInfoInner_ *inner) {
    FIMO_DEBUG_ASSERT(inner)
    return inner->handle == NULL;
}

static void module_info_detach_(struct ModuleInfoInner_ *inner, const bool cleanup_export) {
    FIMO_DEBUG_ASSERT(inner && !module_info_is_detached_(inner) && module_info_can_unload_(inner))

    // Set the handle to NULL, thereby hindering the modules ability to lock the handle.
    struct ModuleHandle_ *handle = inner->handle;
    inner->handle = NULL;

    hashmap_free(inner->dependencies);
    hashmap_free(inner->parameters);
    hashmap_free(inner->namespaces);
    hashmap_free(inner->symbols);
    if (inner->export && inner->export->module_destructor) {
        inner->export->module_destructor(inner->module, inner->module->module_data);
    }
    if (cleanup_export && inner->export) {
        fi_module_export_cleanup_(inner->export);
    }
    module_handle_release_(handle);
    inner->module = NULL;
    inner->export = NULL;
}

static FimoResult module_info_prevent_unload_(struct ModuleInfoInner_ *inner) {
    if (module_info_is_detached_(inner)) {
        return ERR_MOD_INFO_DETACHED_;
    }

    FIMO_DEBUG_ASSERT(inner)
    FimoIntOptionUSize result = fimo_usize_checked_add(inner->unload_lock_count, 1);
    FIMO_ASSERT(result.has_value)
    inner->unload_lock_count = result.data.value;

    return FIMO_EOK;
}

static void module_info_allow_unload_(struct ModuleInfoInner_ *inner) {
    FIMO_DEBUG_ASSERT(inner && !module_info_is_detached_(inner))
    FimoIntOptionUSize result = fimo_usize_checked_sub(inner->unload_lock_count, 1);
    FIMO_ASSERT(result.has_value)
    inner->unload_lock_count = result.data.value;
}

static bool module_info_can_unload_(struct ModuleInfoInner_ *inner) {
    FIMO_DEBUG_ASSERT(inner)
    return inner->unload_lock_count == 0;
}

static FimoResult module_info_set_symbol_(struct ModuleInfoInner_ *inner, const char *name, const char *ns,
                                          const FimoVersion version, FimoModuleDynamicSymbolDestructor destructor,
                                          const void *symbol, const struct ModuleInfoSymbol_ **symbol_element) {
    FIMO_DEBUG_ASSERT(inner && name && ns && symbol_element && !module_info_is_detached_(inner))
    struct ModuleInfoSymbol_ sym;
    const FimoResult error = module_info_symbol_new_(name, ns, version, destructor, symbol, &sym);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    const void *old = hashmap_set(inner->symbols, &sym);
    if (old) {
        module_info_symbol_free_(&sym);
        return ERR_DUPLICATE_SYM_;
    }
    if (hashmap_oom(inner->symbols)) {
        module_info_symbol_free_(&sym);
        return FIMO_ENOMEM;
    }
    *symbol_element = hashmap_get(inner->symbols, &sym);

    return FIMO_EOK;
}

static const struct ModuleInfoSymbol_ *module_info_get_symbol_(struct ModuleInfoInner_ *inner, const char *name,
                                                               const char *ns, const FimoVersion version) {
    FIMO_DEBUG_ASSERT(inner && name && ns)
    if (module_info_is_detached_(inner)) {
        return NULL;
    }
    const struct ModuleInfoSymbol_ *x = hashmap_get(inner->symbols, &(struct ModuleInfoSymbol_){
                                                                            .name = name,
                                                                            .ns = ns,
                                                                    });
    if (x == NULL || !fimo_version_compatible(&x->version, &version)) {
        return NULL;
    }
    return x;
}

static FimoResult module_info_set_param_(struct ModuleInfoInner_ *inner, const char *name,
                                         const FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(inner && name && param && !module_info_is_detached_(inner))
    struct ModuleInfoParam_ p;
    const FimoResult error = module_info_param_new_(name, param, &p);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    const void *old = hashmap_set(inner->parameters, &p);
    if (old) {
        module_info_param_free_(&p);
        return ERR_DUPLICATE_PARAM_;
    }

    if (hashmap_oom(inner->parameters)) {
        module_info_param_free_(&p);
        return FIMO_ENOMEM;
    }

    return FIMO_EOK;
}

static const struct ModuleInfoParam_ *module_info_get_param_(struct ModuleInfoInner_ *inner, const char *name) {
    FIMO_DEBUG_ASSERT(inner && name)
    if (module_info_is_detached_(inner)) {
        return NULL;
    }
    const struct ModuleInfoParam_ *x = hashmap_get(inner->parameters, &(struct ModuleInfoParam_){
                                                                              .name = name,
                                                                      });
    return x;
}

static FimoResult module_info_set_ns_(struct ModuleInfoInner_ *inner, const char *name, const bool is_static) {
    FIMO_DEBUG_ASSERT(inner && name && !module_info_is_detached_(inner))
    struct ModuleInfoNamespace_ ns;
    const FimoResult error = module_info_namespace_new_(name, is_static, &ns);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    const void *old = hashmap_set(inner->namespaces, &ns);
    if (old) {
        module_info_namespace_free_(&ns);
        return ERR_DUPLICATE_NS_;
    }

    if (hashmap_oom(inner->namespaces)) {
        module_info_namespace_free_(&ns);
        return FIMO_ENOMEM;
    }

    return FIMO_EOK;
}

static const struct ModuleInfoNamespace_ *module_info_get_ns_(struct ModuleInfoInner_ *inner, const char *name) {
    FIMO_DEBUG_ASSERT(inner && name)
    if (module_info_is_detached_(inner)) {
        return NULL;
    }
    const struct ModuleInfoNamespace_ *x = hashmap_get(inner->namespaces, &(struct ModuleInfoNamespace_){
                                                                                  .name = name,
                                                                          });
    return x;
}

static void module_info_delete_ns_(struct ModuleInfoInner_ *inner, const char *name) {
    FIMO_DEBUG_ASSERT(inner && name && !module_info_is_detached_(inner))
    struct ModuleInfoNamespace_ *x = (void *)hashmap_delete(inner->namespaces, &(struct ModuleInfoNamespace_){
                                                                                       .name = name,
                                                                               });
    module_info_namespace_free_(x);
}

static FimoResult module_info_set_dependency_(struct ModuleInfoInner_ *inner, const FimoModuleInfo *info,
                                              const bool is_static) {
    FIMO_DEBUG_ASSERT(inner && info && !module_info_is_detached_(inner))
    struct ModuleInfoDependency_ dep;
    const FimoResult error = module_info_dependency_new_(info, is_static, &dep);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    const void *old = hashmap_set(inner->dependencies, &dep);
    if (old) {
        module_info_dependency_free_(&dep);
        return ERR_DUPLICATE_DEP_;
    }

    if (hashmap_oom(inner->dependencies)) {
        module_info_dependency_free_(&dep);
        return FIMO_ENOMEM;
    }

    return FIMO_EOK;
}

static const struct ModuleInfoDependency_ *module_info_get_dependency_(struct ModuleInfoInner_ *inner,
                                                                       const char *name) {
    FIMO_DEBUG_ASSERT(inner && name)
    if (module_info_is_detached_(inner)) {
        return NULL;
    }
    const struct ModuleInfoDependency_ *x = hashmap_get(inner->dependencies, &(struct ModuleInfoDependency_){
                                                                                     .name = name,
                                                                             });
    return x;
}

static void module_info_delete_dependency_(struct ModuleInfoInner_ *inner, const char *name) {
    FIMO_DEBUG_ASSERT(inner && name && !module_info_is_detached_(inner))
    struct ModuleInfoDependency_ *x = (void *)hashmap_delete(inner->dependencies, &(struct ModuleInfoDependency_){
                                                                                          .name = name,
                                                                                  });
    module_info_dependency_free_(x);
}

static bool module_info_next_symbol_(struct ModuleInfoInner_ *inner, FimoUSize *it,
                                     const struct ModuleInfoSymbol_ **item) {
    FIMO_DEBUG_ASSERT(inner && it && item)
    if (module_info_is_detached_(inner)) {
        return false;
    }
    return hashmap_iter(inner->symbols, it, (void **)item);
}

static bool module_info_next_ns_(struct ModuleInfoInner_ *inner, FimoUSize *it,
                                 const struct ModuleInfoNamespace_ **item) {
    FIMO_DEBUG_ASSERT(inner && it && item)
    if (module_info_is_detached_(inner)) {
        return false;
    }
    return hashmap_iter(inner->namespaces, it, (void **)item);
}

static bool module_info_next_dependency_(struct ModuleInfoInner_ *inner, FimoUSize *it,
                                         const struct ModuleInfoDependency_ **item) {
    FIMO_DEBUG_ASSERT(inner && it && item)
    if (module_info_is_detached_(inner)) {
        return false;
    }
    return hashmap_iter(inner->dependencies, it, (void **)item);
}

///////////////////////////////////////////////////////////////////////
//// Module Handle
///////////////////////////////////////////////////////////////////////

// Heap only.
struct ModuleHandle_ {
    FimoAtomicRefCount ref_count;
    const char *module_path;
    void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *);
};

static FimoResult module_handle_new_local_(void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *),
                                           const void *binary_handle, struct ModuleHandle_ **element) {
    FIMO_DEBUG_ASSERT(export_iterator && binary_handle && element)
    FimoResult error = FIMO_EOK;
    *element = fimo_malloc(sizeof(**element), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto alloc_element;
    }

    char *module_path;

#if _WIN32
    HMODULE handle;
    bool found_handle = GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, binary_handle, &handle);
    if (!found_handle) {
        error = FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
        goto get_handle;
    }

    // GetModuleFileNameW does not provide the length of the path, so
    // we try to fetch it iteratively by doubling the path buffer on
    // each iteration.
    FimoUSize path_len_w = MAX_PATH;
    wchar_t *module_bin_path_w;
    while (true) {
        module_bin_path_w = fimo_malloc(sizeof(char) * MAX_PATH, &error);
        if (FIMO_RESULT_IS_ERROR(error)) {
            goto get_module_path_w;
        }

        if (GetModuleFileNameW(handle, module_bin_path_w, (DWORD)path_len_w) == 0) {
            fimo_free(module_bin_path_w);
            if (GetLastError() == ERROR_INSUFFICIENT_BUFFER) {
                path_len_w *= 2;
            }
            else {
                error = FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
                goto get_module_path_w;
            }
        }
        break;
    }

    char *module_bin_path;
    error = path_wide_to_utf8_(module_bin_path_w, &module_bin_path);
    fimo_free(module_bin_path_w);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto convert_path;
    }

    error = path_get_parent_(module_bin_path, &module_path);
    fimo_free(module_bin_path);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto get_parent;
    }

#else
    Dl_info dl_info;
    if (dladdr(binary_handle, &dl_info) == 0) {
        error = FIMO_RESULT_FROM_STRING("`binary_handle` does not belong to a shared library");
        goto find_symbol;
    }

    error = path_get_parent_(dl_info.dli_fname, &module_path);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto get_module_path;
    }

    const char *module_bin_path = export_iterator == fimo_impl_module_export_iterator ? NULL : dl_info.dli_fname;
    void *handle = dlopen(module_bin_path, RTLD_NOW | RTLD_LOCAL | RTLD_NOLOAD);
    if (handle == NULL) {
        const char *error_str = dlerror();
        FimoUSize error_str_len = strlen(error_str);
        char *error_str_cpy = fimo_calloc(error_str_len + 1, &error);
        if (FIMO_RESULT_IS_ERROR(error)) {
            FIMO_RESULT_IGNORE(error);
            error = FIMO_RESULT_FROM_STRING("unknown dlopen failure");
        }
        else {
            memcpy(error_str_cpy, error_str, error_str_len);
            error = FIMO_RESULT_FROM_DYNAMIC_STRING(error_str_cpy);
        }
        goto open_library;
    }
#endif

    **element = (struct ModuleHandle_){
            .ref_count = FIMO_REFCOUNT_INIT,
            .module_path = module_path,
            .export_iterator = export_iterator,
    };

    return FIMO_EOK;

#if _WIN32
get_parent:;
convert_path:;
get_module_path_w:
    CloseHandle(handle);
get_handle:
    fimo_free(*element);
#else
open_library:;
    fimo_free(module_path);
get_module_path:;
find_symbol:
    fimo_free(*element);
#endif
alloc_element:
    return error;
}

static FimoResult module_handle_new_plugin_(const char *path, struct ModuleHandle_ **element) {
    FIMO_DEBUG_ASSERT(path && element)
    char *module_path;
    FimoResult error = path_get_parent_(path, &module_path);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto get_path_parent;
    }

    *element = fimo_malloc(sizeof(**element), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto alloc_element;
    }

    void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *) = NULL;

#if _WIN32
    wchar_t *wide_path;
    error = path_utf8_to_wide_(path, &wide_path);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto get_wide_path;
    }

    HMODULE handle =
            LoadLibraryExW(wide_path, NULL, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
    if (handle == NULL) {
        error = FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
        goto load_library;
    }

    FARPROC symbol = GetProcAddress(handle, "fimo_impl_module_export_iterator");
    if (symbol == NULL) {
        error = FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
        goto load_symbol;
    }

    *(FARPROC *)&export_iterator = symbol;
    fimo_free(wide_path);
#else
    void *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL | RTLD_NODELETE);
    if (handle == NULL) {
        const char *error_str = dlerror();
        FimoUSize error_str_len = strlen(error_str);
        char *error_str_cpy = fimo_calloc(error_str_len + 1, &error);
        if (FIMO_RESULT_IS_ERROR(error)) {
            FIMO_RESULT_IGNORE(error);
            error = FIMO_RESULT_FROM_STRING("unknown dlopen failure");
        }
        else {
            memcpy(error_str_cpy, error_str, error_str_len);
            error = FIMO_RESULT_FROM_DYNAMIC_STRING(error_str_cpy);
        }
        goto open_library;
    }

    dlerror();
    void *symbol = dlsym(handle, "fimo_impl_module_export_iterator");
    const char *error_str = dlerror();
    if (error_str) {
        FimoUSize error_str_len = strlen(error_str);
        char *error_str_cpy = fimo_calloc(error_str_len + 1, &error);
        if (FIMO_RESULT_IS_ERROR(error)) {
            FIMO_RESULT_IGNORE(error);
            error = FIMO_RESULT_FROM_STRING("unknown dlsym failure");
        }
        else {
            memcpy(error_str_cpy, error_str, error_str_len);
            error = FIMO_RESULT_FROM_DYNAMIC_STRING(error_str_cpy);
        }
        goto load_symbol;
    }

    *(void **)(&export_iterator) = symbol;
#endif

    **element = (struct ModuleHandle_){
            .ref_count = FIMO_REFCOUNT_INIT,
            .module_path = module_path,
            .export_iterator = export_iterator,
    };

    return FIMO_EOK;

#if _WIN32
load_symbol:
    FreeLibrary(handle);
load_library:;
    fimo_free(wide_path);
get_wide_path:
    fimo_free(*element);
#else
load_symbol:
    dlclose(handle);
open_library:
    fimo_free(*element);
#endif
alloc_element:
    fimo_free(module_path);
get_path_parent:
    return error;
}

static void module_handle_acquire_(struct ModuleHandle_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_increase_strong_count_atomic(&element->ref_count);
}

static void module_handle_release_(struct ModuleHandle_ *element) {
    FIMO_DEBUG_ASSERT(element)
    bool can_destroy = fimo_decrease_strong_count_atomic(&element->ref_count);
    if (!can_destroy) {
        return;
    }
    fimo_free((char *)element->module_path);
    fimo_free(element);
}

///////////////////////////////////////////////////////////////////////
//// Loading Set Module
///////////////////////////////////////////////////////////////////////

enum ModuleLoadStatus_ {
    MODULE_LOAD_STATUS_UNLOADED_,
    MODULE_LOAD_STATUS_LOADED_,
    MODULE_LOAD_STATUS_ERROR_,
};

struct LoadingSetCallback_ {
    void *data;
    FimoModuleLoadingErrorCallback error;
    FimoModuleLoadingSuccessCallback success;
};

struct LoadingSetModule_ {
    const char *name;
    const FimoModuleInfo *info;
    FimoArrayList callbacks;
    struct ModuleHandle_ *handle;
    const FimoModule *owner;
    enum ModuleLoadStatus_ status;
    const FimoModuleExport *export;
};

static FimoResult loading_set_module_new_(const FimoModuleExport *export, struct ModuleHandle_ *handle,
                                          const FimoModule *owner, struct LoadingSetModule_ *element) {
    FIMO_DEBUG_ASSERT(export && handle && element)
    char *name = NULL;
    FimoResult error = clone_string_(export->name, &name);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    module_handle_acquire_(handle);
    if (owner) {
        const struct ModuleInfo_ *info = module_info_from_module_(element->owner);
        struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
        error = module_info_prevent_unload_(info_inner);
        module_info_unlock_(info_inner);
        if (FIMO_RESULT_IS_ERROR(error)) {
            fimo_free(name);
            return error;
        }
    }

    *element = (struct LoadingSetModule_){
            .name = name,
            .info = NULL,
            .callbacks = fimo_array_list_new(),
            .handle = handle,
            .owner = owner,
            .status = MODULE_LOAD_STATUS_UNLOADED_,
            .export = export,
    };

    return FIMO_EOK;
}

static void loading_set_module_free_(struct LoadingSetModule_ *element) {
    FIMO_DEBUG_ASSERT(element)
    while (!fimo_array_list_is_empty(&element->callbacks)) {
        FIMO_DEBUG_ASSERT(element->status == MODULE_LOAD_STATUS_UNLOADED_ ||
                          element->status == MODULE_LOAD_STATUS_ERROR_)
        struct LoadingSetCallback_ callback;
        FimoResult error = fimo_array_list_pop_back(&element->callbacks, sizeof(callback), &callback, NULL);
        FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        callback.error(element->export, callback.data);
    }
    fimo_free((char *)element->name);
    fimo_array_list_free(&element->callbacks, sizeof(struct LoadingSetCallback_), _Alignof(struct LoadingSetCallback_),
                         NULL);

    if (element->status != MODULE_LOAD_STATUS_LOADED_) {
        fi_module_export_cleanup_(element->export);
    }

    if (element->owner) {
        const struct ModuleInfo_ *info = module_info_from_module_(element->owner);
        struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
        module_info_allow_unload_(info_inner);
        module_info_unlock_(info_inner);
    }
    module_handle_release_(element->handle);

    element->name = NULL;
    element->handle = NULL;
    element->export = NULL;
}

static FimoResult loading_set_module_append_callback_(struct LoadingSetModule_ *element,
                                                      struct LoadingSetCallback_ callback) {
    FIMO_DEBUG_ASSERT(element && callback.success && callback.error)
    switch (element->status) {
        case MODULE_LOAD_STATUS_UNLOADED_:
            return fimo_array_list_push(&element->callbacks, sizeof(callback), _Alignof(struct LoadingSetModule_),
                                        &callback, NULL);
        case MODULE_LOAD_STATUS_LOADED_: {
            FIMO_DEBUG_ASSERT(element->info);
            callback.success(element->info, callback.data);
            return FIMO_EOK;
        }
        case MODULE_LOAD_STATUS_ERROR_: {
            FIMO_DEBUG_ASSERT_FALSE(element->info)
            callback.error(element->export, callback.data);
            return FIMO_EOK;
        }
    }

    FIMO_ASSERT(false)
}

static void loading_set_module_signal_error_(struct LoadingSetModule_ *element) {
    FIMO_DEBUG_ASSERT(element)
    element->status = MODULE_LOAD_STATUS_ERROR_;
    while (!fimo_array_list_is_empty(&element->callbacks)) {
        struct LoadingSetCallback_ callback;
        FimoResult error = fimo_array_list_pop_back(&element->callbacks, sizeof(callback), &callback, NULL);
        FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        callback.error(element->export, callback.data);
    }
}

static void loading_set_module_signal_success(struct LoadingSetModule_ *element, const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(element && info)
    element->status = MODULE_LOAD_STATUS_LOADED_;
    element->info = info;
    while (!fimo_array_list_is_empty(&element->callbacks)) {
        struct LoadingSetCallback_ callback;
        FimoResult error = fimo_array_list_pop_back(&element->callbacks, sizeof(callback), &callback, NULL);
        FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        callback.success(info, callback.data);
    }
}

static uint64_t loading_set_module_hash_(const struct LoadingSetModule_ *item, const uint64_t seed0,
                                         const uint64_t seed1) {
    FimoU64 name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
}

static int loading_set_module_cmp_(const struct LoadingSetModule_ *a, const struct LoadingSetModule_ *b,
                                   const void *udata) {
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Loading Set Symbol
///////////////////////////////////////////////////////////////////////

struct LoadingSetSymbol_ {
    const char *name;
    const char *ns;
    FimoVersion version;
    const char *module;
};

static FimoResult loading_set_symbol_new_(const char *name, const char *ns, const FimoVersion version,
                                          const char *module, struct LoadingSetSymbol_ *element) {
    FIMO_DEBUG_ASSERT(name && ns && module && element)
    char *name_ = NULL;
    FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto alloc_name;
    }

    char *ns_ = NULL;
    error = clone_string_(ns, &ns_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto alloc_ns;
    }

    char *module_ = NULL;
    error = clone_string_(module, &module_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto alloc_module;
    }

    *element = (struct LoadingSetSymbol_){
            .name = name_,
            .ns = ns_,
            .version = version,
            .module = module_,
    };

    return FIMO_EOK;

alloc_module:
    fimo_free(ns_);
alloc_ns:
    fimo_free(name_);
alloc_name:
    return error;
}

static void loading_set_symbol_free_(struct LoadingSetSymbol_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    fimo_free((char *)element->ns);
    fimo_free((char *)element->module);

    element->name = NULL;
    element->ns = NULL;
    element->module = NULL;
}

static uint64_t loading_set_symbol_hash_(const struct LoadingSetSymbol_ *item, const uint64_t seed0,
                                         const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    FimoUSize ns_len = strlen(item->ns);

    uint64_t name_hash = hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
    uint64_t ns_hash = hashmap_xxhash3(item->ns, ns_len * sizeof(char), seed0, seed1);
    return combine_hashes_(name_hash, ns_hash);
}

static int loading_set_symbol_cmp_(const struct LoadingSetSymbol_ *a, const struct LoadingSetSymbol_ *b,
                                   const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    int comp = strcmp(a->name, b->name);
    if (comp != 0) {
        return comp;
    }
    return strcmp(a->ns, b->ns);
}


///////////////////////////////////////////////////////////////////////
//// Loading Set Loading Info
///////////////////////////////////////////////////////////////////////

struct LoadingSetLoadingInfo_ {
    FimoArrayList load_list;
};

static void loading_set_loading_info_new_(struct LoadingSetLoadingInfo_ *element) {
    FIMO_DEBUG_ASSERT(element)
    *element = (struct LoadingSetLoadingInfo_){
            .load_list = fimo_array_list_new(),
    };
}

static void loading_set_loading_info_free_(struct LoadingSetLoadingInfo_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_array_list_free(&element->load_list, sizeof(struct LoadingSetModule_ *), _Alignof(struct LoadingSetModule_ *),
                         NULL);
}

static FimoResult loading_set_loading_info_push_(struct LoadingSetLoadingInfo_ *element,
                                                 struct LoadingSetModule_ *module) {
    FIMO_DEBUG_ASSERT(element && module)
    return fimo_array_list_push(&element->load_list, sizeof(struct LoadingSetModule_ *),
                                _Alignof(struct LoadingSetModule_ *), &module, NULL);
}

static struct LoadingSetModule_ *loading_set_loading_info_pop_(struct LoadingSetLoadingInfo_ *element) {
    FIMO_DEBUG_ASSERT(element)
    struct LoadingSetModule_ *module;
    const FimoResult error = fimo_array_list_pop_back(&element->load_list, sizeof(module), &module, NULL);
    FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
    (void)error;
    return module;
}

static bool loading_set_loading_info_is_empty_(struct LoadingSetLoadingInfo_ *element) {
    FIMO_DEBUG_ASSERT(element)
    return fimo_array_list_is_empty(&element->load_list);
}

struct LoadingSetLoadingInfoEntry_ {
    const char *name;
    FimoU64 node;
};

static FimoResult loading_set_loading_info_entry_new_(const char *name, FimoU64 node,
                                                      struct LoadingSetLoadingInfoEntry_ *element) {
    FIMO_DEBUG_ASSERT(name && element)
    char *name_ = NULL;
    FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *element = (struct LoadingSetLoadingInfoEntry_){
            .name = name_,
            .node = node,
    };

    return FIMO_EOK;
}

static void loading_set_loading_info_entry_free_(struct LoadingSetLoadingInfoEntry_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    element->name = NULL;
}


static uint64_t loading_set_loading_info_entry_hash_(const struct LoadingSetLoadingInfoEntry_ *item,
                                                     const uint64_t seed0, const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, name_len * sizeof(char), seed0, seed1);
}

static int loading_set_loading_info_entry_cmp_(const struct LoadingSetLoadingInfoEntry_ *a,
                                               const struct LoadingSetLoadingInfoEntry_ *b, const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Loading Set
///////////////////////////////////////////////////////////////////////

// Heap only.
struct FimoModuleLoadingSet {
    bool is_loading;
    bool should_recreate_map;
    struct hashmap *modules;
    struct hashmap *symbols;
    mtx_t mutex;
};

static FimoResult loading_set_new_(FimoModuleLoadingSet **set) {
    FIMO_DEBUG_ASSERT(set)
    struct hashmap *modules = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct LoadingSetModule_), 0, 0, 0, (HashFn_)loading_set_module_hash_,
            (CmpFn_)loading_set_module_cmp_, (FreeFn_)loading_set_module_free_, NULL);
    if (modules == NULL) {
        return ERR_MOD_MAP_ALLOC_;
    }

    struct hashmap *symbols = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct LoadingSetSymbol_), 0, 0, 0, (HashFn_)loading_set_symbol_hash_,
            (CmpFn_)loading_set_symbol_cmp_, (FreeFn_)loading_set_symbol_free_, NULL);
    if (symbols == NULL) {
        hashmap_free(modules);
        return ERR_SYM_MAP_ALLOC_;
    }

    FimoResult error;
    *set = fimo_malloc(sizeof(**set), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        hashmap_free(symbols);
        hashmap_free(modules);
        return error;
    }

    **set = (FimoModuleLoadingSet){
            .is_loading = false,
            .should_recreate_map = false,
            .modules = modules,
            .symbols = symbols,
    };

    if (mtx_init(&(*set)->mutex, mtx_plain) != thrd_success) {
        fimo_free(*set);
        hashmap_free(symbols);
        hashmap_free(modules);
        return ERR_MUTEX_INIT_;
    }

    return FIMO_EOK;
}

static void loading_set_free_(FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(set)
    FIMO_DEBUG_ASSERT(!set->is_loading)
    hashmap_free(set->symbols);
    hashmap_free(set->modules);
    mtx_destroy(&set->mutex);
    fimo_free(set);
}

static void loading_set_lock_(FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(set)
    const int result = mtx_lock(&set->mutex);
    FIMO_ASSERT(result == thrd_success)
}

static void loading_set_unlock_(FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(set)
    const int result = mtx_unlock(&set->mutex);
    FIMO_ASSERT(result == thrd_success)
}

static const struct LoadingSetModule_ *loading_set_get_module_(FimoModuleLoadingSet *set, const char *name) {
    FIMO_DEBUG_ASSERT(set && name)
    return hashmap_get(set->modules, &(struct LoadingSetModule_){
                                             .name = name,
                                     });
}

static const struct LoadingSetSymbol_ *loading_set_get_symbol_(FimoModuleLoadingSet *set, const char *name,
                                                               const char *ns, const FimoVersion version) {
    FIMO_DEBUG_ASSERT(set && name && ns)
    const struct LoadingSetSymbol_ *sym = hashmap_get(set->symbols, &(struct LoadingSetSymbol_){
                                                                            .name = name,
                                                                            .ns = ns,
                                                                    });
    if (sym == NULL || !fimo_version_compatible(&sym->version, &version)) {
        return NULL;
    }
    return sym;
}

static bool loading_set_next_module_(FimoModuleLoadingSet *set, FimoUSize *it, const struct LoadingSetModule_ **item) {
    FIMO_DEBUG_ASSERT(set)
    return hashmap_iter(set->modules, it, (void *)item);
}

static FimoResult loading_set_create_info_(FimoModuleLoadingSet *set, FimoInternalModuleContext *ctx,
                                           struct LoadingSetLoadingInfo_ *element) {
    FIMO_DEBUG_ASSERT(set && ctx && element)
    FimoGraph *module_graph;
    FimoResult error = fimo_graph_new(sizeof(struct LoadingSetModule_ *), 0, NULL, NULL, &module_graph);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not create module graph")
        return error;
    }

    struct hashmap *modules = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct LoadingSetLoadingInfoEntry_), 0, 0, 0,
            (HashFn_)loading_set_loading_info_entry_hash_, (CmpFn_)loading_set_loading_info_entry_cmp_,
            (FreeFn_)loading_set_loading_info_entry_free_, NULL);
    if (modules == NULL) {
        ERROR_SIMPLE_(ctx, error, "could not create module map")
        goto free_graph;
    }

    // Allocate a node for each loadable module.
    {
        size_t it = 0;
        const struct LoadingSetModule_ *module = NULL;
        while (loading_set_next_module_(set, &it, &module)) {
            FIMO_DEBUG_ASSERT(module)
            if (module->status != MODULE_LOAD_STATUS_UNLOADED_) {
                continue;
            }

            // Check that no other module with the same name is already loaded.
            if (ctx_get_module_(ctx, module->name) != NULL) {
                WARN_(ctx, "module with the same name already exists, module='%s'", module->name);
                goto skip_module;
            }

            // Check that all imported symbols are already exposed, or will be exposed.
            for (FimoISize i = 0; i < (FimoISize)module->export->symbol_imports_count; i++) {
                const FimoModuleSymbolImport *import = &module->export->symbol_imports[i];
                const struct LoadingSetSymbol_ *symbol =
                        loading_set_get_symbol_(set, import->name, import->ns, import->version);
                // Skip the module if a dependency could not be loaded.
                if (symbol != NULL) {
                    const struct LoadingSetModule_ *exporter = loading_set_get_module_(set, symbol->module);
                    FIMO_DEBUG_ASSERT(exporter)
                    if (exporter->status == MODULE_LOAD_STATUS_ERROR_) {
                        WARN_(ctx,
                              "module can not be loaded as there was an error during the construction of a module "
                              "it depends on, module='%s', dependency='%s'",
                              module->name, exporter->name)
                        goto skip_module;
                    }
                }
                else if (ctx_get_symbol_compatible_(ctx, import->name, import->ns, import->version) == NULL) {
                    WARN_(ctx, "module is missing symbol, module='%s'", module->name);
                    goto skip_module;
                }
            }

            // Check that no exported symbols are already exposed.
            for (FimoISize i = 0; i < (FimoISize)module->export->symbol_exports_count; i++) {
                const FimoModuleSymbolExport *export = &module->export->symbol_exports[i];
                if (ctx_get_symbol_(ctx, export->name, export->ns) != NULL) {
                    WARN_(ctx, "module exports duplicate symbol, module='%s', symbol='%s', ns='%s'", module->name,
                          export->name, export->ns)
                    goto skip_module;
                }
            }
            for (FimoISize i = 0; i < (FimoISize)module->export->dynamic_symbol_exports_count; i++) {
                const FimoModuleDynamicSymbolExport *export = &module->export->dynamic_symbol_exports[i];
                if (ctx_get_symbol_(ctx, export->name, export->ns) != NULL) {
                    WARN_(ctx, "module exports duplicate symbol, module='%s', symbol='%s', ns='%s'", module->name,
                          export->name, export->ns)
                    goto skip_module;
                }
            }

            // Create a new node and insert it into the hashmap.
            FimoU64 node;
            error = fimo_graph_add_node(module_graph, &module, &node);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_SIMPLE_(ctx, error, "could not add a node to the module graph")
                goto free_modules;
            }
            struct LoadingSetLoadingInfoEntry_ entry;
            error = loading_set_loading_info_entry_new_(module->name, node, &entry);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_SIMPLE_(ctx, error, "could not initialize hashmap entry")
                goto free_modules;
            }
            hashmap_set(modules, &entry);
            if (hashmap_oom(modules)) {
                error = FIMO_ENOMEM;
                loading_set_loading_info_entry_free_(&entry);
                ERROR_SIMPLE_(ctx, error, "could not insert entry into hashmap")
                goto free_modules;
            }
            continue;

        skip_module:
            loading_set_module_signal_error_((void *)module);
        }
    }

    // Connect all nodes in the graph.
    {
        size_t it = 0;
        const struct LoadingSetLoadingInfoEntry_ *entry = NULL;
        while (hashmap_iter(modules, &it, (void **)&entry)) {
            FIMO_DEBUG_ASSERT(entry)
            FimoU64 src_node = entry->node;
            const struct LoadingSetModule_ *module = loading_set_get_module_(set, entry->name);
            FIMO_DEBUG_ASSERT(module);

            for (FimoISize i = 0; i < (FimoISize)module->export->symbol_imports_count; i++) {
                const FimoModuleSymbolImport *import = &module->export->symbol_imports[i];
                const struct LoadingSetSymbol_ *symbol =
                        loading_set_get_symbol_(set, import->name, import->ns, import->version);
                if (symbol != NULL) {
                    const struct LoadingSetLoadingInfoEntry_ *exported_entry =
                            hashmap_get(modules, &(struct LoadingSetLoadingInfoEntry_){
                                                         .name = symbol->module,
                                                 });
                    if (exported_entry == NULL ||
                        loading_set_get_module_(set, exported_entry->name)->status == MODULE_LOAD_STATUS_ERROR_) {
                        WARN_(ctx,
                              "module can not be loaded as there was an error during the construction of a module "
                              "it depends on, module='%s', dependency='%s'",
                              module->name, symbol->name)
                        goto connect_skip_module;
                    }

                    FimoU64 dst_node = exported_entry->node;
                    FimoU64 edge_;
                    error = fimo_graph_add_edge(module_graph, src_node, dst_node, NULL, NULL, &edge_);
                    if (FIMO_RESULT_IS_ERROR(error)) {
                        ERROR_SIMPLE_(ctx, error, "could not connect module to its dependency in the module graph")
                        goto free_modules;
                    }
                }
            }
            continue;

        connect_skip_module:
            loading_set_module_signal_error_((void *)module);
        }
    }

    bool is_cyclic;
    error = fimo_graph_is_cyclic(module_graph, &is_cyclic);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not determine if the module load graph is cyclic")
        goto free_modules;
    }

    // Find a suitable load order.
    FimoArrayList ordered_nodes;
    error = fimo_graph_topological_sort(module_graph, false, &ordered_nodes);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not compute a topological order for the module graph")
        goto free_modules;
    }

    loading_set_loading_info_new_(element);
    while (!fimo_array_list_is_empty(&ordered_nodes)) {
        FimoU64 node;
        error = fimo_array_list_pop_front(&ordered_nodes, sizeof(node), &node, NULL);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        struct LoadingSetModule_ **module;
        error = fimo_graph_node_data(module_graph, node, (const void **)&module);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        FIMO_DEBUG_ASSERT(module)
        error = loading_set_loading_info_push_(element, *module);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not create load order")
            goto free_load_list;
        }
    }

    fimo_array_list_free(&ordered_nodes, sizeof(FimoU64), _Alignof(FimoU64), NULL);
    hashmap_free(modules);
    fimo_graph_free(module_graph);

    return FIMO_EOK;

free_load_list:
    loading_set_loading_info_free_(element);
    fimo_array_list_free(&ordered_nodes, sizeof(FimoU64), _Alignof(FimoU64), NULL);
free_modules:
    hashmap_free(modules);
free_graph:
    fimo_graph_free(module_graph);

    return error;
}

///////////////////////////////////////////////////////////////////////
//// Module
///////////////////////////////////////////////////////////////////////

struct Module_ {
    const char *name;
    const FimoModule *module;
    FimoU64 node;
};

static FimoResult module_new_(const FimoModule *module, FimoU64 node, struct Module_ *element) {
    FIMO_DEBUG_ASSERT(module && element)
    char *name = NULL;
    FimoResult error = clone_string_(module->module_info->name, &name);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *element = (struct Module_){
            .name = name,
            .module = module,
            .node = node,
    };

    return FIMO_EOK;
}

static void module_free_(struct Module_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    element->name = NULL;
}

static uint64_t module_hash_(const struct Module_ *item, uint64_t seed0, uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, sizeof(char) * name_len, seed0, seed1);
}

static int module_cmp_(const struct Module_ *a, const struct Module_ *b, void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Symbol
///////////////////////////////////////////////////////////////////////

struct Symbol_ {
    const char *name;
    const char *ns;
    FimoVersion version;
    const char *module;
};

static FimoResult symbol_new_(const char *name, const char *ns, const FimoVersion version, const char *module,
                              struct Symbol_ *element) {
    FIMO_DEBUG_ASSERT(name && ns && module && element)
    char *name_;
    FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    char *ns_;
    error = clone_string_(ns, &ns_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto free_name;
    }

    char *module_;
    error = clone_string_(module, &module_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        goto free_ns;
    }

    *element = (struct Symbol_){
            .name = name_,
            .ns = ns_,
            .version = version,
            .module = module_,
    };

    return FIMO_EOK;

free_ns:
    fimo_free(ns_);
free_name:
    fimo_free(name_);

    return error;
}

static void symbol_free_(struct Symbol_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->module);
    fimo_free((char *)element->ns);
    fimo_free((char *)element->name);

    element->module = NULL;
    element->ns = NULL;
    element->name = NULL;
}

static uint64_t symbol_hash_(const struct Symbol_ *item, const uint64_t seed0, const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    FimoUSize ns_len = strlen(item->ns);

    uint64_t name_hash = hashmap_xxhash3(item->name, sizeof(char) * name_len, seed0, seed1);
    uint64_t ns_hash = hashmap_xxhash3(item->ns, sizeof(char) * ns_len, seed0, seed1);

    return combine_hashes_(name_hash, ns_hash);
}

static int symbol_cmp_(const struct Symbol_ *a, const struct Symbol_ *b, const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    int cmp = strcmp(a->name, b->name);
    if (cmp == 0) {
        cmp = strcmp(a->ns, b->ns);
    }
    return cmp;
}

///////////////////////////////////////////////////////////////////////
//// Namespace
///////////////////////////////////////////////////////////////////////

struct Namespace_ {
    const char *name;
    FimoUSize symbol_count;
    FimoUSize reference_count;
};

static FimoResult namespace_new_(const char *name, struct Namespace_ *element) {
    FIMO_DEBUG_ASSERT(name && element)
    char *name_ = NULL;
    FimoResult error = clone_string_(name, &name_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *element = (struct Namespace_){
            .name = name_,
            .symbol_count = 0,
            .reference_count = 0,
    };

    return FIMO_EOK;
}

static void namespace_free_(struct Namespace_ *element) {
    FIMO_DEBUG_ASSERT(element)
    fimo_free((char *)element->name);
    element->name = NULL;
}

static uint64_t namespace_hash_(const struct Namespace_ *item, const uint64_t seed0, const uint64_t seed1) {
    FIMO_DEBUG_ASSERT(item)
    FimoUSize name_len = strlen(item->name);
    return hashmap_xxhash3(item->name, sizeof(char) * name_len, seed0, seed1);
}

static int namespace_cmp_(const struct Module_ *a, const struct Module_ *b, const void *udata) {
    FIMO_DEBUG_ASSERT(a && b)
    (void)udata;
    return strcmp(a->name, b->name);
}

///////////////////////////////////////////////////////////////////////
//// Context
///////////////////////////////////////////////////////////////////////

static FimoResult ctx_init_(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "initializing the module context")
    const int result = mtx_init(&ctx->mutex, mtx_plain);
    if (result == thrd_error) {
        FimoResult error = ERR_MUTEX_INIT_;
        ERROR_SIMPLE_(ctx, error, "could not initialize mutex")
        return error;
    }

    FimoResult error;
    ctx->symbols = hashmap_new_with_allocator(malloc_, realloc_, free_, sizeof(struct Symbol_), 0, 0, 0,
                                              (HashFn_)symbol_hash_, (CmpFn_)symbol_cmp_, (FreeFn_)symbol_free_, NULL);
    if (ctx->symbols == NULL) {
        error = ERR_SYM_MAP_ALLOC_;
        ERROR_SIMPLE_(ctx, error, "could not initialize symbols map")
        goto deinit_mtx;
    }

    ctx->modules = hashmap_new_with_allocator(malloc_, realloc_, free_, sizeof(struct Module_), 0, 0, 0,
                                              (HashFn_)module_hash_, (CmpFn_)module_cmp_, (FreeFn_)module_free_, NULL);
    if (ctx->modules == NULL) {
        error = ERR_MOD_MAP_ALLOC_;
        ERROR_SIMPLE_(ctx, error, "could not initialize modules map")
        goto deinit_symbols;
    }

    ctx->namespaces = hashmap_new_with_allocator(malloc_, realloc_, free_, sizeof(struct Namespace_), 0, 0, 0,
                                                 (HashFn_)namespace_hash_, (CmpFn_)namespace_cmp_,
                                                 (FreeFn_)namespace_free_, NULL);
    if (ctx->namespaces == NULL) {
        error = ERR_NS_MAP_ALLOC_;
        ERROR_SIMPLE_(ctx, error, "could not initialize namespaces map")
        goto deinit_modules;
    }

    error = fimo_graph_new(sizeof(const FimoModule *), 0, NULL, NULL, &ctx->dependency_graph);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not initialize dependency graph")
        goto deinit_namespaces;
    }

    ctx->is_loading = false;

    return FIMO_EOK;

deinit_namespaces:
    hashmap_free(ctx->namespaces);
    ctx->namespaces = NULL;
deinit_modules:
    hashmap_free(ctx->modules);
    ctx->modules = NULL;
deinit_symbols:
    hashmap_free(ctx->symbols);
    ctx->symbols = NULL;
deinit_mtx:
    mtx_destroy(&ctx->mutex);
    return error;
}

static void ctx_deinit_(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "deinitializing the module context")

    // Since the context is being destroyed there must be no one holding a reference
    // to the main context. As each module implicitly holds a reference to the context,
    // this must also mean that no modules are loaded.
    FIMO_ASSERT(hashmap_count(ctx->symbols) == 0);
    FIMO_ASSERT(hashmap_count(ctx->modules) == 0);
    FIMO_ASSERT(hashmap_count(ctx->namespaces) == 0);
    FIMO_ASSERT(fimo_graph_node_count(ctx->dependency_graph) == 0);
    FIMO_ASSERT_FALSE(ctx->is_loading);

    fimo_graph_free(ctx->dependency_graph);
    hashmap_free(ctx->namespaces);
    hashmap_free(ctx->modules);
    hashmap_free(ctx->symbols);
    mtx_destroy(&ctx->mutex);
}

static FimoResult ctx_lock_(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "")
    if (mtx_lock(&ctx->mutex) == thrd_error) {
        FimoResult error = ERR_MUTEX_LOCK_;
        ERROR_SIMPLE_(ctx, error, "could not lock the context")
        return error;
    }
    return FIMO_EOK;
}

static FimoResult ctx_unlock_(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "")
    if (mtx_unlock(&ctx->mutex) == thrd_error) {
        FimoResult error = ERR_MUTEX_UNLOCK_;
        ERROR_SIMPLE_(ctx, error, "could not unlock the context")
        return error;
    }
    return FIMO_EOK;
}

static FimoResult ctx_add_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner) {
    FIMO_DEBUG_ASSERT(ctx && info_inner && !module_info_is_detached_(info_inner))
    const struct ModuleInfo_ *info = module_info_from_inner_(info_inner);

    TRACE_(ctx, "module='%s'", info->info.name)
    if (hashmap_get(ctx->modules, &(struct Module_){.name = info->info.name}) != NULL) {
        FimoResult error = ERR_DUPLICATE_MOD_;
        ERROR_(ctx, error, "module already exists, module='%s'", info->info.name)
        return error;
    }

    FimoU64 node;
    FimoResult error = fimo_graph_add_node(ctx->dependency_graph, &info_inner->module, &node);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not add the module to the dependency graph")
        return error;
    }

    // Check for no duplicate symbols.
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol = NULL;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            const struct Symbol_ *s = ctx_get_symbol_(ctx, symbol->name, symbol->ns);
            if (s) {
                error = ERR_DUPLICATE_SYM_;
                ERROR_(ctx, error, "symbol already exists, module='%s', symbol='%s', ns='%s'", info->info.name,
                       symbol->name, symbol->ns)
                goto remove_node;
            }
        }
    }
    // Check that all imported namespaces exist.
    {
        FimoUSize it = 0;
        const struct ModuleInfoNamespace_ *ns = NULL;
        while (module_info_next_ns_(info_inner, &it, &ns)) {
            const struct Namespace_ *n = ctx_get_ns_(ctx, ns->name);
            if (n == NULL) {
                error = ERR_MISSING_NS_;
                ERROR_(ctx, error, "namespace does not exist, module='%s', ns='%s'", info->info.name, ns->name)
                goto remove_node;
            }
        }
    }
    // Acquire all imported namespaces.
    {
        FimoUSize it = 0;
        const struct ModuleInfoNamespace_ *ns = NULL;
        while (module_info_next_ns_(info_inner, &it, &ns)) {
            error = ctx_ns_acquire_(ctx, ns->name);
            FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        }
    }
    // Check that all dependencies are met and correct.
    {
        FimoUSize it = 0;
        const struct ModuleInfoDependency_ *dependency = NULL;
        while (module_info_next_dependency_(info_inner, &it, &dependency)) {
            const struct Module_ *dep_mod = ctx_get_module_(ctx, dependency->name);
            if (dep_mod == NULL) {
                error = ERR_MISSING_MOD_;
                ERROR_(ctx, error, "dependency not found, module='%s', dependency='%s'", info->info.name,
                       dependency->name)
                goto release_namespaces;
            }
            FIMO_ASSERT(dependency->info == dep_mod->module->module_info)
            FimoU64 edge;
            error = fimo_graph_add_edge(ctx->dependency_graph, node, dep_mod->node, NULL, NULL, &edge);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_SIMPLE_(ctx, error, "could not add edge to the dependency graph")
                goto release_namespaces;
            }
        }
    }

    // Check the modifiers.
    if (info_inner->export) {
        for (FimoISize i = 0; i < (FimoISize)info_inner->export->modifiers_count; i++) {
            const FimoModuleExportModifier *modifier = &info_inner->export->modifiers[i];
            if (modifier->key != FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY) {
                continue;
            }
            const FimoModuleInfo *dependency = modifier->value;
            FIMO_DEBUG_ASSERT(dependency && dependency->name);
            const struct Module_ *dep_mod = ctx_get_module_(ctx, dependency->name);
            if (dep_mod == NULL) {
                error = ERR_MISSING_MOD_;
                ERROR_(ctx, error, "dependency not found, module='%s', dependency='%s'", info->info.name,
                       dependency->name)
                goto release_namespaces;
            }
            FIMO_ASSERT(dependency == dep_mod->module->module_info);
            FimoU64 edge;
            error = fimo_graph_add_edge(ctx->dependency_graph, node, dep_mod->node, NULL, NULL, &edge);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_SIMPLE_(ctx, error, "could not add edge to the dependency graph")
                goto release_namespaces;
            }
        }
    }

    // Check that the dependency graph is cycle free
    bool is_cyclic;
    error = fimo_graph_is_cyclic(ctx->dependency_graph, &is_cyclic);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not determine if the dependency graph is cycle free")
        goto release_namespaces;
    }
    if (is_cyclic) {
        error = ERR_CYCLIC_DEPENDENCY_;
        ERROR_SIMPLE_(ctx, error, "adding the module would introduce a cyclic dependency")
        goto release_namespaces;
    }

    // Allocate all exported namespaces.
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol = NULL;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            error = ctx_ns_allocate_if_not_found_(ctx, symbol->ns);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_(ctx, error, "failed to allocate ns, module='%s', ns='%s'", info->info.name, symbol->ns)
                goto remove_allocated_ns;
            }
        }
    }
    // Export all symbols.
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol = NULL;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            error = ctx_insert_symbol_(ctx, symbol->name, symbol->ns, symbol->version, info->info.name);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_(ctx, error, "failed to allocate export symbol, module='%s', symbol='%s', ns='%s'",
                       info->info.name, symbol->name, symbol->ns)
                goto remove_symbol_export;
            }
        }
    }
    // Insert the module.
    {
        struct Module_ module_;
        error = module_new_(info_inner->module, node, &module_);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_(ctx, error, "failed to allocate module, module='%s'", info->info.name)
            goto remove_symbol_export;
        }

        hashmap_set(ctx->modules, &module_);
        if (hashmap_oom(ctx->modules)) {
            module_free_(&module_);
            error = FIMO_ENOMEM;
            ERROR_(ctx, error, "failed to insert module, module='%s'", info->info.name)
            goto remove_symbol_export;
        }
    }

    return FIMO_EOK;

remove_symbol_export: {
    FimoUSize it = 0;
    const struct ModuleInfoSymbol_ *symbol = NULL;
    while (module_info_next_symbol_(info_inner, &it, &symbol)) {
        if (ctx_get_symbol_(ctx, symbol->name, symbol->ns)) {
            ctx_remove_symbol_(ctx, symbol->name, symbol->ns);
        }
    }
}
remove_allocated_ns: {
    FimoUSize it = 0;
    const struct ModuleInfoSymbol_ *symbol = NULL;
    while (module_info_next_symbol_(info_inner, &it, &symbol)) {
        ctx_ns_free_if_empty_(ctx, symbol->name);
    }
}
release_namespaces: {
    FimoUSize it = 0;
    const struct ModuleInfoNamespace_ *ns = NULL;
    while (module_info_next_ns_(info_inner, &it, &ns)) {
        ctx_ns_release_(ctx, ns->name);
    }
}
remove_node: {
    void *data;
    const FimoResult error_ = fimo_graph_remove_node(ctx->dependency_graph, node, &data);
    FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error_))
    FIMO_ASSERT(data)
}

    return error;
}

static FimoResult ctx_remove_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner) {
    FIMO_DEBUG_ASSERT(ctx && info_inner && !module_info_is_detached_(info_inner))
    const struct ModuleInfo_ *info = module_info_from_inner_(info_inner);

    TRACE_(ctx, "module='%s'", info->info.name)
    if (!ctx_can_remove_module_(ctx, info_inner)) {
        ERROR_(ctx, FIMO_EPERM, "the module can not be removed, module='%s'", info->info.name)
        return FIMO_EPERM;
    }

    const struct Module_ *module_ = hashmap_get(ctx->modules, &(struct Module_){
                                                                      .name = info->info.name,
                                                              });
    if (module_ == NULL || module_->module != info_inner->module) {
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "module is not registered with the backend, module='%s'", info->info.name)
        return error;
    }

    FimoUSize count;
    const FimoU64 node = module_->node;
    FimoResult error = fimo_graph_neighbors_count(ctx->dependency_graph, node, true, &count);
    FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
    if (count != 0) {
        error = ERR_MOD_IN_USE_;
        ERROR_(ctx, error, "module is still in use, module='%s'", info->info.name)
        return error;
    }

    // Remove all symbols.
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol = NULL;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            ctx_remove_symbol_(ctx, symbol->name, symbol->ns);
        }
    }
    // Release all namespaces.
    {
        FimoUSize it = 0;
        const struct ModuleInfoNamespace_ *ns = NULL;
        while (module_info_next_ns_(info_inner, &it, &ns)) {
            ctx_ns_release_(ctx, ns->name);
        }
    }
    // Check that no empty namespace is still referenced.
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol = NULL;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            if (strcmp(GLOBAL_NS, symbol->ns) == 0) {
                continue;
            }
            const struct Namespace_ *ns = ctx_get_ns_(ctx, symbol->ns);
            if (ns && ns->reference_count != 0 && ns->symbol_count == 0) {
                error = ERR_NS_IN_USE_;
                ERROR_(ctx, error, "namespace is still in use, module='%s', ns='%s'", info->info.name, ns->name)
                goto rollback_ns;
            }
        }
    }

    module_ = hashmap_delete(ctx->modules, &(struct Module_){
                                                   .name = info->info.name,
                                           });
    FIMO_ASSERT(module_)
    module_free_((struct Module_ *)module_);

    void *data_;
    error = fimo_graph_remove_node(ctx->dependency_graph, node, &data_);
    FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
    FIMO_ASSERT(data_)

    return FIMO_EOK;

rollback_ns:;
    {
        FimoUSize it = 0;
        const struct ModuleInfoNamespace_ *ns = NULL;
        while (module_info_next_ns_(info_inner, &it, &ns)) {
            const FimoResult error_ = ctx_ns_acquire_(ctx, ns->name);
            FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error_))
        }
    }
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol = NULL;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            const FimoResult error_ =
                    ctx_insert_symbol_(ctx, symbol->name, symbol->ns, symbol->version, info->info.name);
            FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error_))
        }
    }

    return error;
}

static FimoResult ctx_link_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner,
                                   struct ModuleInfoInner_ *other_inner) {
    FIMO_DEBUG_ASSERT(ctx && info_inner && other_inner && !module_info_is_detached_(info_inner))
    const struct ModuleInfo_ *info = module_info_from_inner_(info_inner);
    const struct ModuleInfo_ *other_info = module_info_from_inner_(other_inner);

    TRACE_(ctx, "module='%s', other='%s'", info->info.name, other_info->info.name)
    if (module_info_is_detached_(other_inner)) {
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "module is not registered with the module subsystem, module='%s'", other_info->info.name)
        return error;
    }
    if (module_info_get_dependency_(info_inner, other_info->info.name) != NULL) {
        FimoResult error = ERR_DUPLICATE_LINK_;
        ERROR_(ctx, error, "modules are already linked, module='%s', other='%s'", info->info.name,
               other_info->info.name)
        return error;
    }
    if (other_info->type == MODULE_TYPE_PSEUDO_) {
        FimoResult error = ERR_IS_PSEUDO_;
        ERROR_(ctx, error, "can not link to a pseudo module, module='%s', other='%s'", info->info.name,
               other_info->info.name)
        return error;
    }

    const struct Module_ *inner_module = ctx_get_module_(ctx, info->info.name);
    const struct Module_ *other_module = ctx_get_module_(ctx, other_info->info.name);
    FIMO_DEBUG_ASSERT(inner_module)
    FIMO_DEBUG_ASSERT(other_module)

    bool would_introduce_cycle;
    FimoResult error = fimo_graph_path_exists(ctx->dependency_graph, other_module->node, inner_module->node,
                                              &would_introduce_cycle);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not determine if linking the modules would introduce a cycle")
        return error;
    }

    FimoU64 edge;
    error = fimo_graph_add_edge(ctx->dependency_graph, inner_module->node, other_module->node, NULL, NULL, &edge);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not add edge to the dependency graph")
        return error;
    }

    error = module_info_set_dependency_(info_inner, &other_info->info, false);
    if (FIMO_RESULT_IS_ERROR(error)) {
        void *edge_data;
        const FimoResult error_ = fimo_graph_remove_edge(ctx->dependency_graph, edge, &edge_data);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error_))
        (void)error_;
        FIMO_DEBUG_ASSERT(edge_data == NULL)
        ERROR_(ctx, error, "could not insert other into the module info dependency map, module='%s', dependency='%s'",
               info->info.name, other_info->info.name)
        return error;
    }

    return FIMO_EOK;
}

static FimoResult ctx_unlink_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner,
                                     struct ModuleInfoInner_ *other_inner) {
    FIMO_DEBUG_ASSERT(ctx && info_inner && other_inner && !module_info_is_detached_(info_inner) &&
                      !module_info_is_detached_(other_inner))
    const struct ModuleInfo_ *info = module_info_from_inner_(info_inner);
    const struct ModuleInfo_ *other_info = module_info_from_inner_(other_inner);

    TRACE_(ctx, "module='%s', other='%s'", info->info.name, other_info->info.name)
    const struct ModuleInfoDependency_ *dependency = module_info_get_dependency_(info_inner, other_info->info.name);
    if (dependency == NULL) {
        FimoResult error = ERR_MISSING_LINK_;
        ERROR_(ctx, error, "modules are not linked, module='%s', other='%s'", info->info.name, other_info->info.name)
        return error;
    }
    if (dependency->is_static) {
        FimoResult error = ERR_STATIC_LINK_;
        ERROR_(ctx, error, "can not unlink static module links, module='%s', other='%s'", info->info.name,
               other_info->info.name)
        return error;
    }

    const struct Module_ *ctx_module = ctx_get_module_(ctx, info->info.name);
    const struct Module_ *other_ctx_module = ctx_get_module_(ctx, other_info->info.name);
    FIMO_DEBUG_ASSERT(ctx_module)
    FIMO_DEBUG_ASSERT(other_ctx_module)

    FimoU64 edge;
    bool contained;
    FimoResult error =
            fimo_graph_find_edge(ctx->dependency_graph, ctx_module->node, other_ctx_module->node, &edge, &contained);
    FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
    FIMO_DEBUG_ASSERT(contained)
    (void)error;

    void *edge_data;
    error = fimo_graph_remove_edge(ctx->dependency_graph, edge, &edge_data);
    FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
    FIMO_DEBUG_ASSERT(edge_data == NULL)
    (void)error;

    module_info_delete_dependency_(info_inner, other_info->info.name);

    return FIMO_EOK;
}

static bool ctx_can_remove_module_(FimoInternalModuleContext *ctx, struct ModuleInfoInner_ *info_inner) {
    FIMO_DEBUG_ASSERT(ctx && info_inner && !module_info_is_detached_(info_inner));
    const struct ModuleInfo_ *info = module_info_from_inner_(info_inner);
    TRACE_(ctx, "module='%s'", info->info.name)

    // Check if the module info has been marked as unloadable
    if (!module_info_can_unload_(info_inner)) {
        return false;
    }

    // Check that no symbols are in use.
    {
        FimoUSize it = 0;
        const struct ModuleInfoSymbol_ *symbol;
        while (module_info_next_symbol_(info_inner, &it, &symbol)) {
            if (FIMO_MODULE_SYMBOL_IS_LOCKED(&symbol->symbol)) {
                return false;
            }
        }
    }

    // Check that there are no dependencies left.
    const struct Module_ *module_ = ctx_get_module_(ctx, info->info.name);
    FIMO_DEBUG_ASSERT(module_)
    FimoUSize neighbors;
    FimoResult error = fimo_graph_neighbors_count(ctx->dependency_graph, module_->node, true, &neighbors);
    FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
    (void)error;

    return neighbors == 0;
}

static FimoResult ctx_cleanup_loose_modules(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "cleaning up loose modules")

    bool has_next;
    FimoGraphExternals *iter;
    FimoResult error = fimo_graph_externals_new(ctx->dependency_graph, false, &iter, &has_next);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not construct externals iterator")
        return error;
    }

    while (has_next) {
        FimoU64 node;
        const FimoModule **module_ptr;
        error = fimo_graph_externals_item(iter, &node, (const void **)&module_ptr);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
        FIMO_DEBUG_ASSERT(module_ptr)
        const FimoModule *module = *module_ptr;
        const struct ModuleInfo_ *info = module_info_from_module_(module);
        if (info->type != MODULE_TYPE_REGULAR_) {
            error = fimo_graph_externals_next(iter, &has_next);
            FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
            continue;
        }
        struct ModuleInfoInner_ *info_inner = module_info_lock_(info);

        if (!ctx_can_remove_module_(ctx, info_inner)) {
            error = fimo_graph_externals_next(iter, &has_next);
            FIMO_DEBUG_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))
            continue;
        }

        error = ctx_remove_module_(ctx, info_inner);
        if (FIMO_RESULT_IS_ERROR(error)) {
            module_info_unlock_(info_inner);
            ERROR_(ctx, error, "could not remove module, module='%s'", module->module_info->name)
            fimo_graph_externals_free(iter);
            return error;
        }
        fi_module_free_(info_inner, NULL);

        // Rebuild the iterator since we modified the dependency graph
        fimo_graph_externals_free(iter);
        error = fimo_graph_externals_new(ctx->dependency_graph, false, &iter, &has_next);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not construct externals iterator")
            return error;
        }
    }
    fimo_graph_externals_free(iter);

    return FIMO_EOK;
}

static const struct Module_ *ctx_get_module_(FimoInternalModuleContext *ctx, const char *name) {
    FIMO_DEBUG_ASSERT(ctx && name)
    TRACE_(ctx, "name='%s'", name)
    return hashmap_get(ctx->modules, &(struct Module_){.name = name});
}

static const struct Symbol_ *ctx_get_symbol_(FimoInternalModuleContext *ctx, const char *name, const char *ns) {
    FIMO_DEBUG_ASSERT(ctx && name && ns)
    TRACE_(ctx, "name='%s', symbol='%s'", name, ns)
    return hashmap_get(ctx->symbols, &(struct Symbol_){.name = name, .ns = ns});
}

static const struct Symbol_ *ctx_get_symbol_compatible_(FimoInternalModuleContext *ctx, const char *name,
                                                        const char *ns, const FimoVersion version) {
    FIMO_DEBUG_ASSERT(ctx && name && ns)
    TRACE_(ctx, "name='%s', symbol='%s'", name, ns)
    const struct Symbol_ *sym = ctx_get_symbol_(ctx, name, ns);
    if (sym == NULL || !fimo_version_compatible(&sym->version, &version)) {
        return NULL;
    }
    return sym;
}

static const struct Namespace_ *ctx_get_ns_(FimoInternalModuleContext *ctx, const char *name) {
    FIMO_DEBUG_ASSERT(ctx && name)
    TRACE_(ctx, "name='%s'", name)
    return hashmap_get(ctx->namespaces, &(struct Namespace_){.name = name});
}

static FimoResult ctx_ns_allocate_if_not_found_(FimoInternalModuleContext *ctx, const char *name) {
    FIMO_DEBUG_ASSERT(ctx && name)
    TRACE_(ctx, "name='%s'", name)
    if (strcmp(name, GLOBAL_NS) == 0) {
        return FIMO_EOK;
    }

    if (ctx_get_ns_(ctx, name)) {
        return FIMO_EOK;
    }

    struct Namespace_ ns;
    const FimoResult error = namespace_new_(name, &ns);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not create namespace, ns='%s'", name)
        return error;
    }

    hashmap_set(ctx->namespaces, &ns);
    if (hashmap_oom(ctx->namespaces)) {
        namespace_free_(&ns);
        ERROR_(ctx, FIMO_ENOMEM, "could not insert namespace into the context, ns='%s'", name)
        return FIMO_ENOMEM;
    }

    return FIMO_EOK;
}

static void ctx_ns_free_if_empty_(FimoInternalModuleContext *ctx, const char *name) {
    FIMO_DEBUG_ASSERT(ctx && name)
    TRACE_(ctx, "name='%s'", name)
    if (strcmp(name, GLOBAL_NS) == 0) {
        return;
    }

    const struct Namespace_ *ns = ctx_get_ns_(ctx, name);
    FIMO_DEBUG_ASSERT(ns)
    if (ns->reference_count == 0 && ns->symbol_count == 0) {
        ns = hashmap_delete(ctx->namespaces, ns);
        namespace_free_((struct Namespace_ *)ns);
    }
}

static FimoResult ctx_ns_acquire_(FimoInternalModuleContext *ctx, const char *name) {
    FIMO_DEBUG_ASSERT(ctx && name)
    TRACE_(ctx, "name='%s'", name)
    if (strcmp(name, GLOBAL_NS) == 0) {
        return FIMO_EOK;
    }

    struct Namespace_ *ns = (struct Namespace_ *)ctx_get_ns_(ctx, name);
    if (ns == NULL) {
        FimoResult error = ERR_MISSING_NS_;
        ERROR_(ctx, error, "namespace not found, ns='%s'", name)
        return error;
    }

    const FimoIntOptionUSize x = fimo_usize_checked_add(ns->reference_count, 1);
    if (!x.has_value) {
        ERROR_(ctx, FIMO_ERANGE, "namespace reference count overflow, ns='%s'", name)
        return FIMO_ERANGE;
    }

    ns->reference_count = x.data.value;
    return FIMO_EOK;
}

static void ctx_ns_release_(FimoInternalModuleContext *ctx, const char *name) {
    FIMO_DEBUG_ASSERT(ctx && name)
    TRACE_(ctx, "name='%s'", name)
    if (strcmp(name, GLOBAL_NS) == 0) {
        return;
    }

    struct Namespace_ *ns = (struct Namespace_ *)ctx_get_ns_(ctx, name);
    FIMO_DEBUG_ASSERT(ns)
    FIMO_DEBUG_ASSERT(ns->reference_count != 0)
    ns->reference_count--;

    ctx_ns_free_if_empty_(ctx, name);
}

static FimoResult ctx_insert_symbol_(FimoInternalModuleContext *ctx, const char *name, const char *ns,
                                     const FimoVersion version, const char *module) {
    FIMO_DEBUG_ASSERT(ctx && name && ns && module)
    TRACE_(ctx, "name='%s', ns='%s', module='%s'", name, ns, module)
    if (ctx_get_symbol_(ctx, name, ns)) {
        FimoResult error = ERR_DUPLICATE_SYM_;
        ERROR_(ctx, error, "symbol already exists, name='%s', ns='%s'", name, ns)
        return error;
    }

    struct Symbol_ sym;
    FimoResult error = symbol_new_(name, ns, version, module, &sym);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not create new symbol")
        return error;
    }

    hashmap_set(ctx->symbols, &sym);
    if (hashmap_oom(ctx->symbols)) {
        symbol_free_(&sym);
        ERROR_SIMPLE_(ctx, FIMO_ENOMEM, "could not insert symbol into the context")
        return FIMO_ENOMEM;
    }

    if (strcmp(ns, GLOBAL_NS) != 0) {
        struct Namespace_ *ns_ = (struct Namespace_ *)ctx_get_ns_(ctx, ns);
        if (ns_ == NULL) {
            error = ERR_MISSING_NS_;
            ERROR_(ctx, error, "missing namespace, ns='%s'", ns)
            goto remove_symbol;
        }

        const FimoIntOptionUSize x = fimo_usize_checked_add(ns_->symbol_count, 1);
        if (!x.has_value) {
            error = FIMO_ERANGE;
            ERROR_(ctx, error, "namespace symbol count overflow, ns='%s'", ns)
            goto remove_symbol;
        }

        ns_->symbol_count = x.data.value;
    }

    return FIMO_EOK;

remove_symbol: {
    hashmap_delete(ctx->symbols, &sym);
    symbol_free_(&sym);
}

    return error;
}

static void ctx_remove_symbol_(FimoInternalModuleContext *ctx, const char *name, const char *ns) {
    FIMO_DEBUG_ASSERT(ctx && name && ns)
    TRACE_(ctx, "name='%s', ns='%s'", name, ns)
    struct Symbol_ *sym = (struct Symbol_ *)ctx_get_symbol_(ctx, name, ns);
    FIMO_DEBUG_ASSERT(sym)

    sym = (struct Symbol_ *)hashmap_delete(ctx->symbols, sym);
    symbol_free_(sym);

    if (strcmp(ns, GLOBAL_NS) != 0) {
        struct Namespace_ *ns_ = (struct Namespace_ *)ctx_get_ns_(ctx, ns);
        FIMO_ASSERT(ns_ && ns_->symbol_count != 0)
        ns_->symbol_count--;
        ctx_ns_free_if_empty_(ctx, ns);
    }
}

static FimoResult ctx_load_set(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(ctx && set)
    if (ctx->is_loading) {
        FimoResult error = ERR_IS_LOADING_;
        ERROR_SIMPLE_(ctx, error, "a set is already being loaded")
        return error;
    }
    FIMO_DEBUG_ASSERT(!set->is_loading)
    ctx->is_loading = true;
    set->is_loading = true;

    struct LoadingSetLoadingInfo_ loading_info;
    set->should_recreate_map = false;
    FimoResult error = loading_set_create_info_(set, ctx, &loading_info);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not construct load order")
        goto on_critical_error;
    }

    while (!loading_set_loading_info_is_empty_(&loading_info)) {
        if (set->should_recreate_map) {
            set->should_recreate_map = false;
            loading_set_loading_info_free_(&loading_info);
            error = loading_set_create_info_(set, ctx, &loading_info);
            if (FIMO_RESULT_IS_ERROR(error)) {
                ERROR_SIMPLE_(ctx, error, "could not construct load order")
                goto on_critical_error;
            }
        }

        struct LoadingSetModule_ *module = loading_set_loading_info_pop_(&loading_info);

        // Recheck that all dependencies could be loaded.
        for (FimoISize i = 0; i < (FimoISize)module->export->symbol_imports_count; i++) {
            const FimoModuleSymbolImport *import = &module->export->symbol_imports[i];
            const struct LoadingSetSymbol_ *symbol =
                    loading_set_get_symbol_(set, import->name, import->ns, import->version);
            // Skip the module if a dependency could not be loaded.
            if (symbol != NULL) {
                const struct LoadingSetModule_ *exporter = loading_set_get_module_(set, symbol->module);
                FIMO_DEBUG_ASSERT(exporter)
                if (exporter->status == MODULE_LOAD_STATUS_ERROR_) {
                    WARN_(ctx,
                          "module can not be loaded as there was an error during the construction of a module "
                          "it depends on, module='%s', dependency='%s'",
                          module->name, exporter->name)
                    goto skip_module;
                }
            }
        }

        // Check that the explicit dependencies exist.
        for (FimoISize i = 0; i < (FimoISize)module->export->modifiers_count; i++) {
            const FimoModuleExportModifier *modifier = &module->export->modifiers[i];
            if (modifier->key != FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY) {
                continue;
            }
            const FimoModuleInfo *dependency = modifier->value;
            FIMO_DEBUG_ASSERT(dependency && dependency->name)
            if (ctx_get_module_(ctx, dependency->name) == NULL) {
                WARN_(ctx,
                      "module can not be loaded as the module specified by the dependency "
                      "modifier does not exist, module='%s', dependency='%s'",
                      module->name, dependency->name)
                goto skip_module;
            }
        }

        // Construct the module.
        FimoModule *constructed;
        error = fi_module_new_from_export(ctx, set, module->export, module->handle, &constructed);
        if (FIMO_RESULT_IS_ERROR(error)) {
            FimoResultString error_name = fimo_result_error_name(error);
            FimoResultString error_description = fimo_result_error_description(error);
            WARN_(ctx, "skipping module due to construction error, module='%s', error='%s:%s'", module->name,
                  error_name.str, error_description.str)
            fimo_result_string_release(error_name);
            fimo_result_string_release(error_description);
            goto skip_module;
        }

        // Register it with the backend.
        const struct ModuleInfo_ *constructed_info = module_info_from_module_(constructed);
        struct ModuleInfoInner_ *constructed_info_inner = module_info_lock_(constructed_info);
        error = ctx_add_module_(ctx, constructed_info_inner);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not register module with the backend")
            fi_module_free_(constructed_info_inner, NULL);
            goto free_loding_info;
        }
        module_info_unlock_(constructed_info_inner);

        // Signal loading success.
        loading_set_module_signal_success(module, constructed->module_info);
        continue;

    skip_module:
        loading_set_module_signal_error_(module);
    }

    set->is_loading = false;
    ctx->is_loading = false;
    return FIMO_EOK;

free_loding_info:
    loading_set_loading_info_free_(&loading_info);
on_critical_error:
    set->is_loading = false;
    ctx->is_loading = false;
    return error;
}

///////////////////////////////////////////////////////////////////////
//// Fimo Module
///////////////////////////////////////////////////////////////////////

static FimoResult fi_module_new_pseudo_(FimoInternalModuleContext *ctx, const char *name, FimoModule **element) {
    FIMO_DEBUG_ASSERT(ctx && name && element)
    TRACE_(ctx, "name='%s', element='%p'", name, (void *)element)
    struct ModuleHandle_ *handle;
    void (*iterator)(bool (*)(const FimoModuleExport *, void *), void *) = fimo_impl_module_export_iterator;
    FimoResult error = module_handle_new_local_(fimo_impl_module_export_iterator, *(const void **)&iterator, &handle);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not construct module handle")
        return error;
    }

    struct ModuleInfo_ *info;
    error = module_info_new_(name, NULL, NULL, NULL, NULL, handle, NULL, MODULE_TYPE_PSEUDO_, &info);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not construct module info")
        module_handle_release_(handle);
        return error;
    }

    fimo_internal_context_acquire(TO_CTX_(ctx));
    FimoContext ctx_;
    error = fimo_internal_context_to_public_ctx(TO_CTX_(ctx), &ctx_);
    FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))

    *element = fimo_malloc(sizeof(**element), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not allocate module")
        goto release_ctx;
    }
    info->inner.module = *element;

    **element = (FimoModule){
            .parameters = NULL,
            .resources = NULL,
            .imports = NULL,
            .exports = NULL,
            .module_info = &info->info,
            .context = ctx_,
            .module_data = NULL,
    };

    return FIMO_EOK;

release_ctx:
    fimo_internal_context_release(TO_CTX_(ctx));
    module_info_release_(info, false);

    return error;
}

static FimoResult fi_module_new_from_export(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                            const FimoModuleExport *export, struct ModuleHandle_ *handle,
                                            FimoModule **element) {
    FIMO_DEBUG_ASSERT(ctx && export && handle && element)
    module_handle_acquire_(handle);
    *element = NULL;

    struct ModuleInfo_ *info;
    FimoResult error = module_info_new_(export->name, export->description, export->author, export->license,
                                        handle->module_path, handle, export, MODULE_TYPE_REGULAR_, &info);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not construct module info")
        module_handle_release_(handle);
        return error;
    }
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);

    fimo_internal_context_acquire(TO_CTX_(ctx));
    FimoContext ctx_;
    error = fimo_internal_context_to_public_ctx(TO_CTX_(ctx), &ctx_);
    FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))

    *element = fimo_malloc(sizeof(**element), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not allocate module")
        goto release_ctx;
    }
    info->inner.module = *element;

    **element = (FimoModule){
            .parameters = NULL,
            .resources = NULL,
            .imports = NULL,
            .exports = NULL,
            .module_info = &info->info,
            .context = ctx_,
            .module_data = NULL,
    };

    // Init parameters.
    FimoArrayList params = fimo_array_list_new();
    for (FimoISize i = 0; i < export->parameters_count; i++) {
        const FimoModuleParamDecl *decl = &export->parameters[i];
        struct ParamData_ param_data = {
                .owner = *element,
                .type = decl->type,
        };
        switch (decl->type) {
            case FIMO_MODULE_PARAM_TYPE_U8:
                param_data.value.u8 = decl->default_value.u8;
                break;
            case FIMO_MODULE_PARAM_TYPE_U16:
                param_data.value.u16 = decl->default_value.u16;
                break;
            case FIMO_MODULE_PARAM_TYPE_U32:
                param_data.value.u32 = decl->default_value.u32;
                break;
            case FIMO_MODULE_PARAM_TYPE_U64:
                param_data.value.u64 = decl->default_value.u64;
                break;
            case FIMO_MODULE_PARAM_TYPE_I8:
                param_data.value.i8 = decl->default_value.i8;
                break;
            case FIMO_MODULE_PARAM_TYPE_I16:
                param_data.value.i16 = decl->default_value.i16;
                break;
            case FIMO_MODULE_PARAM_TYPE_I32:
                param_data.value.i32 = decl->default_value.i32;
                break;
            case FIMO_MODULE_PARAM_TYPE_I64:
                param_data.value.i64 = decl->default_value.i64;
                break;
        }
        FimoModuleParam *param;
        error = param_new_(decl->read_access, decl->write_access, decl->setter, decl->getter, param_data, &param);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not initialize parameter")
            goto release_parameters;
        }
        error = fimo_array_list_push(&params, sizeof(FimoModuleParam *), _Alignof(FimoModuleParam *), &param, NULL);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert parameter into the parameter table")
            goto release_parameters;
        }
        error = module_info_set_param_(info_inner, decl->name, param);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert parameter into the module info")
            goto release_parameters;
        }
    }
    (*element)->parameters = params.elements;

    // Init resources.
    FimoArrayList resources = fimo_array_list_new();
    for (FimoISize i = 0; i < export->resources_count; i++) {
        const FimoModuleResourceDecl *resource = &export->resources[i];
        char *resource_path;
        error = path_join(handle->module_path, resource->path, &resource_path);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not construct resource path")
            goto release_resources;
        }
        error = fimo_array_list_push(&resources, sizeof(const char *), _Alignof(const char *), &resource_path, NULL);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert resource into the resource table")
            goto release_resources;
        }
    }
    (*element)->resources = resources.elements;

    // Init namespaces.
    for (FimoISize i = 0; i < export->namespace_imports_count; i++) {
        const FimoModuleNamespaceImport *import = &export->namespace_imports[i];
        const struct Namespace_ *ns = ctx_get_ns_(ctx, import->name);
        if (ns == NULL) {
            ERROR_(ctx, error, "could not find namespace, ns='%s'", import->name)
            goto release_namespaces;
        }
        error = module_info_set_ns_(info_inner, import->name, true);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert namespace into the module info")
            goto release_namespaces;
        }
    }

    // Init imports.
    FimoArrayList imports = fimo_array_list_new();
    for (FimoISize i = 0; i < export->symbol_imports_count; i++) {
        const FimoModuleSymbolImport *import = &export->symbol_imports[i];
        const struct Symbol_ *symbol = ctx_get_symbol_compatible_(ctx, import->name, import->ns, import->version);
        if (symbol == NULL) {
            ERROR_(ctx, error, "could not find symbol, symbol='%s', ns='%s'", import->name, import->ns)
            goto release_imports;
        }
        const struct Module_ *module = ctx_get_module_(ctx, symbol->module);
        FIMO_DEBUG_ASSERT(module);
        const struct ModuleInfo_ *module_info = module_info_from_module_(module->module);
        struct ModuleInfoInner_ *module_info_inner = module_info_lock_(module_info);
        const struct ModuleInfoSymbol_ *module_info_symbol =
                module_info_get_symbol_(module_info_inner, symbol->name, symbol->ns, symbol->version);
        FIMO_DEBUG_ASSERT(module_info_symbol);
        const FimoModuleRawSymbol *raw_symbol = &module_info_symbol->symbol;
        error = fimo_array_list_push(&imports, sizeof(FimoModuleRawSymbol *), _Alignof(FimoModuleRawSymbol *),
                                     &raw_symbol, NULL);
        if (FIMO_RESULT_IS_ERROR(error)) {
            module_info_unlock_(module_info_inner);
            ERROR_SIMPLE_(ctx, error, "could not insert symbol into the import table")
            goto release_imports;
        }
        if (module_info_get_dependency_(info_inner, symbol->module) == NULL) {
            error = module_info_set_dependency_(info_inner, module->module->module_info, true);
            if (FIMO_RESULT_IS_ERROR(error)) {
                module_info_unlock_(module_info_inner);
                ERROR_SIMPLE_(ctx, error, "could not insert dependency into the module info")
                goto release_imports;
            }
        }
        module_info_unlock_(module_info_inner);
    }
    (*element)->imports = imports.elements;

    // Init the module.
    if (export->module_constructor) {
        void *module_data = NULL;
        module_info_unlock_(info_inner);
        loading_set_unlock_(set);
        ctx_unlock_(ctx);
        error = export->module_constructor(*element, set, &module_data);
        ctx_lock_(ctx);
        loading_set_lock_(set);
        module_info_lock_(info);
        (*element)->module_data = module_data;
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not construct the module data")
            goto release_imports;
        }
    }

    // Init the static exports.
    FimoArrayList exports = fimo_array_list_new();
    for (FimoISize i = 0; i < export->symbol_exports_count; i++) {
        const FimoModuleSymbolExport *symbol = &export->symbol_exports[i];
        const struct ModuleInfoSymbol_ *info_symbol;
        error = module_info_set_symbol_(info_inner, symbol->name, symbol->ns, symbol->version, NULL, symbol->symbol,
                                        &info_symbol);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert symbol into the module info")
            goto release_exports;
        }
        FIMO_DEBUG_ASSERT(info_symbol)
        const FimoModuleRawSymbol *raw_symbol = &info_symbol->symbol;
        error = fimo_array_list_push(&exports, sizeof(const FimoModuleRawSymbol *),
                                     _Alignof(const FimoModuleRawSymbol *), &raw_symbol, NULL);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert symbol into the export table")
            goto release_exports;
        }
    }

    // Init the dynamic exports.
    for (FimoISize i = 0; i < export->dynamic_symbol_exports_count; i++) {
        const FimoModuleDynamicSymbolExport *symbol = &export->dynamic_symbol_exports[i];
        void *sym;
        error = symbol->constructor(*element, &sym);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_(ctx, error, "could not construct symbol, symbol='%s', ns='%s'", symbol->name, symbol->ns)
            goto release_exports;
        }
        const struct ModuleInfoSymbol_ *info_symbol;
        error = module_info_set_symbol_(info_inner, symbol->name, symbol->ns, symbol->version, symbol->destructor, sym,
                                        &info_symbol);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert symbol into the module info")
            symbol->destructor(sym);
            goto release_exports;
        }
        FIMO_DEBUG_ASSERT(info_symbol)
        const FimoModuleRawSymbol *raw_symbol = &info_symbol->symbol;
        error = fimo_array_list_push(&exports, sizeof(const FimoModuleRawSymbol *),
                                     _Alignof(const FimoModuleRawSymbol *), &raw_symbol, NULL);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not insert symbol into the export table")
            goto release_exports;
        }
    }
    (*element)->exports = exports.elements;

    module_info_unlock_(info_inner);

    return FIMO_EOK;

release_exports:
    if ((*element)->exports == NULL) {
        fimo_array_list_free(&exports, sizeof(const FimoModuleRawSymbol *), _Alignof(const FimoModuleRawSymbol *),
                             NULL);
    }
release_imports:
    if ((*element)->imports == NULL) {
        fimo_array_list_free(&imports, sizeof(FimoModuleRawSymbol *), _Alignof(FimoModuleRawSymbol *), NULL);
    }
release_namespaces:;
release_resources:;
    if ((*element)->resources == NULL) {
        fimo_array_list_free(&resources, sizeof(const char *), _Alignof(const char *), NULL);
    }
release_parameters:
    if ((*element)->parameters == NULL) {
        fimo_array_list_free(&params, sizeof(FimoModuleParam *), _Alignof(FimoModuleParam *), NULL);
    }
release_ctx:
    fimo_internal_context_release(TO_CTX_(ctx));
    module_info_unlock_(info_inner);
    module_info_release_(info, false);

    if (*element != NULL) {
        fimo_free(*element);
    }

    return error;
}

static void fi_module_free_(struct ModuleInfoInner_ *info_inner, FimoContext *context) {
    FIMO_DEBUG_ASSERT(info_inner && !module_info_is_detached_(info_inner))
    FimoModule *module = (FimoModule *)info_inner->module;
    const struct ModuleInfo_ *info = module_info_from_inner_(info_inner);
    module_info_detach_(info_inner, true);
    module_info_unlock_(info_inner);
    module_info_release_(info, true);

    if (module->parameters) {
        fimo_free((void *)module->parameters);
        module->parameters = NULL;
    }
    if (module->resources) {
        fimo_free((void *)module->resources);
        module->resources = module;
    }
    if (module->imports) {
        fimo_free((void *)module->imports);
        module->imports = NULL;
    }

    if (context) {
        *context = module->context;
    }
    else {
        fimo_internal_context_release(module->context.data);
    }
}

///////////////////////////////////////////////////////////////////////
//// Fimo Module Info
///////////////////////////////////////////////////////////////////////

static void fi_module_info_acquire_(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    const struct ModuleInfo_ *module_info = module_info_from_module_info_(info);
    module_info_acquire_(module_info);
}

static void fi_module_info_release_(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    const struct ModuleInfo_ *module_info = module_info_from_module_info_(info);
    module_info_release_(module_info, true);
}

static bool fi_module_info_is_loaded(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    const struct ModuleInfo_ *module_info = module_info_from_module_info_(info);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(module_info);
    bool loaded = !module_info_is_detached_(info_inner);
    module_info_unlock_(info_inner);
    return loaded;
}

static FimoResult fi_module_info_lock_unload_(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info);
    const struct ModuleInfo_ *module_info = module_info_from_module_info_(info);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(module_info);
    const FimoResult error = module_info_prevent_unload_(info_inner);
    module_info_unlock_(info_inner);
    return error;
}

static void fi_module_info_unlock_unload(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info);
    const struct ModuleInfo_ *module_info = module_info_from_module_info_(info);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(module_info);
    module_info_allow_unload_(info_inner);
    module_info_unlock_(info_inner);
}

///////////////////////////////////////////////////////////////////////
//// Fimo Module Export
///////////////////////////////////////////////////////////////////////

static void fi_module_export_cleanup_(const FimoModuleExport *export) {
    FIMO_DEBUG_ASSERT(export)

    // If the modifiers list is invalid we do nothing.
    if ((export->modifiers == NULL && export->modifiers_count != 0) ||
        (export->modifiers != NULL && export->modifiers_count == 0)) {
        return;
    }

    for (FimoISize i = 0; i < (FimoISize) export->modifiers_count; i++) {
        const FimoModuleExportModifier *modifier = &export->modifiers[i];

        // Skip invalid modifiers.
        if (modifier->key < 0 || modifier->key >= FIMO_MODULE_EXPORT_MODIFIER_KEY_LAST) {
            continue;
        }

        switch (modifier->key) {
            case FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR: {
                const FimoModuleExportModifierDestructor *value = modifier->value;
                if (value == NULL) {
                    continue;
                }
                value->destructor(value->data);
                break;
            }
            case FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY: {
                const FimoModuleInfo *value = modifier->value;
                if (value == NULL) {
                    continue;
                }
                FIMO_MODULE_INFO_RELEASE(value);
                break;
            }
            default:
                break;
        }
    }
}

static bool fi_module_export_parameters_are_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->parameters == NULL && export->parameters_count != 0) ||
        (export->parameters != NULL && export->parameters_count == 0)) {
        WARN_(ctx, "invalid parameters count, module='%s', parameters='%p', parameters_count='%u'", export->name,
              (void *)export->parameters, export->parameters_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->parameters_count; i++) {
        const FimoModuleParamDecl *param = &export->parameters[i];
        if (param->name == NULL) {
            WARN_(ctx, "parameter name is 'NULL', module='%s'", export->name)
            return false;
        }
        if (param->type > FIMO_MODULE_PARAM_TYPE_I64) {
            WARN_(ctx, "invalid parameter type, module='%s', parameter='%s', type='%d'", export->name, param->name,
                  param->type)
            return false;
        }
        if (param->read_access > FIMO_MODULE_PARAM_ACCESS_PRIVATE) {
            WARN_(ctx, "invalid parameter read access, module='%s', parameter='%s', access='%d'", export->name,
                  param->name, param->read_access)
            return false;
        }
        if (param->write_access > FIMO_MODULE_PARAM_ACCESS_PRIVATE) {
            WARN_(ctx, "invalid parameter write access, module='%s', parameter='%s', access='%d'", export->name,
                  param->name, param->write_access)
            return false;
        }
        if (param->setter == NULL) {
            WARN_(ctx, "parameter setter is 'NULL', module='%s', parameter='%s'", export->name, param->name)
            return false;
        }
        if (param->getter == NULL) {
            WARN_(ctx, "parameter getter is 'NULL', module='%s', parameter='%s'", export->name, param->name)
            return false;
        }

        for (FimoISize j = 0; j < i; j++) {
            const FimoModuleParamDecl *p = &export->parameters[j];
            if (strcmp(param->name, p->name) == 0) {
                WARN_(ctx, "duplicate parameter, module='%s', parameter='%s'", export->name, param->name)
                return false;
            }
        }
    }

    return true;
}

static bool fi_module_export_resources_are_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->resources == NULL && export->resources_count != 0) ||
        (export->resources != NULL && export->resources_count == 0)) {
        WARN_(ctx, "invalid resources count, module='%s', resources='%p', resources_count='%u'", export->name,
              (void *)export->resources, export->resources_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->resources_count; i++) {
        const FimoModuleResourceDecl *resource = &export->resources[i];
        if (resource->path == NULL) {
            WARN_(ctx, "resource path is 'NULL', module='%s'", export->name)
            return false;
        }
        if (resource->path[0] != '\0' && (resource->path[0] == '\\' || resource->path[0] == '/')) {
            WARN_(ctx, "resource path begins with a slash, module='%s', resource='%s'", export->name, resource->path)
            return false;
        }
    }

    return true;
}

static bool fi_module_export_namespaces_are_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->namespace_imports == NULL && export->namespace_imports_count != 0) ||
        (export->namespace_imports != NULL && export->namespace_imports_count == 0)) {
        WARN_(ctx, "invalid namespace import count, module='%s', namespace_imports='%p', namespace_imports_count='%u'",
              export->name, (void *)export->namespace_imports, export->namespace_imports_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->namespace_imports_count; i++) {
        const FimoModuleNamespaceImport *ns = &export->namespace_imports[i];
        if (ns->name == NULL) {
            WARN_(ctx, "namespace import name is 'Null', module='%s'", export->name)
            return false;
        }
    }

    return true;
}

static bool fi_module_export_imports_are_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->symbol_imports == NULL && export->symbol_imports_count != 0) ||
        (export->symbol_imports != NULL && export->symbol_imports_count == 0)) {
        WARN_(ctx, "invalid symbol import count, module='%s', symbol_imports='%p', symbol_imports_count='%u'",
              export->name, (void *)export->symbol_imports, export->symbol_imports_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->symbol_imports_count; i++) {
        const FimoModuleSymbolImport *sym = &export->symbol_imports[i];
        if (sym->name == NULL) {
            WARN_(ctx, "symbol import name is 'Null', module='%s'", export->name)
            return false;
        }
        if (strcmp(sym->ns, GLOBAL_NS) != 0) {
            bool found = false;
            for (FimoISize j = 0; j < (FimoISize) export->namespace_imports_count; j++) {
                const FimoModuleNamespaceImport *ns = &export->namespace_imports[j];
                if (strcmp(sym->ns, ns->name) == 0) {
                    found = true;
                    break;
                }
            }
            if (!found) {
                WARN_(ctx, "symbol uses a namespace that was not imported, module='%s', symbol='%s', ns='%s'",
                      export->name, sym->name, sym->ns)
                return false;
            }
        }
    }

    return true;
}

static bool fi_module_export_static_exports_are_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->symbol_exports == NULL && export->symbol_exports_count != 0) ||
        (export->symbol_exports != NULL && export->symbol_exports_count == 0)) {
        WARN_(ctx, "invalid symbol export count, module='%s', symbol_exports='%p', symbol_exports_count='%u'",
              export->name, (void *)export->symbol_exports, export->symbol_exports_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->symbol_exports_count; i++) {
        const FimoModuleSymbolExport *sym = &export->symbol_exports[i];
        if (sym->name == NULL) {
            WARN_(ctx, "symbol export name is 'NULL', module='%s'", export->name)
            return false;
        }
        if (sym->ns == NULL) {
            WARN_(ctx, "symbol export namespace is 'NULL', module='%s', symbol='%s'", export->name, sym->name)
            return false;
        }
        if (sym->symbol == NULL) {
            WARN_(ctx, "symbol export is 'NULL', module='%s', symbol='%s', ns='%s'", export->name, sym->name, sym->ns)
            return false;
        }
        for (FimoISize j = 0; j < (FimoISize) export->symbol_imports_count; j++) {
            const FimoModuleSymbolImport *s = &export->symbol_imports[j];
            if (strcmp(sym->name, s->name) == 0 && strcmp(sym->ns, s->ns) == 0) {
                WARN_(ctx, "duplicate symbol, module='%s', symbol='%s', ns='%s'", export->name, sym->name, sym->ns)
                return false;
            }
        }
        for (FimoISize j = 0; j < i; j++) {
            const FimoModuleSymbolExport *s = &export->symbol_exports[j];
            if (strcmp(sym->name, s->name) == 0 && strcmp(sym->ns, s->ns) == 0) {
                WARN_(ctx, "duplicate symbol, module='%s', symbol='%s', ns='%s'", export->name, sym->name, sym->ns)
                return false;
            }
        }
    }

    return true;
}

static bool fi_module_export_dynamic_exports_are_valid_(const FimoModuleExport *export,
                                                        FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->dynamic_symbol_exports == NULL && export->dynamic_symbol_exports_count != 0) ||
        (export->dynamic_symbol_exports != NULL && export->dynamic_symbol_exports_count == 0)) {
        WARN_(ctx,
              "invalid dynamic symbol export count, module='%s', dynamic_symbol_exports='%p', "
              "dynamic_symbol_exports_count='%u'",
              export->name, (void *)export->dynamic_symbol_exports, export->dynamic_symbol_exports_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->dynamic_symbol_exports_count; i++) {
        const FimoModuleDynamicSymbolExport *sym = &export->dynamic_symbol_exports[i];
        if (sym->name == NULL) {
            WARN_(ctx, "symbol export name is 'NULL', module='%s'", export->name)
            return false;
        }
        if (sym->ns == NULL) {
            WARN_(ctx, "symbol export namespace is 'NULL', module='%s', symbol='%s'", export->name, sym->name)
            return false;
        }
        if (sym->constructor == NULL) {
            WARN_(ctx, "symbol constructor is 'NULL', module='%s', symbol='%s', ns='%s'", export->name, sym->name,
                  sym->ns)
            return false;
        }
        if (sym->destructor == NULL) {
            WARN_(ctx, "symbol destructor is 'NULL', module='%s', symbol='%s', ns='%s'", export->name, sym->name,
                  sym->ns)
            return false;
        }
        for (FimoISize j = 0; j < (FimoISize) export->symbol_imports_count; j++) {
            const FimoModuleSymbolImport *s = &export->symbol_imports[j];
            if (strcmp(sym->name, s->name) == 0 && strcmp(sym->ns, s->ns) == 0) {
                WARN_(ctx, "duplicate symbol, module='%s', symbol='%s', ns='%s'", export->name, sym->name, sym->ns)
                return false;
            }
        }
        for (FimoISize j = 0; j < (FimoISize) export->symbol_exports_count; j++) {
            const FimoModuleSymbolExport *s = &export->symbol_exports[j];
            if (strcmp(sym->name, s->name) == 0 && strcmp(sym->ns, s->ns) == 0) {
                WARN_(ctx, "duplicate symbol, module='%s', symbol='%s', ns='%s'", export->name, sym->name, sym->ns)
                return false;
            }
        }
        for (FimoISize j = 0; j < i; j++) {
            const FimoModuleDynamicSymbolExport *s = &export->dynamic_symbol_exports[j];
            if (strcmp(sym->name, s->name) == 0 && strcmp(sym->ns, s->ns) == 0) {
                WARN_(ctx, "duplicate symbol, module='%s', symbol='%s', ns='%s'", export->name, sym->name, sym->ns)
                return false;
            }
        }
    }

    return true;
}

static bool fi_module_export_modifiers_are_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    if ((export->modifiers == NULL && export->modifiers_count != 0) ||
        (export->modifiers != NULL && export->modifiers_count == 0)) {
        WARN_(ctx, "invalid modifiers count, module='%s', modifiers='%p', modifiers_count='%u'", export->name,
              (void *)export->modifiers, export->modifiers_count)
        return false;
    }
    for (FimoISize i = 0; i < (FimoISize) export->modifiers_count; i++) {
        const FimoModuleExportModifier *modifier = &export->modifiers[i];
        switch (modifier->key) {
            case FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR:
            case FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY: {
                if (modifier->value == NULL) {
                    WARN_(ctx, "no value set for modifier, module='%s', modifier='%d'", export->name, modifier->key)
                    return false;
                }
                break;
            }
            default: {
                WARN_(ctx, "unrecognized modifier key, module='%s', modifier='%d'", export->name, modifier->key)
                return false;
            }
        }
    }

    return true;
}

static bool fi_module_export_is_valid_(const FimoModuleExport *export, FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(export && ctx)
    _Static_assert(FIMO_MODULE_EXPORT_ABI == 0, "Unknown module abi version");
    if (export->type != FIMO_STRUCT_TYPE_MODULE_EXPORT) {
        WARN_(ctx, "invalid module struct type, type='%d'", export->type)
        return false;
    }
    if (export->next != NULL) {
        WARN_(ctx, "next pointer must currently be 'NULL', next='%p'", (void *)export->next)
        return false;
    }
    if (export->export_abi != FIMO_MODULE_EXPORT_ABI) {
        WARN_(ctx, "unknown module abi version, export_abi='%d'", export->export_abi)
        return false;
    }
    if (export->name == NULL) {
        WARN_SIMPLE_(ctx, "module name is 'NULL'")
        return false;
    }
    if ((export->module_constructor == NULL) != (export->module_destructor == NULL)) {
        WARN_(ctx, "module constructor must both be set or null, module='%s'", export->name)
        return false;
    }

    if (!fi_module_export_parameters_are_valid_(export, ctx) || !fi_module_export_resources_are_valid_(export, ctx) ||
        !fi_module_export_namespaces_are_valid_(export, ctx) || !fi_module_export_imports_are_valid_(export, ctx) ||
        !fi_module_export_imports_are_valid_(export, ctx) || !fi_module_export_static_exports_are_valid_(export, ctx) ||
        !fi_module_export_dynamic_exports_are_valid_(export, ctx) ||
        !fi_module_export_modifiers_are_valid_(export, ctx)) {
        return false;
    }

    return true;
}

///////////////////////////////////////////////////////////////////////
//// Trampoline functions
///////////////////////////////////////////////////////////////////////

FimoResult fimo_internal_trampoline_module_pseudo_module_new(void *ctx, const FimoModule **module) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_pseudo_module_new(TO_MODULE_CTX_(ctx), module);
}

FimoResult fimo_internal_trampoline_module_pseudo_module_destroy(void *ctx, const FimoModule *module,
                                                                 FimoContext *module_context) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_pseudo_module_destroy(TO_MODULE_CTX_(ctx), module, module_context);
}

FimoResult fimo_internal_trampoline_module_set_new(void *ctx, FimoModuleLoadingSet **set) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_new(TO_MODULE_CTX_(ctx), set);
}

FimoResult fimo_internal_trampoline_module_set_has_module(void *ctx, FimoModuleLoadingSet *set, const char *name,
                                                          bool *has_module) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_has_module(TO_MODULE_CTX_(ctx), set, name, has_module);
}

FimoResult fimo_internal_trampoline_module_set_has_symbol(void *ctx, FimoModuleLoadingSet *set, const char *name,
                                                          const char *ns, FimoVersion version, bool *has_symbol) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_has_symbol(TO_MODULE_CTX_(ctx), set, name, ns, version, has_symbol);
}

FimoResult fimo_internal_trampoline_module_set_append_callback(void *ctx, FimoModuleLoadingSet *set,
                                                               const char *module_name,
                                                               FimoModuleLoadingSuccessCallback on_success,
                                                               FimoModuleLoadingErrorCallback on_error,
                                                               void *user_data) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_append_callback(TO_MODULE_CTX_(ctx), set, module_name, on_success, on_error,
                                                    user_data);
}

FimoResult fimo_internal_trampoline_module_set_append_freestanding_module(void *ctx, const FimoModule *module,
                                                                          FimoModuleLoadingSet *set,
                                                                          const FimoModuleExport *export) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_append_freestanding_module(TO_MODULE_CTX_(ctx), module, set, export);
}

FimoResult fimo_internal_trampoline_module_set_append_modules(
        void *ctx, FimoModuleLoadingSet *set, const char *module_path, FimoModuleLoadingFilter filter,
        void *filter_data, void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *),
        const void *binary_handle) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_append_modules(TO_MODULE_CTX_(ctx), set, module_path, filter, filter_data,
                                                   export_iterator, binary_handle);
}

FimoResult fimo_internal_trampoline_module_set_dismiss(void *ctx, FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_dismiss(TO_MODULE_CTX_(ctx), set);
}

FimoResult fimo_internal_trampoline_module_set_finish(void *ctx, FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_set_finish(TO_MODULE_CTX_(ctx), set);
}

FimoResult fimo_internal_trampoline_module_find_by_name(void *ctx, const char *name, const FimoModuleInfo **module) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_find_by_name(TO_MODULE_CTX_(ctx), name, module);
}

FimoResult fimo_internal_trampoline_module_find_by_symbol(void *ctx, const char *name, const char *ns,
                                                          FimoVersion version, const FimoModuleInfo **module) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_find_by_symbol(TO_MODULE_CTX_(ctx), name, ns, version, module);
}

FimoResult fimo_internal_trampoline_module_namespace_exists(void *ctx, const char *ns, bool *exists) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_namespace_exists(TO_MODULE_CTX_(ctx), ns, exists);
}

FimoResult fimo_internal_trampoline_module_namespace_include(void *ctx, const FimoModule *module, const char *ns) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_namespace_include(TO_MODULE_CTX_(ctx), module, ns);
}

FimoResult fimo_internal_trampoline_module_namespace_exclude(void *ctx, const FimoModule *module, const char *ns) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_namespace_exclude(TO_MODULE_CTX_(ctx), module, ns);
}

FimoResult fimo_internal_trampoline_module_namespace_included(void *ctx, const FimoModule *module, const char *ns,
                                                              bool *is_included, bool *is_static) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_namespace_included(TO_MODULE_CTX_(ctx), module, ns, is_included, is_static);
}

FimoResult fimo_internal_trampoline_module_acquire_dependency(void *ctx, const FimoModule *module,
                                                              const FimoModuleInfo *dependency) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_acquire_dependency(TO_MODULE_CTX_(ctx), module, dependency);
}

FimoResult fimo_internal_trampoline_module_relinquish_dependency(void *ctx, const FimoModule *module,
                                                                 const FimoModuleInfo *dependency) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_relinquish_dependency(TO_MODULE_CTX_(ctx), module, dependency);
}

FimoResult fimo_internal_trampoline_module_has_dependency(void *ctx, const FimoModule *module,
                                                          const FimoModuleInfo *other, bool *has_dependency,
                                                          bool *is_static) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_has_dependency(TO_MODULE_CTX_(ctx), module, other, has_dependency, is_static);
}

FimoResult fimo_internal_trampoline_module_param_query(void *ctx, const char *module_name, const char *param,
                                                       FimoModuleParamType *type, FimoModuleParamAccess *read,
                                                       FimoModuleParamAccess *write) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_query(TO_MODULE_CTX_(ctx), module_name, param, type, read, write);
}

FimoResult fimo_internal_trampoline_module_param_set_public(void *ctx, const void *value, FimoModuleParamType type,
                                                            const char *module_name, const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_set_public(TO_MODULE_CTX_(ctx), value, type, module_name, param);
}

FimoResult fimo_internal_trampoline_module_param_get_public(void *ctx, void *value, FimoModuleParamType *type,
                                                            const char *module_name, const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_get_public(TO_MODULE_CTX_(ctx), value, type, module_name, param);
}

FimoResult fimo_internal_trampoline_module_param_set_dependency(void *ctx, const FimoModule *module, const void *value,
                                                                FimoModuleParamType type, const char *module_name,
                                                                const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_set_dependency(TO_MODULE_CTX_(ctx), module, value, type, module_name, param);
}

FimoResult fimo_internal_trampoline_module_param_get_dependency(void *ctx, const FimoModule *module, void *value,
                                                                FimoModuleParamType *type, const char *module_name,
                                                                const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_get_dependency(TO_MODULE_CTX_(ctx), module, value, type, module_name, param);
}

FimoResult fimo_internal_trampoline_module_load_symbol(void *ctx, const FimoModule *module, const char *name,
                                                       const char *ns, FimoVersion version,
                                                       const FimoModuleRawSymbol **symbol) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_load_symbol(TO_MODULE_CTX_(ctx), module, name, ns, version, symbol);
}

FimoResult fimo_internal_trampoline_module_unload(void *ctx, const FimoModuleInfo *module) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_unload(TO_MODULE_CTX_(ctx), module);
}

FimoResult fimo_internal_trampoline_module_param_set_private(void *ctx, const FimoModule *module, const void *value,
                                                             FimoModuleParamType type, FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_set_private(TO_MODULE_CTX_(ctx), module, value, type, param);
}

FimoResult fimo_internal_trampoline_module_param_get_private(void *ctx, const FimoModule *module, void *value,
                                                             FimoModuleParamType *type, const FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_get_private(TO_MODULE_CTX_(ctx), module, value, type, param);
}

FimoResult fimo_internal_trampoline_module_param_set_inner(void *ctx, const FimoModule *module, const void *value,
                                                           FimoModuleParamType type, FimoModuleParamData *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_set_inner(TO_MODULE_CTX_(ctx), module, value, type, param);
}

FimoResult fimo_internal_trampoline_module_get_inner(void *ctx, const FimoModule *module, void *value,
                                                     FimoModuleParamType *type, const FimoModuleParamData *param) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_module_param_get_inner(TO_MODULE_CTX_(ctx), module, value, type, param);
}

///////////////////////////////////////////////////////////////////////
//// Module Subsystem API
///////////////////////////////////////////////////////////////////////

FIMO_MUST_USE
FimoResult fimo_internal_module_init(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "initializing the module subsystem")

    const FimoResult error = ctx_init_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not initialize the module subsystem")
        return error;
    }

    return FIMO_EOK;
}

void fimo_internal_module_destroy(FimoInternalModuleContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    TRACE_SIMPLE_(ctx, "destroying the module subsystem")
    ctx_deinit_(ctx);
}

FIMO_MUST_USE
FimoResult fimo_internal_module_pseudo_module_new(FimoInternalModuleContext *ctx, const FimoModule **module) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p'", (void *)module)
        return FIMO_EINVAL;
    }

    TRACE_SIMPLE_(ctx, "new pseudo module")
    ctx_lock_(ctx);
    char name_buffer[32];
    const FimoU64 num_modules = (FimoU64)hashmap_count(ctx->modules);

    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    snprintf(name_buffer, sizeof(name_buffer), "_pseudo_%" PRIu64, num_modules);
    FIMO_PRAGMA_MSVC(warning(pop))

    FimoModule *module_;
    FimoResult error = fi_module_new_pseudo_(ctx, name_buffer, &module_);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not create a new module object")
        return error;
    }

    const struct ModuleInfo_ *info = module_info_from_module_(module_);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    error = ctx_add_module_(ctx, info_inner);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not add module to context")
        fi_module_free_(info_inner, NULL);
        return error;
    }
    module_info_unlock_(info_inner);
    ctx_unlock_(ctx);

    *module = module_;
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_pseudo_module_destroy(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                      FimoContext *module_context) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || module_context == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', module_context='%p'", (void *)module,
               (void *)module_context)
        return FIMO_EINVAL;
    }

    TRACE_SIMPLE_(ctx, "destroying pseudo module")
    ctx_lock_(ctx);
    const struct ModuleInfo_ *info = module_info_from_module_(module);
    if (module_info_from_module_(module)->type != MODULE_TYPE_PSEUDO_) {
        FimoResult error = ERR_IS_NOT_PSEUDO_;
        ERROR_SIMPLE_(ctx, error, "module is not a pseudo module")
        return error;
    }
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);

    FimoResult error = ctx_remove_module_(ctx, info_inner);
    if (FIMO_RESULT_IS_ERROR(error)) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        ERROR_SIMPLE_(ctx, error, "could not remove module from context")
        return error;
    }

    fi_module_free_(info_inner, module_context);

    error = ctx_cleanup_loose_modules(ctx);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not cleanup loose modules")
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_new(FimoInternalModuleContext *ctx, FimoModuleLoadingSet **set) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, set='%p'", (void *)set)
        return FIMO_EINVAL;
    }

    TRACE_SIMPLE_(ctx, "")
    const FimoResult error = loading_set_new_(set);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not create set")
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_has_module(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                               const char *name, bool *has_module) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL || name == NULL || has_module == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, set='%p', name='%p', has_module='%p'", (void *)set,
               (void *)name, (void *)has_module)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "name='%s'", name)
    loading_set_lock_(set);
    const struct LoadingSetModule_ *module = loading_set_get_module_(set, name);
    *has_module = module != NULL;
    loading_set_unlock_(set);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_has_symbol(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                               const char *name, const char *ns, const FimoVersion version,
                                               bool *has_symbol) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL || name == NULL || ns == NULL || has_symbol == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, set='%p', name='%p', ns='%p', has_symbol='%p'", (void *)set,
               (void *)name, (void *)ns, (void *)has_symbol)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "name='%s', ns='%s'", name, ns)
    loading_set_lock_(set);
    const struct LoadingSetSymbol_ *sym = loading_set_get_symbol_(set, name, ns, version);
    *has_symbol = sym != NULL;
    loading_set_unlock_(set);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_append_callback(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                                    const char *module_name,
                                                    const FimoModuleLoadingSuccessCallback on_success,
                                                    const FimoModuleLoadingErrorCallback on_error, void *user_data) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL || module_name == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, set='%p', module_name='%p'", (void *)set, (void *)module_name)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module='%s'", module_name)
    const struct LoadingSetCallback_ callback = {
            .data = user_data,
            .error = on_error,
            .success = on_success,
    };

    loading_set_lock_(set);
    struct LoadingSetModule_ *module = (void *)loading_set_get_module_(set, module_name);
    if (module == NULL) {
        loading_set_unlock_(set);
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "module does not exist, module='%s'", module_name)
        return error;
    }

    const FimoResult error = loading_set_module_append_callback_(module, callback);
    loading_set_unlock_(set);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not append callback")
        return error;
    }

    return FIMO_EOK;
}

static FimoResult add_module_(FimoInternalModuleContext *ctx, struct hashmap *symbols, struct hashmap *modules,
                              struct hashmap *opt_symbols, struct hashmap *opt_modules, struct ModuleHandle_ *handle,
                              const FimoModuleExport *export, const FimoModule *owner) {
    FIMO_DEBUG_ASSERT(ctx && symbols && modules && handle && export)
    if (hashmap_get(modules, &(struct LoadingSetModule_){.name = export->name}) ||
        (opt_modules && hashmap_get(opt_modules, &(struct LoadingSetModule_){.name = export->name}))) {
        FimoResult error = ERR_DUPLICATE_MOD_;
        ERROR_(ctx, error, "duplicate module, module='%s'", export->name)
        return error;
    }
    for (FimoISize i = 0; i < (FimoISize) export->symbol_exports_count; i++) {
        const FimoModuleSymbolExport *sym = &export->symbol_exports[i];
        if (hashmap_get(symbols, &(struct LoadingSetSymbol_){.name = sym->name, .ns = sym->ns}) ||
            (opt_symbols && hashmap_get(opt_symbols, &(struct LoadingSetSymbol_){.name = sym->name, .ns = sym->ns}))) {
            FimoResult error = ERR_DUPLICATE_SYM_;
            ERROR_(ctx, error, "duplicate symbol, symbol='%s', ns='%s'", sym->name, sym->ns)
            return error;
        }
    }
    for (FimoISize i = 0; i < (FimoISize) export->dynamic_symbol_exports_count; i++) {
        const FimoModuleDynamicSymbolExport *sym = &export->dynamic_symbol_exports[i];
        if (hashmap_get(symbols, &(struct LoadingSetSymbol_){.name = sym->name, .ns = sym->ns}) ||
            (opt_symbols && hashmap_get(opt_symbols, &(struct LoadingSetSymbol_){.name = sym->name, .ns = sym->ns}))) {
            FimoResult error = ERR_DUPLICATE_SYM_;
            ERROR_(ctx, error, "duplicate symbol, symbol='%s', ns='%s'", sym->name, sym->ns)
            return error;
        }
    }

    for (FimoISize i = 0; i < (FimoISize) export->symbol_exports_count; i++) {
        const FimoModuleSymbolExport *sym = &export->symbol_exports[i];
        struct LoadingSetSymbol_ symbol;
        FimoResult error = loading_set_symbol_new_(sym->name, sym->ns, sym->version, export->name, &symbol);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not create symbol")
            return error;
        }
        hashmap_set(symbols, &symbol);
        if (hashmap_oom(symbols)) {
            error = FIMO_ENOMEM;
            loading_set_symbol_free_(&symbol);
            ERROR_SIMPLE_(ctx, error, "could not insert symbol")
            return error;
        }
    }
    for (FimoISize i = 0; i < (FimoISize) export->dynamic_symbol_exports_count; i++) {
        const FimoModuleDynamicSymbolExport *sym = &export->dynamic_symbol_exports[i];
        struct LoadingSetSymbol_ symbol;
        FimoResult error = loading_set_symbol_new_(sym->name, sym->ns, sym->version, export->name, &symbol);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not create symbol")
            return error;
        }
        hashmap_set(symbols, &symbol);
        if (hashmap_oom(symbols)) {
            error = FIMO_ENOMEM;
            loading_set_symbol_free_(&symbol);
            ERROR_SIMPLE_(ctx, error, "could not insert symbol")
            return error;
        }
    }

    {
        struct LoadingSetModule_ module;
        FimoResult error = loading_set_module_new_(export, handle, owner, &module);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_SIMPLE_(ctx, error, "could not create module")
            return error;
        }
        hashmap_set(modules, &module);
        if (hashmap_oom(modules)) {
            error = FIMO_ENOMEM;
            loading_set_module_free_(&module);
            ERROR_SIMPLE_(ctx, error, "could not insert symbol")
            return error;
        }
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_append_freestanding_module(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                               FimoModuleLoadingSet *set,
                                                               const FimoModuleExport *export) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || set == NULL || export == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', set='%p', export='%p'", (void *)module,
               (void *)set, (void *)export)
        return FIMO_EINVAL;
    }
    TRACE_SIMPLE_(ctx, "appending new freestanding module")

    // Check that the export is valid.
    if (!fi_module_export_is_valid_(export, ctx)) {
        fi_module_export_cleanup_(export);
        FimoResult error = ERR_INVALID_EXPORT_;
        ERROR_SIMPLE_(ctx, error, "export is invalid");
        return error;
    }

    // Inherit the same handle as the parent module.
    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    struct ModuleHandle_ *handle = info_inner->handle;
    module_handle_acquire_(handle);
    module_info_unlock_(info_inner);

    // Insert the export into the set.
    loading_set_lock_(set);
    FimoResult error = add_module_(ctx, set->symbols, set->modules, NULL, NULL, info_inner->handle, export, module);
    loading_set_unlock_(set);
    module_handle_release_(handle);
    if (FIMO_RESULT_IS_ERROR(error)) {
        fi_module_export_cleanup_(export);
        ERROR_(ctx, error, "could not insert the export into the set, module='%s'", export->name);
        return error;
    }

    return FIMO_EOK;
}

struct AppendModulesData_ {
    FimoInternalModuleContext *ctx;
    FimoResult error;
    FimoModuleLoadingFilter filter;
    void *filter_data;
    FimoArrayList exports;
};

static bool append_modules_iterator_(const FimoModuleExport *export, void *data) {
    struct AppendModulesData_ *d = data;

    if (!fi_module_export_is_valid_(export, d->ctx)) {
        fi_module_export_cleanup_(export);
        return true;
    }

    if (d->filter == NULL || d->filter(export, d->filter_data)) {
        d->error = fimo_array_list_push(&d->exports, sizeof(const FimoModuleExport *),
                                        _Alignof(const FimoModuleExport *), &export, NULL);
        if (FIMO_RESULT_IS_ERROR(d->error)) {
            return false;
        }
    }

    return true;
}

static FimoResult extract_exports_(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                   struct ModuleHandle_ *handle, FimoArrayList exports) {
    FIMO_ASSERT(ctx && set && handle)
    FimoResult error;
    struct hashmap *symbols = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct LoadingSetSymbol_), 0, 0, 0, (HashFn_)loading_set_symbol_hash_,
            (CmpFn_)loading_set_symbol_cmp_, (FreeFn_)loading_set_symbol_free_, NULL);
    if (symbols == NULL) {
        error = ERR_SYM_MAP_ALLOC_;
        ERROR_SIMPLE_(ctx, error, "could not allocate symbols map")
        goto alloc_symbols;
    }

    struct hashmap *modules = hashmap_new_with_allocator(
            malloc_, realloc_, free_, sizeof(struct LoadingSetModule_), 0, 0, 0, (HashFn_)loading_set_module_hash_,
            (CmpFn_)loading_set_module_cmp_, (FreeFn_)loading_set_module_free_, NULL);
    if (modules == NULL) {
        error = ERR_MOD_MAP_ALLOC_;
        ERROR_SIMPLE_(ctx, error, "could not allocate modules map")
        goto alloc_modules;
    }

    while (!fimo_array_list_is_empty(&exports)) {
        const FimoModuleExport *export;
        error = fimo_array_list_pop_back(&exports, sizeof(export), &export, NULL);
        FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(error))

        error = add_module_(ctx, symbols, modules, set->symbols, set->modules, handle, export, NULL);
        if (FIMO_RESULT_IS_ERROR(error)) {
            ERROR_(ctx, error, "could not add export to set, module='%s'", export->name)
            goto iterate_exports;
        }
    }

    {
        size_t iter_idx = 0;
        struct LoadingSetSymbol_ *iter_item = NULL;
        while (hashmap_iter(symbols, &iter_idx, (void **)&iter_item)) {
            iter_item = (void *)hashmap_delete(symbols, iter_item);
            iter_idx = 0;

            hashmap_set(set->symbols, iter_item);
            if (hashmap_oom(set->symbols)) {
                loading_set_symbol_free_(iter_item);
                error = FIMO_ENOMEM;
                ERROR_SIMPLE_(ctx, error, "could not merge symbol maps")
                goto transfer_symbols;
            }
        }
    }
    {
        size_t iter_idx = 0;
        struct LoadingSetModule_ *iter_item = NULL;
        while (hashmap_iter(modules, &iter_idx, (void *)&iter_item)) {
            iter_item = (void *)hashmap_delete(modules, iter_item);
            iter_idx = 0;

            hashmap_set(set->modules, iter_item);
            if (hashmap_oom(set->modules)) {
                loading_set_module_free_(iter_item);
                error = FIMO_ENOMEM;
                ERROR_SIMPLE_(ctx, error, "could not merge module maps")
                goto transfer_modules;
            }
        }
    }

    hashmap_free(modules);
    hashmap_free(symbols);

    set->should_recreate_map = true;
    return FIMO_EOK;

transfer_modules: {
    size_t iter_idx = 0;
    void *iter_item = NULL;
    while (hashmap_iter(modules, &iter_idx, &iter_item)) {
        void *mod = (void *)hashmap_delete(set->modules, iter_item);
        if (mod) {
            loading_set_module_free_(mod);
        }
    }
}
transfer_symbols: {
    size_t iter_idx = 0;
    void *iter_item = NULL;
    while (hashmap_iter(symbols, &iter_idx, &iter_item)) {
        void *sym = (void *)hashmap_delete(set->symbols, iter_item);
        if (sym) {
            loading_set_symbol_free_(sym);
        }
    }
}
iterate_exports:
    hashmap_free(modules);
alloc_modules:
    hashmap_free(symbols);
alloc_symbols:
    return error;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_append_modules(
        FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set, const char *module_path,
        const FimoModuleLoadingFilter filter, void *filter_data,
        void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *), const void *binary_handle) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL || export_iterator == NULL || binary_handle == NULL) {
        ERROR_(ctx, FIMO_EINVAL,
               "invalid null parameter, set='%p', module_path='%s', export_iterator='%p', binary_handle='%p'",
               (void *)set, module_path, *(void **)&export_iterator, binary_handle)
        return FIMO_EINVAL;
    }

    FimoResult error;
    struct ModuleHandle_ *handle = NULL;
    if (module_path) {
        TRACE_(ctx, "module_path='%s'", module_path)
        error = module_handle_new_plugin_(module_path, &handle);
    }
    else {
        TRACE_SIMPLE_(ctx, "local module")
        error = module_handle_new_local_(export_iterator, binary_handle, &handle);
    }
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not create module handle")
        return error;
    }

    struct AppendModulesData_ data = {.ctx = ctx,
                                      .error = FIMO_EOK,
                                      .filter = filter,
                                      .filter_data = filter_data,
                                      .exports = fimo_array_list_new()};
    handle->export_iterator(append_modules_iterator_, &data);
    if (FIMO_RESULT_IS_ERROR(data.error)) {
        error = data.error;
        ERROR_SIMPLE_(ctx, error, "could not iterate through the module exports of the binary")
        goto iterate_exports;
    }

    loading_set_lock_(set);
    error = extract_exports_(ctx, set, handle, data.exports);
    loading_set_unlock_(set);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not extract the module exports")
        goto extract_exports;
    }

    fimo_array_list_free(&data.exports, sizeof(const FimoModuleExport *), _Alignof(const FimoModuleExport *), NULL);
    module_handle_release_(handle);

    return FIMO_EOK;

extract_exports:;
iterate_exports:
    fimo_array_list_free(&data.exports, sizeof(const FimoModuleExport *), _Alignof(const FimoModuleExport *), NULL);
    module_handle_release_(handle);

    return error;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_dismiss(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, set='%p'", (void *)set)
        return FIMO_EINVAL;
    }

    TRACE_SIMPLE_(ctx, "dismissing set")
    loading_set_lock_(set);
    if (set->is_loading) {
        loading_set_unlock_(set);
        FimoResult error = ERR_IS_LOADING_;
        ERROR_SIMPLE_(ctx, error, "set is being loaded")
        return error;
    }
    loading_set_unlock_(set);
    loading_set_free_(set);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_set_finish(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set) {
    FIMO_DEBUG_ASSERT(ctx)
    if (set == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, set='%p'", (void *)set)
        return FIMO_EINVAL;
    }

    TRACE_SIMPLE_(ctx, "loading module set")
    ctx_lock_(ctx);
    loading_set_lock_(set);
    const FimoResult error = ctx_load_set(ctx, set);
    loading_set_unlock_(set);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not load set")
        loading_set_free_(set);
        return error;
    }

    loading_set_free_(set);
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_find_by_name(FimoInternalModuleContext *ctx, const char *name,
                                             const FimoModuleInfo **module) {
    FIMO_DEBUG_ASSERT(ctx)
    if (name == NULL || module == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, name='%p', module='%p'", (void *)name, (void *)module)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "name='%s'", name)
    ctx_lock_(ctx);
    const struct Module_ *mod = ctx_get_module_(ctx, name);
    if (mod == NULL) {
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "no module by the given name exists, module='%s'", name)
        return error;
    }
    *module = FIMO_MODULE_INFO_ACQUIRE(mod->module->module_info);
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_find_by_symbol(FimoInternalModuleContext *ctx, const char *name, const char *ns,
                                               const FimoVersion version, const FimoModuleInfo **module) {
    FIMO_DEBUG_ASSERT(ctx)
    if (name == NULL || ns == NULL || module == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, name='%p', ns='%p', module='%p'", (void *)name, (void *)ns,
               (void *)module)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "name='%s', ns='%s', version='%" PRIu32 ".%" PRIu32 ".%" PRIu32 "+%" PRIu64 "'", name, ns,
           version.major, version.minor, version.patch, version.build)
    ctx_lock_(ctx);
    const struct Symbol_ *symbol = ctx_get_symbol_compatible_(ctx, name, ns, version);
    if (symbol == NULL) {
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_SYM_;
        ERROR_(ctx, error,
               "no compatible symbol was found, name='%s', ns='%s', version='%" PRIu32 ".%" PRIu32 ".%" PRIu32
               "+%" PRIu64 "'",
               name, ns, version.major, version.minor, version.patch, version.build)
        return error;
    }

    const struct Module_ *mod = ctx_get_module_(ctx, symbol->module);
    FIMO_DEBUG_ASSERT(mod)
    *module = FIMO_MODULE_INFO_ACQUIRE(mod->module->module_info);
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_namespace_exists(FimoInternalModuleContext *ctx, const char *ns, bool *exists) {
    FIMO_DEBUG_ASSERT(ctx)
    if (ns == NULL || exists == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, ns='%p', exists='%p'", (void *)ns, (void *)exists)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "ns='%s'", ns)
    ctx_lock_(ctx);
    const struct Namespace_ *namespace = ctx_get_ns_(ctx, ns);
    *exists = namespace != NULL;
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_namespace_include(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                  const char *ns) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || ns == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', ns='%p'", (void *)module, (void *)ns)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "ns='%s', module='%s'", ns, module->module_info->name)
    ctx_lock_(ctx);
    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    const struct ModuleInfoNamespace_ *info_namespace = module_info_get_ns_(info_inner, ns);
    if (info_namespace != NULL) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_NS_INCLUDED_;
        ERROR_(ctx, error, "namespace was already included by the module, ns='%s', module='%s'", ns, info->info.name)
        return error;
    }

    const struct Namespace_ *namespace = ctx_get_ns_(ctx, ns);
    if (namespace == NULL) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_NS_;
        ERROR_(ctx, error, "namespace does not exist, ns='%s', module='%s'", ns, info->info.name)
        return error;
    }

    FimoResult error = ctx_ns_acquire_(ctx, ns);
    if (FIMO_RESULT_IS_ERROR(error)) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        ERROR_(ctx, error, "could not acquire namespace, ns='%s', module='%s'", ns, info->info.name)
        return error;
    }

    error = module_info_set_ns_(info_inner, ns, false);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ctx_ns_release_(ctx, ns);
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        ERROR_(ctx, error, "could not insert namespace into the module info, ns='%s', module='%s'", ns, info->info.name)
        return error;
    }
    module_info_unlock_(info_inner);
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_namespace_exclude(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                  const char *ns) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || ns == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', ns='%p'", (void *)module, (void *)ns)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "ns='%s', module='%s'", ns, module->module_info->name)
    ctx_lock_(ctx);
    struct ModuleInfo_ *info = (void *)module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    const struct ModuleInfoNamespace_ *info_namespace = module_info_get_ns_(info_inner, ns);
    if (info_namespace == NULL) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_NS_NOT_INCLUDED_;
        ERROR_(ctx, error, "namespace was not included by the module, ns='%s', module='%s'", ns, info->info.name)
        return error;
    }
    if (info_namespace->is_static) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_STATIC_NS_;
        ERROR_(ctx, error, "can not exclude static namespace, ns='%s', module='%s'", ns, info->info.name)
        return error;
    }

    module_info_delete_ns_(info_inner, ns);
    ctx_ns_release_(ctx, ns);
    module_info_unlock_(info_inner);
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_namespace_included(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                   const char *ns, bool *is_included, bool *is_static) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || ns == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', ns='%p', is_included='%p', is_static='%p'",
               (void *)module, (void *)ns, (void *)is_included, (void *)is_static)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "ns='%s', module='%s'", ns, module->module_info->name)
    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    const struct ModuleInfoNamespace_ *info_namespace = module_info_get_ns_(info_inner, ns);
    if (info_namespace != NULL) {
        *is_included = true;
        *is_static = info_namespace->is_static;
    }
    else {
        *is_included = false;
        *is_static = false;
    }
    module_info_unlock_(info_inner);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_acquire_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                   const FimoModuleInfo *dependency) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || dependency == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', dependency='%p'", (void *)module,
               (void *)dependency)
        return FIMO_EINVAL;
    }

    const struct ModuleInfo_ *info = module_info_from_module_(module);
    const struct ModuleInfo_ *dependency_info = module_info_from_module_info_(dependency);
    TRACE_(ctx, "module='%s', dependency='%s'", info->info.name, dependency_info->info.name)
    if (info == dependency_info) {
        FimoResult error = ERR_CYCLIC_DEPENDENCY_;
        ERROR_(ctx, error, "can not link module to itself, module='%s'", info->info.name)
        return error;
    }

    ctx_lock_(ctx);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    struct ModuleInfoInner_ *dependency_info_inner = module_info_lock_(dependency_info);
    const FimoResult error = ctx_link_module_(ctx, info_inner, dependency_info_inner);
    module_info_unlock_(dependency_info_inner);
    module_info_unlock_(info_inner);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not acquire dependency, module='%s', dependency='%s'", module->module_info->name,
               dependency->name)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_relinquish_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                      const FimoModuleInfo *dependency) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || dependency == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', dependency='%p'", (void *)module,
               (void *)dependency)
        return FIMO_EINVAL;
    }

    const struct ModuleInfo_ *info = module_info_from_module_(module);
    const struct ModuleInfo_ *dependency_info = module_info_from_module_info_(dependency);
    TRACE_(ctx, "module='%s', dependency='%s'", module->module_info->name, dependency->name)
    if (info == dependency_info) {
        FimoResult error = ERR_NOT_A_DEPENDENCY_;
        ERROR_(ctx, error, "module can not relinquish itself, module='%s'", info->info.name)
        return error;
    }

    ctx_lock_(ctx);
    struct ModuleInfoInner_ *inner = module_info_lock_(info);
    struct ModuleInfoInner_ *dependency_info_inner = module_info_lock_(dependency_info);
    const FimoResult error = ctx_unlink_module_(ctx, inner, dependency_info_inner);
    module_info_unlock_(dependency_info_inner);
    module_info_unlock_(inner);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not relinquish dependency, module='%s', dependency='%s'", module->module_info->name,
               dependency->name)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_has_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                               const FimoModuleInfo *other, bool *has_dependency, bool *is_static) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || other == NULL || has_dependency == NULL || is_static == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', other='%p', has_dependency='%p', is_static='%p'",
               (void *)module, (void *)other, (void *)has_dependency, (void *)is_static)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module='%s', other='%s'", module->module_info->name, other->name)
    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    const struct ModuleInfoDependency_ *dependency = module_info_get_dependency_(info_inner, other->name);
    if (dependency == NULL) {
        *has_dependency = false;
        *is_static = false;
    }
    else {
        *has_dependency = true;
        *is_static = dependency->is_static;
    }
    module_info_unlock_(info_inner);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_load_symbol(FimoInternalModuleContext *ctx, const FimoModule *module, const char *name,
                                            const char *ns, FimoVersion version, const FimoModuleRawSymbol **symbol) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || name == NULL || ns == NULL || symbol == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', name='%p', ns='%p', symbol='%p'", (void *)module,
               (void *)name, (void *)ns, (void *)symbol)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module='%s', name='%s', ns='%s', version='%" PRIu32 ".%" PRIu32 ".%" PRIu32 "+%" PRIu64 "'",
           module->module_info->name, name, ns, version.major, version.minor, version.patch, version.build)
    ctx_lock_(ctx);
    const struct Symbol_ *sym = ctx_get_symbol_compatible_(ctx, name, ns, version);
    if (sym == NULL) {
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_SYM_;
        ERROR_(ctx, error,
               "could not find a compatible symbol, module='%s', name='%s', ns='%s', version='%" PRIu32 ".%" PRIu32
               ".%" PRIu32 "+%" PRIu64 "'",
               module->module_info->name, name, ns, version.major, version.minor, version.patch, version.build)
        return error;
    }

    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    const struct ModuleInfoDependency_ *dependency = module_info_get_dependency_(info_inner, sym->module);
    if (dependency == NULL) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_NOT_A_DEPENDENCY_;
        ERROR_(ctx, error,
               "module exposing the symbol is not a dependency, exposed_by='%s', module='%s', name='%s', ns='%s', "
               "version='%" PRIu32 ".%" PRIu32 ".%" PRIu32 "+%" PRIu64 "'",
               sym->module, module->module_info->name, name, ns, version.major, version.minor, version.patch,
               version.build)
        return error;
    }
    if (module_info_get_ns_(info_inner, ns) == NULL && strcmp(ns, GLOBAL_NS) != 0) {
        module_info_unlock_(info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_NS_NOT_INCLUDED_;
        ERROR_(ctx, error,
               "module does not include the namespace it tried to load a symbol from, module='%s', name='%s', ns='%s', "
               "version='%" PRIu32 ".%" PRIu32 ".%" PRIu32 "+%" PRIu64 "'",
               module->module_info->name, name, ns, version.major, version.minor, version.patch, version.build)
        return error;
    }

    const struct Module_ *symbol_owner = ctx_get_module_(ctx, sym->module);
    FIMO_DEBUG_ASSERT(symbol_owner)
    const struct ModuleInfo_ *symbol_owner_info = module_info_from_module_(symbol_owner->module);
    struct ModuleInfoInner_ *symbol_owner_info_inner = module_info_lock_(symbol_owner_info);
    const struct ModuleInfoSymbol_ *info_symbol = module_info_get_symbol_(symbol_owner_info_inner, name, ns, version);
    FIMO_DEBUG_ASSERT(info_symbol);
    *symbol = &info_symbol->symbol;

    module_info_unlock_(symbol_owner_info_inner);
    module_info_unlock_(info_inner);
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_unload(FimoInternalModuleContext *ctx, const FimoModuleInfo *module) {
    FIMO_DEBUG_ASSERT(ctx)

    ctx_lock_(ctx);
    if (module != NULL) {
        TRACE_(ctx, "unloading module, module='%s'", module->name)
        const struct ModuleInfo_ *info = module_info_from_module_info_(module);
        if (info->type != MODULE_TYPE_REGULAR_) {
            ctx_unlock_(ctx);
            ERROR_SIMPLE_(ctx, FIMO_EPERM, "can only unload regular modules")
            return FIMO_EPERM;
        }
        struct ModuleInfoInner_ *info_inner = module_info_lock_(info);

        const FimoResult error = ctx_remove_module_(ctx, info_inner);
        if (FIMO_RESULT_IS_ERROR(error)) {
            module_info_unlock_(info_inner);
            ctx_unlock_(ctx);
            ERROR_SIMPLE_(ctx, error, "could not remove module from context")
            return error;
        }

        fi_module_free_(info_inner, NULL);
    }

    const FimoResult error = ctx_cleanup_loose_modules(ctx);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_SIMPLE_(ctx, error, "could not cleanup loose modules")
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_query(FimoInternalModuleContext *ctx, const char *module_name, const char *param,
                                            FimoModuleParamType *type, FimoModuleParamAccess *read,
                                            FimoModuleParamAccess *write) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module_name == NULL || param == NULL || type == NULL || read == NULL || write == NULL) {
        ERROR_(ctx, FIMO_EINVAL,
               "invalid null parameter, module_name='%p', param='%p', type='%p', read='%p', write='%p'",
               (void *)module_name, (void *)param, (void *)type, (void *)read, (void *)write)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module_name='%s', param='%s'", module_name, param)
    ctx_lock_(ctx);
    const struct Module_ *module = ctx_get_module_(ctx, module_name);
    if (module == NULL) {
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "module does not exist, module='%s'", module_name)
        return error;
    }

    const struct ModuleInfo_ *module_info = module_info_from_module_(module->module);
    struct ModuleInfoInner_ *module_info_inner = module_info_lock_(module_info);
    const struct ModuleInfoParam_ *module_param = module_info_get_param_(module_info_inner, param);
    if (module_param == NULL) {
        module_info_unlock_(module_info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_PARAM_;
        ERROR_(ctx, error, "parameter not found, module='%s', param='%s'", module_name, param)
        return error;
    }

    *type = module_param->param->data.type;
    *read = module_param->param->read;
    *write = module_param->param->write;

    module_info_unlock_(module_info_inner);
    ctx_unlock_(ctx);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_set_public(FimoInternalModuleContext *ctx, const void *value,
                                                 const FimoModuleParamType type, const char *module_name,
                                                 const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (value == NULL || module_name == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, value='%p', module_name='%p', param='%p'", (void *)value,
               (void *)module_name, (void *)param)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module_name='%s', param='%s'", module_name, param)
    ctx_lock_(ctx);
    const struct Module_ *module = ctx_get_module_(ctx, module_name);
    if (module == NULL) {
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "module does not exist, module='%s'", module_name)
        return error;
    }

    const struct ModuleInfo_ *module_info = module_info_from_module_(module->module);
    struct ModuleInfoInner_ *module_info_inner = module_info_lock_(module_info);
    const struct ModuleInfoParam_ *module_param = module_info_get_param_(module_info_inner, param);
    if (module_param == NULL) {
        module_info_unlock_(module_info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_PARAM_;
        ERROR_(ctx, error, "parameter not found, module='%s', param='%s'", module_name, param)
        return error;
    }

    if (!param_can_write_public(module_param->param)) {
        module_info_unlock_(module_info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_NO_WRITE_PERMISSION_;
        ERROR_(ctx, error, "write not permitted, module='%s', param='%s'", module_name, param)
        return error;
    }

    const FimoResult error = param_write_((FimoModuleParam *)module_param->param, module->module, value, type);
    module_info_unlock_(module_info_inner);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not write to param, module='%s', param='%s'", module_name, param)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_get_public(FimoInternalModuleContext *ctx, void *value, FimoModuleParamType *type,
                                                 const char *module_name, const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (value == NULL || type == NULL || module_name == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, value='%p', type='%p', module_name='%p', param='%p'",
               (void *)value, (void *)type, (void *)module_name, (void *)param)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module_name='%s', param='%s'", module_name, param)
    ctx_lock_(ctx);
    const struct Module_ *module = ctx_get_module_(ctx, module_name);
    if (module == NULL) {
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_MOD_;
        ERROR_(ctx, error, "module does not exist, module='%s'", module_name)
        return error;
    }

    const struct ModuleInfo_ *module_info = module_info_from_module_(module->module);
    struct ModuleInfoInner_ *module_info_inner = module_info_lock_(module_info);
    const struct ModuleInfoParam_ *module_param = module_info_get_param_(module_info_inner, param);
    if (module_param == NULL) {
        module_info_unlock_(module_info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_MISSING_PARAM_;
        ERROR_(ctx, error, "parameter not found, module='%s', param='%s'", module_name, param)
        return error;
    }

    if (!param_can_read_public(module_param->param)) {
        module_info_unlock_(module_info_inner);
        ctx_unlock_(ctx);
        FimoResult error = ERR_NO_READ_PERMISSION_;
        ERROR_(ctx, error, "read not permitted, module='%s', param='%s'", module_name, param)
        return error;
    }

    const FimoResult error = param_read_(module_param->param, module->module, value, type);
    module_info_unlock_(module_info_inner);
    ctx_unlock_(ctx);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not read from param, module='%s', param='%s'", module_name, param)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_set_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                     const void *value, const FimoModuleParamType type,
                                                     const char *module_name, const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || value == NULL || module_name == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', value='%p', module_name='%p', param='%p'",
               (void *)module, (void *)value, (void *)module_name, (void *)param)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module='%p', module_name='%s', param='%s'", (void *)module, module_name, param)
    const struct ModuleInfo_ *caller_info = module_info_from_module_(module);
    struct ModuleInfoInner_ *caller_info_inner = module_info_lock_(caller_info);
    const struct ModuleInfoDependency_ *dep = module_info_get_dependency_(caller_info_inner, module_name);
    if (dep == NULL) {
        module_info_unlock_(caller_info_inner);
        FimoResult error = ERR_NOT_A_DEPENDENCY_;
        ERROR_(ctx, error, "module is not a dependency, module='%s', caller='%s'", module_name,
               module->module_info->name)
        return error;
    }

    const struct ModuleInfo_ *dep_info = module_info_from_module_info_(dep->info);
    struct ModuleInfoInner_ *dep_info_inner = module_info_lock_(dep_info);
    const struct ModuleInfoParam_ *dep_param = module_info_get_param_(dep_info_inner, param);
    if (dep_param == NULL) {
        module_info_unlock_(dep_info_inner);
        module_info_unlock_(caller_info_inner);
        FimoResult error = ERR_MISSING_PARAM_;
        ERROR_(ctx, error, "parameter not found, module='%s', parameter='%s'", module_name, param)
        return error;
    }

    if (!param_can_write_dependency(dep_param->param, caller_info_inner)) {
        module_info_unlock_(dep_info_inner);
        module_info_unlock_(caller_info_inner);
        FimoResult error = ERR_NO_WRITE_PERMISSION_;
        ERROR_(ctx, error, "write not permitted, caller='%s', module='%s', parameter='%s'", module->module_info->name,
               module_name, param)
        return error;
    }

    FIMO_ASSERT(!module_info_is_detached_(dep_info_inner) && dep_info_inner->module)
    const FimoResult error = param_write_((FimoModuleParam *)dep_param->param, dep_info_inner->module, value, type);
    module_info_unlock_(dep_info_inner);
    module_info_unlock_(caller_info_inner);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not write to param, module='%s', param='%s'", module_name, param)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_get_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                     void *value, FimoModuleParamType *type, const char *module_name,
                                                     const char *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || value == NULL || type == NULL || module_name == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL,
               "invalid null parameter, module='%p', value='%p', type='%p', module_name='%p', param='%p'",
               (void *)module, value, (void *)type, (void *)module_name, (void *)param)
        return FIMO_EINVAL;
    }

    TRACE_(ctx, "module='%p', module_name='%s', param='%s'", (void *)module, module_name, param)
    const struct ModuleInfo_ *caller_info = module_info_from_module_(module);
    struct ModuleInfoInner_ *caller_info_inner = module_info_lock_(caller_info);
    const struct ModuleInfoDependency_ *dep = module_info_get_dependency_(caller_info_inner, module_name);
    if (dep == NULL) {
        module_info_unlock_(caller_info_inner);
        FimoResult error = ERR_NOT_A_DEPENDENCY_;
        ERROR_(ctx, error, "module is not a dependency, module='%s', caller='%s'", module_name,
               module->module_info->name)
        return error;
    }

    const struct ModuleInfo_ *dep_info = module_info_from_module_info_(dep->info);
    struct ModuleInfoInner_ *dep_info_inner = module_info_lock_(dep_info);
    const struct ModuleInfoParam_ *dep_param = module_info_get_param_(dep_info_inner, param);
    if (dep_param == NULL) {
        module_info_unlock_(dep_info_inner);
        module_info_unlock_(caller_info_inner);
        FimoResult error = ERR_MISSING_PARAM_;
        ERROR_(ctx, error, "parameter not found, module='%s', parameter='%s'", module_name, param)
        return error;
    }

    if (!param_can_read_dependency(dep_param->param, caller_info_inner)) {
        module_info_unlock_(dep_info_inner);
        module_info_unlock_(caller_info_inner);
        FimoResult error = ERR_NO_READ_PERMISSION_;
        ERROR_(ctx, error, "read not permitted, caller='%s', module='%s', parameter='%s'", module->module_info->name,
               module_name, param)
        return error;
    }

    FIMO_ASSERT(!module_info_is_detached_(dep_info_inner) && dep_info_inner->module)
    const FimoResult error = param_read_(dep_param->param, dep_info_inner->module, value, type);

    module_info_unlock_(dep_info_inner);
    module_info_unlock_(caller_info_inner);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not read from param, module='%s', param='%s'", module_name, param)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_set_private(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                  const void *value, const FimoModuleParamType type,
                                                  FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || value == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', value='%p', type='%d', param='%p'",
               (void *)module, value, type, (void *)param)
        return FIMO_EINVAL;
    }

    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    TRACE_(ctx, "module='%p', param='%p', owner='%p', read='%d', write='%d', type='%d'", (void *)module, (void *)param,
           (void *)param->data.owner, param->read, param->write, param->data.type)
    if (!param_can_write_private(param, module)) {
        FimoResult error = ERR_NO_WRITE_PERMISSION_;
        ERROR_(ctx, error, "write not permitted, caller='%s', module='%s'", module->module_info->name,
               param->data.owner->module_info->name)
        return error;
    }

    const FimoResult error = param_write_(param, module, value, type);
    module_info_unlock_(info_inner);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not write to param, module='%s'", module->module_info->name)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_get_private(FimoInternalModuleContext *ctx, const FimoModule *module, void *value,
                                                  FimoModuleParamType *type, const FimoModuleParam *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || value == NULL || type == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', value='%p', type='%p', param='%p'",
               (void *)module, (void *)value, (void *)type, (void *)param)
        return FIMO_EINVAL;
    }

    const struct ModuleInfo_ *info = module_info_from_module_(module);
    struct ModuleInfoInner_ *info_inner = module_info_lock_(info);
    TRACE_(ctx, "module='%p', param='%p', owner='%p', read='%d', write='%d', type='%d'", (void *)module, (void *)param,
           (void *)param->data.owner, param->read, param->write, param->data.type)
    if (!param_can_read_private(param, module)) {
        FimoResult error = ERR_NO_READ_PERMISSION_;
        ERROR_(ctx, error, "read not permitted, caller='%s', module='%s'", module->module_info->name,
               param->data.owner->module_info->name)
        return error;
    }

    const FimoResult error = param_read_(param, module, value, type);
    module_info_unlock_(info_inner);
    if (FIMO_RESULT_IS_ERROR(error)) {
        ERROR_(ctx, error, "could not read from param, module='%s'", module->module_info->name)
        return error;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_set_inner(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                const void *value, const FimoModuleParamType type,
                                                FimoModuleParamData *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || value == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', value='%p', type='%d', param='%p'",
               (void *)module, value, type, (void *)param)
        return FIMO_EINVAL;
    }

    struct ParamData_ *data = (struct ParamData_ *)param;
    TRACE_(ctx, "module='%p', param='%p', owner='%p', type='%d'", (void *)module, (void *)data, (void *)data->owner,
           data->type)

    if (!param_data_is_owner_(data, module)) {
        FimoResult error = ERR_NO_READ_PERMISSION_;
        ERROR_(ctx, error, "read not permitted, caller='%s', owner='%s'", module->module_info->name,
               data->owner->module_info->name)
        return error;
    }

    if (!param_data_type_matches_(data, type)) {
        FimoResult error = ERR_PARAM_TYPE_;
        ERROR_(ctx, error, "invalid parameter type, required='%d', got='%d'", data->type, type)
        return error;
    }

    param_data_write_(data, value);
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoResult fimo_internal_module_param_get_inner(FimoInternalModuleContext *ctx, const FimoModule *module, void *value,
                                                FimoModuleParamType *type, const FimoModuleParamData *param) {
    FIMO_DEBUG_ASSERT(ctx)
    if (module == NULL || value == NULL || type == NULL || param == NULL) {
        ERROR_(ctx, FIMO_EINVAL, "invalid null parameter, module='%p', value='%p', type='%p', param='%p'",
               (void *)module, (void *)value, (void *)type, (void *)param)
        return FIMO_EINVAL;
    }

    const struct ParamData_ *data = (const struct ParamData_ *)param;
    TRACE_(ctx, "module='%p', param='%p', owner='%p', type='%d'", (void *)module, (void *)data, (void *)data->owner,
           data->type)
    if (!param_data_is_owner_(data, module)) {
        FimoResult error = ERR_NO_READ_PERMISSION_;
        ERROR_(ctx, error, "read not permitted, caller='%s', module='%s'", module->module_info->name,
               data->owner->module_info->name)
        return error;
    }

    param_data_read_(data, value, type);
    return FIMO_EOK;
}
