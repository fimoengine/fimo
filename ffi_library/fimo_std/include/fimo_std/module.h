#ifndef FIMO_MODULE_H
#define FIMO_MODULE_H

#include <assert.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/version.h>

#include <fimo_std/impl/module.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Constructs a new `FimoU8` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_U8_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                         \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U8, .read_access = READ, .write_access = WRITE, .setter = SETTER,               \
        .getter = GETTER, .name = NAME, .default_value = {.u8 = VALUE},                                                \
    }

/**
 * Constructs a new `FimoU16` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_U16_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                        \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U16, .read_access = READ, .write_access = WRITE, .setter = SETTER,              \
        .getter = GETTER, .name = NAME, .default_value = {.u16 = VALUE},                                               \
    }

/**
 * Constructs a new `FimoU32` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_U32_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                        \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U32, .read_access = READ, .write_access = WRITE, .setter = SETTER,              \
        .getter = GETTER, .name = NAME, .default_value = {.u32 = VALUE},                                               \
    }

/**
 * Constructs a new `FimoU64` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_U64_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                        \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U64, .read_access = READ, .write_access = WRITE, .setter = SETTER,              \
        .getter = GETTER, .name = NAME, .default_value = {.u64 = VALUE},                                               \
    }

/**
 * Constructs a new `FimoI8` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_I8_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                         \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_I8, .read_access = READ, .write_access = WRITE, .setter = SETTER,               \
        .getter = GETTER, .name = NAME, .default_value = {.i8 = VALUE},                                                \
    }

/**
 * Constructs a new `FimoI16` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_I16_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                        \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U16, .read_access = READ, .write_access = WRITE, .setter = SETTER,              \
        .getter = GETTER, .name = NAME, .default_value = {.i16 = VALUE},                                               \
    }

/**
 * Constructs a new `FimoI32` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_I32_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                        \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U32, .read_access = READ, .write_access = WRITE, .setter = SETTER,              \
        .getter = GETTER, .name = NAME, .default_value = {.i32 = VALUE},                                               \
    }

/**
 * Constructs a new `FimoI64` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 * @param SETTER parameter setter
 * @param GETTER parameter getter
 */
#define FIMO_MODULE_PARAM_I64_COMPLEX(NAME, VALUE, READ, WRITE, SETTER, GETTER)                                        \
    {                                                                                                                  \
        .type = FIMO_MODULE_PARAM_TYPE_U64, .read_access = READ, .write_access = WRITE, .setter = SETTER,              \
        .getter = GETTER, .name = NAME, .default_value = {.i64 = VALUE},                                               \
    }

/**
 * Constructs a new `FimoU8` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_U8(NAME, VALUE, READ, WRITE)                                                                 \
    FIMO_MODULE_PARAM_U8_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoU16` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_U16(NAME, VALUE, READ, WRITE)                                                                \
    FIMO_MODULE_PARAM_U16_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoU32` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_U32(NAME, VALUE, READ, WRITE)                                                                \
    FIMO_MODULE_PARAM_U32_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoU64` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_U64(NAME, VALUE, READ, WRITE)                                                                \
    FIMO_MODULE_PARAM_U64_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI8` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_I8(NAME, VALUE, READ, WRITE)                                                                 \
    FIMO_MODULE_PARAM_I8_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI16` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_I16(NAME, VALUE, READ, WRITE)                                                                \
    FIMO_MODULE_PARAM_I16_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI32` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_I32(NAME, VALUE, READ, WRITE)                                                                \
    FIMO_MODULE_PARAM_I32_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI64` parameter declaration.
 *
 * @param NAME parameter name
 * @param VALUE default value
 * @param READ read access group
 * @param WRITE write access group
 */
#define FIMO_MODULE_PARAM_I64(NAME, VALUE, READ, WRITE)                                                                \
    FIMO_MODULE_PARAM_I64_COMPLEX(NAME, VALUE, READ, WRITE, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new resource declaration.
 *
 * @param PATH path to the resource relative to the module directory
 */
#define FIMO_MODULE_RESOURCE(PATH)                                                                                     \
    { .path = PATH }

/**
 * Constructs a new namespace import declaration.
 *
 * @param NS namespace name
 */
#define FIMO_MODULE_IMPORT_NAMESPACE(NS)                                                                               \
    { .name = NS }

/**
 * Constructs a new symbol import declaration.
 *
 * @param NAME symbol name
 * @param NS symbol namespace
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 */
#define FIMO_MODULE_IMPORT_SYMBOL_NS(NAME, NS, V_MAJOR, V_MINOR, V_PATCH)                                              \
    { .version = FIMO_VERSION(V_MAJOR, V_MINOR, V_PATCH), .name = NAME, .ns = NS }

/**
 * Constructs a new symbol import declaration.
 *
 * @param NAME symbol name
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 */
#define FIMO_MODULE_IMPORT_SYMBOL(NAME, V_MAJOR, V_MINOR, V_PATCH)                                                     \
    FIMO_MODULE_IMPORT_SYMBOL_NS(NAME, "", V_MAJOR, V_MINOR, V_PATCH)

/**
 * Constructs a new symbol export declaration for a variable.
 *
 * @param NAME symbol name
 * @param NS symbol namespace
 * @param VAR variable to export
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 */
#define FIMO_MODULE_EXPORT_SYMBOL_VAR_NS(NAME, NS, VAR, V_MAJOR, V_MINOR, V_PATCH)                                     \
    { .symbol = (void *)&VAR, .version = FIMO_VERSION(V_MAJOR, V_MINOR, V_PATCH), .name = NAME, .ns = NS }

/**
 * Constructs a new symbol export declaration for a function.
 *
 * @param NAME symbol name
 * @param NS symbol namespace
 * @param FUNC function to export
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 */
#define FIMO_MODULE_EXPORT_SYMBOL_FUNC_NS(NAME, NS, FUNC, V_MAJOR, V_MINOR, V_PATCH)                                   \
    { .symbol = *(void **)&FUNC, .version = FIMO_VERSION(V_MAJOR, V_MINOR, V_PATCH), .name = NAME, .ns = NS }

/**
 * Constructs a new symbol export declaration for a variable.
 *
 * @param NAME symbol name
 * @param VAR variable to export
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 */
#define FIMO_MODULE_EXPORT_SYMBOL_VAR(NAME, VAR, V_MAJOR, V_MINOR, V_PATCH)                                            \
    FIMO_MODULE_EXPORT_SYMBOL_VAR_NS(NAME, "", VAR, V_MAJOR, V_MINOR, V_PATCH)

/**
 * Constructs a new symbol export declaration for a function.
 *
 * @param NAME symbol name
 * @param FUNC function to export
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 */
#define FIMO_MODULE_EXPORT_SYMBOL_FUNC(NAME, FUNC, V_MAJOR, V_MINOR, V_PATCH)                                          \
    FIMO_MODULE_EXPORT_SYMBOL_FUNC_NS(NAME, "", FUNC, V_MAJOR, V_MINOR, V_PATCH)

/**
 * Constructs a new dynamic symbol export declaration.
 *
 * @param NAME symbol name
 * @param NS symbol namespace
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 * @param CONSTR constructor
 * @param DESTR destructor
 */
#define FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL_NS(NAME, NS, V_MAJOR, V_MINOR, V_PATCH, CONSTR, DESTR)                       \
    {                                                                                                                  \
        .constructor = CONSTR, .destructor = DESTR, .version = FIMO_VERSION(V_MAJOR, V_MINOR, V_PATCH), .name = NAME,  \
        .ns = NS                                                                                                       \
    }

/**
 * Constructs a new dynamic symbol export declaration.
 *
 * @param NAME symbol name
 * @param V_MAJOR major version of the symbol
 * @param V_MINOR minor version of the symbol
 * @param V_PATCH patch version of the symbol
 * @param CONSTR constructor
 * @param DESTR destructor
 */
#define FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL(NAME, V_MAJOR, V_MINOR, V_PATCH, CONSTR, DESTR)                              \
    FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL_NS(NAME, "", V_MAJOR, V_MINOR, V_PATCH, CONSTR, DESTR)

/**
 * ABI version of the current module export.
 */
#define FIMO_MODULE_EXPORT_ABI 0

#ifdef _WIN32
#define FIMO_MODULE_EXPORT_MODULE__(VAR)                                                                               \
    __declspec(allocate(FIMO_IMPL_MODULE_SECTION)) const FimoModuleExport *FIMO_VAR(VAR) = &VAR;
#else
#define FIMO_MODULE_EXPORT_MODULE__(VAR)                                                                               \
    const FimoModuleExport *FIMO_VAR(VAR) __attribute__((retain, used, section(FIMO_IMPL_MODULE_SECTION))) = &VAR;
#endif

#define FIMO_MODULE_EXPORT_MODULE_(VAR, NAME, DESC, AUTHOR, LICENSE, ...)                                              \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wmissing-field-initializers")                                             \
    FimoModuleExport VAR = {.type = FIMO_STRUCT_TYPE_MODULE_EXPORT,                                                    \
                            .next = NULL,                                                                              \
                            .export_abi = FIMO_MODULE_EXPORT_ABI,                                                      \
                            .name = NAME,                                                                              \
                            .description = DESC,                                                                       \
                            .author = AUTHOR,                                                                          \
                            .license = LICENSE,                                                                        \
                            __VA_ARGS__};                                                                              \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)                                                                                \
    FIMO_MODULE_EXPORT_MODULE__(VAR)

/**
 * Exports a new module.
 *
 * @param NAME module name (not NULL)
 * @param DESC module description
 * @param AUTHOR module author
 * @param LICENSE module license
 */
#define FIMO_MODULE_EXPORT_MODULE(NAME, DESC, AUTHOR, LICENSE, ...)                                                    \
    FIMO_MODULE_EXPORT_MODULE_(FIMO_VAR(fimo_module_export_private), NAME, DESC, AUTHOR, LICENSE, __VA_ARGS__)

/**
 * Specifies the module parameters.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param PARAM_LIST array of `FimoModuleParamDecl`
 */
#define FIMO_MODULE_EXPORT_MODULE_PARAMS(PARAM_LIST)                                                                   \
    .parameters = PARAM_LIST, .parameters_count = (FimoU32)(sizeof(PARAM_LIST) / sizeof(FimoModuleParamDecl))

/**
 * Specifies the module resources.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param RESOURCE_LIST array of `FimoModuleResourceDecl`
 */
#define FIMO_MODULE_EXPORT_MODULE_RESOURCES(RESOURCE_LIST)                                                             \
    .resources = RESOURCE_LIST, .resources_count = (FimoU32)(sizeof(RESOURCE_LIST) / sizeof(FimoModuleResourceDecl))

/**
 * Specifies the module namespace imports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param NS_LIST array of `FimoModuleNamespaceImport`
 */
#define FIMO_MODULE_EXPORT_MODULE_NAMESPACES(NS_LIST)                                                                  \
    .namespace_imports = NS_LIST,                                                                                      \
    .namespace_imports_count = (FimoU32)(sizeof(NS_LIST) / sizeof(FimoModuleNamespaceImport))

/**
 * Specifies the module symbol imports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param SYM_LIST array of `FimoModuleSymbolImport`
 */
#define FIMO_MODULE_EXPORT_MODULE_SYMBOL_IMPORTS(SYM_LIST)                                                             \
    .symbol_imports = SYM_LIST, .symbol_imports_count = (FimoU32)(sizeof(SYM_LIST) / sizeof(FimoModuleSymbolImport))

/**
 * Specifies the static module symbol exports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param SYM_LIST array of `FimoModuleSymbolExport`
 */
#define FIMO_MODULE_EXPORT_MODULE_SYMBOL_EXPORTS(SYM_LIST)                                                             \
    .symbol_exports = SYM_LIST, .symbol_exports_count = (FimoU32)(sizeof(SYM_LIST) / sizeof(FimoModuleSymbolExport))

/**
 * Specifies the dynamic module symbol exports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param SYM_LIST array of `FimoModuleDynamicSymbolExport`
 */
#define FIMO_MODULE_EXPORT_MODULE_DYNAMIC_SYMBOL_EXPORTS(SYM_LIST)                                                     \
    .dynamic_symbol_exports = SYM_LIST,                                                                                \
    .dynamic_symbol_exports_count = (FimoU32)(sizeof(SYM_LIST) / sizeof(FimoModuleDynamicSymbolExport))

/**
 * Specifies the modifiers of the module export.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param MOD_LIST array of `FimoModuleExportModifier`
 */
#define FIMO_MODULE_EXPORT_MODULE_MODIFIERS(MOD_LIST)                                                                  \
    .modifiers = MOD_LIST, .modifiers_count = (FimoU32)(sizeof(MOD_LIST) / sizeof(FimoModuleExportModifier))

/**
 * Specifies the constructor and destructor of a module.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param CONSTR constructor function
 * @param DESTR destructor function
 */
#define FIMO_MODULE_EXPORT_MODULE_CONSTRUCTOR(CONSTR, DESTR) .module_constructor = CONSTR, .module_destructor = DESTR,

/**
 * Declares a new parameter table.
 *
 * For compatibility with C++, the table must contain at least one
 * element. Use `FIMO_MODULE_PARAM_TABLE_EMPTY` in that case. Otherwise,
 * the members of the struct must be declared with calls to
 * `FIMO_MODULE_PARAM_TABLE_PARAM`. The params must be declared in the
 * same order as they are specified in the module.
 *
 * @param NAME parameter table name
 * @param PARAM_COUNT number of parameters in the table
 * @param DECL table struct declaration
 */
#define FIMO_MODULE_PARAM_TABLE(NAME, PARAM_COUNT, DECL)                                                               \
    DECL;                                                                                                              \
    static_assert(alignof(NAME) == alignof(FimoModuleParam *), "Unexpected padding in module param table");            \
    static_assert(sizeof(NAME) > 0, "Unexpected size of module param table");                                          \
    static_assert(sizeof(NAME) == PARAM_COUNT * sizeof(FimoModuleParam *), "Unexpected size of module param table");

/**
 * Declares a placeholder param.
 */
#define FIMO_MODULE_PARAM_TABLE_EMPTY FimoModuleParam *empty__

/**
 * Declares a new parameter for the param table.
 *
 * @param NAME name of the parameter
 */
#define FIMO_MODULE_PARAM_TABLE_PARAM(NAME) FimoModuleParam *NAME

/**
 * Declares a new resource table.
 *
 * For compatibility with C++, the table must contain at least one
 * element. Use `FIMO_MODULE_RESOURCE_TABLE_EMPTY` in that case. Otherwise,
 * the members of the struct must be declared with calls to
 * `FIMO_MODULE_RESOURCE_TABLE_PARAM`. The resources must be declared
 * in the same order as they are specified in the module.
 *
 * @param NAME symbol table name
 * @param SYM_COUNT number of symbols in the table
 * @param DECL table struct declaration
 */
#define FIMO_MODULE_RESOURCE_TABLE(NAME, RES_COUNT, DECL)                                                              \
    DECL;                                                                                                              \
    static_assert(alignof(NAME) == alignof(const char *), "Unexpected padding in module resource table");              \
    static_assert(sizeof(NAME) > 0, "Unexpected size of module resource table");                                       \
    static_assert(sizeof(NAME) == RES_COUNT * sizeof(const char *), "Unexpected size of module resource table");

/**
 * Declares a placeholder param.
 */
#define FIMO_MODULE_RESOURCE_TABLE_EMPTY const char *empty__


/**
 * Declares a new resource for the resource table.
 *
 * @param NAME name of the resource
 */
#define FIMO_MODULE_RESOURCE_TABLE_PARAM(NAME) const char *NAME

/**
 * Declares a new symbol table.
 *
 * For compatibility with C++, the table must contain at least one
 * element. Use `FIMO_MODULE_SYMBOL_TABLE_EMPTY` in that case. Otherwise,
 * the members of the struct must be declared with calls to
 * `FIMO_MODULE_SYMBOL_TABLE_VAR` and `FIMO_MODULE_SYMBOL_TABLE_FUNC.
 * The symbols must be declared in the same order as they are specified
 * in the module.
 *
 * @param NAME symbol table name
 * @param SYM_COUNT number of symbols in the table
 * @param DECL table struct declaration
 */
#define FIMO_MODULE_SYMBOL_TABLE(NAME, SYM_COUNT, DECL)                                                                \
    DECL;                                                                                                              \
    static_assert(alignof(NAME) == alignof(const FimoModuleRawSymbol *), "Unexpected padding in module symbol table"); \
    static_assert(sizeof(NAME) > 0, "Unexpected size of module symbol table");                                         \
    static_assert(sizeof(NAME) == SYM_COUNT * sizeof(const FimoModuleRawSymbol *), "Unexpected size of module symbol " \
                                                                                   "table");

/**
 * Declares a placeholder symbol.
 */
#define FIMO_MODULE_SYMBOL_TABLE_EMPTY FimoModuleRawSymbol empty__;

/**
 * Declares a new variable symbol for the symbol table.
 *
 * @param NAME name of the symbol in the symbol table
 * @param TYPE type of the symbol
 */
#define FIMO_MODULE_SYMBOL_TABLE_VAR(NAME, TYPE)                                                                       \
    const struct {                                                                                                     \
        const TYPE *data;                                                                                              \
        _Atomic(FimoUSize) lock;                                                                                       \
    } *NAME;                                                                                                           \
    static_assert(sizeof(const TYPE *) == sizeof(const void *), "Unexpected symbol size");                             \
    static_assert(alignof(const TYPE *) == alignof(const void *), "Unexpected symbol alignment")

/**
 * Declares a new function symbol for the symbol table.
 *
 * @param NAME name of the symbol in the symbol table
 * @param RET function return type
 * @param ARGS function parameter list
 */
#define FIMO_MODULE_SYMBOL_TABLE_FUNC(NAME, RET, ...)                                                                  \
    const struct {                                                                                                     \
        const RET (*data)(__VA_ARGS__);                                                                                \
        _Atomic(FimoUSize) lock;                                                                                       \
    } *NAME;                                                                                                           \
    static_assert(sizeof(RET(*const)(__VA_ARGS__)) == sizeof(const void *), "Unexpected symbol size");                 \
    static_assert(alignof(RET(*const)(__VA_ARGS__)) == alignof(const void *), "Unexpected symbol alignment")

/**
 * Locks a symbol and returns it.
 *
 * @param SYMBOL a symbol
 */
#define FIMO_MODULE_SYMBOL_LOCK(SYMBOL)                                                                                \
    (fimo_impl_module_symbol_acquire((_Atomic(FimoUSize) *)&(SYMBOL)->lock), (SYMBOL)->data)

/**
 * Unlocks a symbol.
 *
 * @param SYMBOL a symbol
 */
#define FIMO_MODULE_SYMBOL_RELEASE(SYMBOL) fimo_impl_module_symbol_release((_Atomic(FimoUSize) *)&(SYMBOL)->lock)

/**
 * Checks whether the symbol is locked.
 *
 * @param SYMBOL a symbol
 */
#define FIMO_MODULE_SYMBOL_IS_LOCKED(SYMBOL) fimo_impl_module_symbol_is_used((_Atomic(FimoUSize) *)&(SYMBOL)->lock)

/**
 * Acquires a reference to a `FimoModuleInfo`.
 *
 * @param INFO pointer to a `FimoModuleInfo`
 */
#define FIMO_MODULE_INFO_ACQUIRE(INFO) (fimo_impl_module_info_acquire(INFO), INFO)

/**
 * Releases a reference to a `FimoModuleInfo`.
 *
 * @param INFO pointer to a `FimoModuleInfo`
 */
#define FIMO_MODULE_INFO_RELEASE(INFO) fimo_impl_module_info_release(INFO)

/**
 * Checks whether a module is loaded.
 *
 * @param INFO pointer to a `FimoModuleInfo`
 */
#define FIMO_MODULE_INFO_IS_LOADED(INFO) fimo_impl_module_info_is_loaded(INFO)

/**
 * Locks the module from being unloaded.
 *
 * A module may be locked recursively. Each call to lock must be paired with
 * a call to unlock.
 *
 * @param INFO pointer to a `FimoModuleInfo`
 */
#define FIMO_MODULE_INFO_LOCK_UNLOAD(INFO) fimo_impl_module_info_lock_unload(INFO)

/**
 * Unlocks a module, allowing it to be unloaded.
 *
 * @param INFO pointer to a `FimoModuleInfo`
 */
#define FIMO_MODULE_INFO_UNLOCK_UNLOAD(INFO) fimo_impl_module_info_unlock_unload(INFO)

typedef struct FimoModule FimoModule;

/**
 * Constructor function for a dynamic symbol.
 *
 * The constructor is in charge of constructing an instance of
 * a symbol. To that effect, it is provided  an instance to the
 * module. The resulting symbol is written into the last argument.
 *
 * @param arg0 pointer to the module
 * @param arg1 pointer to the resulting symbol
 *
 * @return Status code.
 */
typedef FimoResult (*FimoModuleDynamicSymbolConstructor)(const FimoModule *arg0, void **arg1);

/**
 * Destructor function for a dynamic symbol.
 *
 * The destructor is safe to assume, that the symbol is no longer
 * used by any other module. During its destruction, a symbol is
 * not allowed to access the module backend.
 *
 * @param arg0 symbol to destroy
 */
typedef void (*FimoModuleDynamicSymbolDestructor)(void *arg0);

/**
 * Type-erased set of modules to load by the backend.
 */
typedef struct FimoModuleLoadingSet FimoModuleLoadingSet;

/**
 * Constructor function for a module.
 *
 * The module constructor allows a module implementor to initialize
 * some module specific data at module load time. Some use cases for
 * module constructors are initialization of global module data, or
 * fetching optional symbols. Returning an error aborts the loading
 * of the module. Is called before the symbols of the modules are
 * exported/initialized.
 *
 * @param arg0 pointer to the partially initialized module
 * @param arg1 module set that contained the module
 * @param arg2 pointer to the resulting module data
 *
 * @return Status code.
 */
typedef FimoResult (*FimoModuleConstructor)(const FimoModule *arg0, FimoModuleLoadingSet *arg1, void **arg2);

/**
 * Destructor function for a module.
 *
 * During its destruction, a module is not allowed to access the
 * module backend.
 *
 * @param arg0 pointer to the module
 * @param arg1 module data to destroy
 */
typedef void (*FimoModuleDestructor)(const FimoModule *arg0, void *arg1);

/**
 * Data type of a module parameter.
 */
typedef enum FimoModuleParamType {
    FIMO_MODULE_PARAM_TYPE_U8,
    FIMO_MODULE_PARAM_TYPE_U16,
    FIMO_MODULE_PARAM_TYPE_U32,
    FIMO_MODULE_PARAM_TYPE_U64,
    FIMO_MODULE_PARAM_TYPE_I8,
    FIMO_MODULE_PARAM_TYPE_I16,
    FIMO_MODULE_PARAM_TYPE_I32,
    FIMO_MODULE_PARAM_TYPE_I64,
} FimoModuleParamType;

/**
 * Access group for a module parameter.
 */
typedef enum FimoModuleParamAccess {
    FIMO_MODULE_PARAM_ACCESS_PUBLIC,
    FIMO_MODULE_PARAM_ACCESS_DEPENDENCY,
    FIMO_MODULE_PARAM_ACCESS_PRIVATE,
} FimoModuleParamAccess;

/**
 * A type-erased module parameter.
 */
typedef struct FimoModuleParam FimoModuleParam;

/**
 * A type-erased internal data type for a module parameter.
 */
typedef struct FimoModuleParamData FimoModuleParamData;

/**
 * Setter for a module parameter.
 *
 * The setter can perform some validation before the parameter is set.
 * If the setter produces an error, the parameter won't be modified.
 *
 * @param arg0 pointer to the module
 * @param arg1 pointer to the new value
 * @param arg2 type of the value
 * @param arg3 data of the parameter
 *
 * @return Status code.
 */
typedef FimoResult (*FimoModuleParamSet)(const FimoModule *arg0, const void *arg1, FimoModuleParamType arg2,
                                         FimoModuleParamData *arg3);

/**
 * Getter for a module parameter.
 *
 * @param arg0 pointer to the module
 * @param arg1 buffer to store the value into
 * @param arg2 buffer to store the type of the value into
 * @param arg3 data of the parameter
 *
 * @return Status code.
 */
typedef FimoResult (*FimoModuleParamGet)(const FimoModule *arg0, void *arg1, FimoModuleParamType *arg2,
                                         const FimoModuleParamData *arg3);

/**
 * Declaration of a module parameter.
 */
typedef struct FimoModuleParamDecl {
    /**
     * Type of the parameter.
     */
    FimoModuleParamType type;
    /**
     * Access group specifier for the read permission.
     */
    FimoModuleParamAccess read_access;
    /**
     * Access group specifier for the write permission.
     */
    FimoModuleParamAccess write_access;
    /**
     * Setter function for the parameter.
     */
    FimoModuleParamSet setter;
    /**
     * Getter function for the parameter.
     */
    FimoModuleParamGet getter;
    /**
     * Name of the parameter.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Default value of the parameter.
     */
    union {
        FimoU8 u8;
        FimoU16 u16;
        FimoU32 u32;
        FimoU64 u64;
        FimoI8 i8;
        FimoI16 i16;
        FimoI32 i32;
        FimoI64 i64;
    } default_value;
} FimoModuleParamDecl;

/**
 * Declaration of a module resource.
 */
typedef struct FimoModuleResourceDecl {
    /**
     * Resource path relative to the module directory.
     *
     * Must not be `NULL` or begin with a slash.
     */
    const char *path;
} FimoModuleResourceDecl;

/**
 * Declaration of a module namespace import.
 */
typedef struct FimoModuleNamespaceImport {
    /**
     * Imported namespace.
     *
     * Must not be `NULL`.
     */
    const char *name;
} FimoModuleNamespaceImport;

/**
 * Declaration of a module symbol import.
 */
typedef struct FimoModuleSymbolImport {
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Symbol namespace.
     *
     * Must not be `NULL`.
     */
    const char *ns;
} FimoModuleSymbolImport;

/**
 * Declaration of a static module symbol export.
 */
typedef struct FimoModuleSymbolExport {
    /**
     * Pointer to the symbol.
     */
    const void *symbol;
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Symbol namespace.
     *
     * Must not be `NULL`.
     */
    const char *ns;
} FimoModuleSymbolExport;

/**
 * Declaration of a dynamic module symbol export.
 */
typedef struct FimoModuleDynamicSymbolExport {
    /**
     * Symbol constructor.
     *
     * Must not be `NULL`.
     */
    FimoModuleDynamicSymbolConstructor constructor;
    /**
     * Symbol destructor.
     *
     * Must not be `NULL`.
     */
    FimoModuleDynamicSymbolDestructor destructor;
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Symbol namespace.
     *
     * Must not be `NULL`.
     */
    const char *ns;
} FimoModuleDynamicSymbolExport;

/**
 * Valid keys of `FimoModuleExportModifier`.
 */
typedef enum FimoModuleExportModifierKey {
    /**
     * Specifies that the module export has a destructor function
     * that must be called. The value must be a pointer to a
     * `FimoModuleExportModifierDestructor`.
     */
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR,
    /**
     * Specifies that the module should acquire a static dependency
     * to another module. The value must be a strong reference to
     * a `FimoModuleInfo`.
     */
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY,
    FIMO_MODULE_EXPORT_MODIFIER_KEY_LAST,
    FIMO_MODULE_EXPORT_MODIFIER_KEY_FORCE32 = 0x7FFFFFFF
} FimoModuleExportModifierKey;

/**
 * A modifier declaration for a module export.
 */
typedef struct FimoModuleExportModifier {
    FimoModuleExportModifierKey key;
    const void *value;
} FimoModuleExportModifier;

/**
 * Value for the `FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR` modifier key.
 */
typedef struct FimoModuleExportModifierDestructor {
    /**
     * Type-erased data to pass to the destructor.
     */
    void *data;
    /**
     * Destructor function.
     */
    void (*destructor)(void *);
} FimoModuleExportModifierDestructor;

/**
 * Declaration of a module export.
 */
typedef struct FimoModuleExport {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_MODULE_EXPORT`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * ABI version of the module export.
     *
     * Must be `FIMO_MODULE_EXPORT_ABI`.
     */
    FimoI32 export_abi;
    /**
     * Module name.
     *
     * The module name must be unique to the module.
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Module description.
     */
    const char *description;
    /**
     * Module author.
     */
    const char *author;
    /**
     * Module license.
     */
    const char *license;
    /**
     * List of parameters exposed by the module.
     *
     * A module is not allowed to expose duplicate parameters.
     */
    const FimoModuleParamDecl *parameters;
    /**
     * Number of parameters exposed by the module.
     */
    FimoU32 parameters_count;
    /**
     * List of resources declared by the module.
     */
    const FimoModuleResourceDecl *resources;
    /**
     * Number of resources declared by the module.
     */
    FimoU32 resources_count;
    /**
     * List of namespaces to import by the module.
     *
     * A module is only allowed to import and export symbols
     * from/to an imported namespace. It is an error to specify
     * a namespace, that does not exist, without exporting to
     * that namespace.
     */
    const FimoModuleNamespaceImport *namespace_imports;
    /**
     * Number of namespaces to import by the module.
     */
    FimoU32 namespace_imports_count;
    /**
     * List of symbols to import by the module.
     *
     * Upon loading, the module is provided the listed symbols.
     * If some symbols are not available, the loading fails.
     */
    const FimoModuleSymbolImport *symbol_imports;
    /**
     * Number of symbols to import by the module.
     */
    FimoU32 symbol_imports_count;
    /**
     * List of static symbols exported by the module.
     *
     * The named symbols will be made available to all other
     * modules. Trying to export a duplicate symbol will
     * result in an error upon loading of the module.
     */
    const FimoModuleSymbolExport *symbol_exports;
    /**
     * Number of static symbols exported by the module.
     */
    FimoU32 symbol_exports_count;
    /**
     * List of dynamic symbols exported by the module.
     *
     * A dynamic symbol is a symbol, whose creation is deferred
     * until loading of the module. This is useful, in case
     * the symbol depends on the module imports.
     */
    const FimoModuleDynamicSymbolExport *dynamic_symbol_exports;
    /**
     * Number of dynamic symbols exported by the module.
     */
    FimoU32 dynamic_symbol_exports_count;
    /**
     * List of modifier key-value pairs for the exported module.
     */
    const FimoModuleExportModifier *modifiers;
    /**
     * Number of modifiers for the module.
     */
    FimoU32 modifiers_count;
    /**
     * Optional constructor for the module.
     *
     * If a module defines a constructor it must also specify
     * a destructor function.
     */
    FimoModuleConstructor module_constructor;
    /**
     * Optional destructor for the module.
     *
     * Must be specified, if the module specifies a constructor,
     * and must be `NULL` otherwise.
     */
    FimoModuleDestructor module_destructor;
} FimoModuleExport;

/**
 * Type-erased symbol definition.
 */
typedef struct FimoModuleRawSymbol {
    /**
     * Pointer to the symbol.
     */
    const void *data;
    /**
     * Lock count of the symbol.
     */
    _Atomic(FimoUSize) lock;
} FimoModuleRawSymbol;

// The use of atomics trips up our generation of rust bindings,
// so we disable them.
#ifndef FIMO_STD_BINDGEN
#ifdef FIMO_STD_BUILD_SHARED
FIMO_EXPORT
bool fimo_impl_module_symbol_is_used(_Atomic(FimoUSize) *lock);
FIMO_EXPORT
void fimo_impl_module_symbol_acquire(_Atomic(FimoUSize) *lock);
FIMO_EXPORT
void fimo_impl_module_symbol_release(_Atomic(FimoUSize) *lock);
#else
static FIMO_INLINE_ALWAYS bool fimo_impl_module_symbol_is_used(_Atomic(FimoUSize) *lock) {
    return atomic_load_explicit(lock, memory_order_acquire) != 0;
}

static FIMO_INLINE_ALWAYS void fimo_impl_module_symbol_acquire(_Atomic(FimoUSize) *lock) {
    FimoUSize count = atomic_fetch_add_explicit(lock, 1, memory_order_acquire);
    FIMO_ASSERT(count < (FimoUSize)FIMO_ISIZE_MAX)
}

static FIMO_INLINE_ALWAYS void fimo_impl_module_symbol_release(_Atomic(FimoUSize) *lock) {
    FimoUSize count = atomic_fetch_sub_explicit(lock, 1, memory_order_release);
    FIMO_ASSERT(count != 0)
}
#endif
#endif

/**
 * Opaque type for a parameter table of a module.
 *
 * The layout of a parameter table is equivalent to an array of
 * `FimoModuleParam*`, where each entry represents one parameter
 * of the module parameter declaration list.
 */
typedef void FimoModuleParamTable;

/**
 * Opaque type for a resource path table of a module.
 *
 * The import table is equivalent to an array of `const char*`,
 * where each entry represents one resource path. The resource
 * paths are ordered in declaration order.
 */
typedef void FimoModuleResourceTable;

/**
 * Opaque type for a symbol import table of a module.
 *
 * The import table is equivalent to an array of `const FimoModuleRawSymbol*`,
 * where each entry represents one symbol of the module symbol
 * import list. The symbols are ordered in declaration order.
 */
typedef void FimoModuleSymbolImportTable;

/**
 * Opaque type for a symbol export table of a module.
 *
 * The export table is equivalent to an array of `const FimoModuleRawSymbol*`,
 * where each entry represents one symbol of the module symbol
 * export list, followed by the entries of the dynamic symbol
 * export list.
 */
typedef void FimoModuleSymbolExportTable;

/**
 * Info of a loaded module.
 */
typedef struct FimoModuleInfo {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_MODULE_INFO`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * Module name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Module description.
     */
    const char *description;
    /**
     * Module author.
     */
    const char *author;
    /**
     * Module license.
     */
    const char *license;
    /**
     * Path to the module directory.
     */
    const char *module_path;
    /**
     * Increases the reference count of the instance.
     *
     * Not `NULL`.
     */
    void (*acquire)(const struct FimoModuleInfo *);
    /**
     * Decreases the reference count of the instance.
     *
     * Not `NULL`.
     */
    void (*release)(const struct FimoModuleInfo *);
    /**
     * Returns whether the owning module is still loaded.
     *
     * Not `NULL`.
     */
    bool (*is_loaded)(const struct FimoModuleInfo *);
    /**
     * Prevents the module from being unloaded.
     *
     * Not `NULL`.
     */
    FimoResult (*lock_unload)(const struct FimoModuleInfo *);
    /**
     * Unlocks a previously locked module, allowing it to be unloaded.
     *
     * Not `NULL`.
     */
    void (*unlock_unload)(const struct FimoModuleInfo *);
} FimoModuleInfo;

#ifndef FIMO_STD_BINDGEN
#ifdef FIMO_STD_BUILD_SHARED
FIMO_EXPORT
void fimo_impl_module_info_acquire(const FimoModuleInfo *info);
FIMO_EXPORT
void fimo_impl_module_info_release(const FimoModuleInfo *info);
FIMO_EXPORT
bool fimo_impl_module_info_is_loaded(const FimoModuleInfo *info);
FIMO_EXPORT
FimoResult fimo_impl_module_info_lock_unload(const FimoModuleInfo *info);
FIMO_EXPORT
void fimo_impl_module_info_unlock_unload(const FimoModuleInfo *info);
#else
static FIMO_INLINE_ALWAYS void fimo_impl_module_info_acquire(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    info->acquire(info);
}

static FIMO_INLINE_ALWAYS void fimo_impl_module_info_release(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    info->release(info);
}

static FIMO_INLINE_ALWAYS bool fimo_impl_module_info_is_loaded(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    return info->is_loaded(info);
}

static FIMO_INLINE_ALWAYS FimoResult fimo_impl_module_info_lock_unload(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    return info->lock_unload(info);
}

static FIMO_INLINE_ALWAYS void fimo_impl_module_info_unlock_unload(const FimoModuleInfo *info) {
    FIMO_DEBUG_ASSERT(info)
    info->unlock_unload(info);
}
#endif
#endif

/**
 * State of a loaded module.
 *
 * A module is self-contained, and may not be passed to other modules.
 * An instance of `FimoModule` is valid for as long as the owning module
 * remains loaded. Modules must not leak any resources outside it's own
 * module, ensuring that they are destroyed upon module unloading.
 */
typedef struct FimoModule {
    /**
     * Module parameter table.
     */
    const FimoModuleParamTable *parameters;
    /**
     * Module resource table.
     */
    const FimoModuleResourceTable *resources;
    /**
     * Module symbol import table.
     */
    const FimoModuleSymbolImportTable *imports;
    /**
     * Module symbol export table.
     */
    const FimoModuleSymbolExportTable *exports;
    /**
     * Module info.
     */
    const FimoModuleInfo *module_info;
    /**
     * Context that loaded the module.
     */
    FimoContext context;
    /**
     * Private data of the module.
     */
    void *module_data;
} FimoModule;

/**
 * A filter for selection modules to load by the module backend.
 *
 * The filter function is passed the module export declaration
 * and can then decide whether the module should be loaded by
 * the backend.
 *
 * @param arg0 module export to inspect
 * @param arg1 filter data
 *
 * @return `true`, if the module should be loaded.
 */
typedef bool (*FimoModuleLoadingFilter)(const FimoModuleExport *arg0, void *arg1);

/**
 * A callback for successfully loading a module.
 *
 * The callback function is called when the backend was successful
 * in loading the requested module, making it then possible to
 * request symbols.
 *
 * @param arg0 loaded module
 * @param arg1 callback data
 */
typedef void (*FimoModuleLoadingSuccessCallback)(const FimoModuleInfo *arg0, void *arg1);

/**
 * A callback for a module loading error.
 *
 * The callback function is called when the backend was not
 * successful in loading the requested module.
 *
 * @param arg0 module that caused the error
 * @param arg1 callback data
 */
typedef void (*FimoModuleLoadingErrorCallback)(const FimoModuleExport *arg0, void *arg1);

/**
 * VTable of the module subsystem.
 *
 * Changing the VTable is a breaking change.
 */
typedef struct FimoModuleVTableV0 {
    FimoResult (*pseudo_module_new)(void *, const FimoModule **);
    FimoResult (*pseudo_module_destroy)(void *, const FimoModule *, FimoContext *);
    FimoResult (*set_new)(void *, FimoModuleLoadingSet **);
    FimoResult (*set_has_module)(void *, FimoModuleLoadingSet *, const char *, bool *);
    FimoResult (*set_has_symbol)(void *, FimoModuleLoadingSet *, const char *, const char *, FimoVersion, bool *);
    FimoResult (*set_append_callback)(void *, FimoModuleLoadingSet *, const char *, FimoModuleLoadingSuccessCallback,
                                      FimoModuleLoadingErrorCallback, void *);
    FimoResult (*set_append_freestanding_module)(void *, const FimoModule *, FimoModuleLoadingSet *,
                                                 const FimoModuleExport *);
    FimoResult (*set_append_modules)(void *, FimoModuleLoadingSet *, const char *, FimoModuleLoadingFilter, void *,
                                     void (*)(bool (*)(const FimoModuleExport *, void *), void *), const void *);
    FimoResult (*set_dismiss)(void *, FimoModuleLoadingSet *);
    FimoResult (*set_finish)(void *, FimoModuleLoadingSet *);
    FimoResult (*find_by_name)(void *, const char *, const FimoModuleInfo **);
    FimoResult (*find_by_symbol)(void *, const char *, const char *, FimoVersion, const FimoModuleInfo **);
    FimoResult (*namespace_exists)(void *, const char *, bool *);
    FimoResult (*namespace_include)(void *, const FimoModule *, const char *);
    FimoResult (*namespace_exclude)(void *, const FimoModule *, const char *);
    FimoResult (*namespace_included)(void *, const FimoModule *, const char *, bool *, bool *);
    FimoResult (*acquire_dependency)(void *, const FimoModule *, const FimoModuleInfo *);
    FimoResult (*relinquish_dependency)(void *, const FimoModule *, const FimoModuleInfo *);
    FimoResult (*has_dependency)(void *, const FimoModule *, const FimoModuleInfo *, bool *, bool *);
    FimoResult (*load_symbol)(void *, const FimoModule *, const char *, const char *, FimoVersion,
                              const FimoModuleRawSymbol **);
    FimoResult (*unload)(void *, const FimoModuleInfo *);
    FimoResult (*param_query)(void *, const char *, const char *, FimoModuleParamType *, FimoModuleParamAccess *,
                              FimoModuleParamAccess *);
    FimoResult (*param_set_public)(void *, const void *, FimoModuleParamType, const char *, const char *);
    FimoResult (*param_get_public)(void *, void *, FimoModuleParamType *, const char *, const char *);
    FimoResult (*param_set_dependency)(void *, const FimoModule *, const void *, FimoModuleParamType, const char *,
                                       const char *);
    FimoResult (*param_get_dependency)(void *, const FimoModule *, void *, FimoModuleParamType *, const char *,
                                       const char *);
    FimoResult (*param_set_private)(void *, const FimoModule *, const void *, FimoModuleParamType, FimoModuleParam *);
    FimoResult (*param_get_private)(void *, const FimoModule *, void *, FimoModuleParamType *, const FimoModuleParam *);
    FimoResult (*param_set_inner)(void *, const FimoModule *, const void *, FimoModuleParamType, FimoModuleParamData *);
    FimoResult (*param_get_inner)(void *, const FimoModule *, void *, FimoModuleParamType *,
                                  const FimoModuleParamData *);
} FimoModuleVTableV0;

/**
 * Constructs a new pseudo module.
 *
 * The functions of the module backend require that the caller owns
 * a reference to their own module. This is a problem, as the constructor
 * of the context won't be assigned a module instance during bootstrapping.
 * As a workaround, we allow for the creation of pseudo modules, i.e.,
 * module handles without an associated module.
 *
 * @param context the context
 * @param module resulting pseudo module
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_pseudo_module_new(FimoContext context, const FimoModule **module);

/**
 * Destroys an existing pseudo module.
 *
 * By destroying the pseudo module, the caller ensures that they
 * relinquished all access to handles derived by the module backend.
 *
 * @param module pseudo module to destroy
 * @param module_context extracted context from the module
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_pseudo_module_destroy(const FimoModule *module, FimoContext *module_context);

/**
 * Constructs a new empty module set.
 *
 * The loading of a module fails, if at least one dependency can
 * not be satisfied, which requires the caller to manually find a
 * suitable loading order. To facilitate the loading, we load
 * multiple modules together, and automatically determine an
 * appropriate load order for all modules inside the module set.
 *
 * @param context the context
 * @param module_set new module set
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_new(FimoContext context, FimoModuleLoadingSet **module_set);

/**
 * Checks whether a module set contains a module.
 *
 * @param context the context
 * @param module_set module set to query
 * @param name name of the module
 * @param has_module query result
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_has_module(FimoContext context, FimoModuleLoadingSet *module_set, const char *name,
                                      bool *has_module);

/**
 * Checks whether a module set contains a symbol.
 *
 * @param context the context
 * @param module_set module set to query
 * @param name symbol name
 * @param ns namespace name
 * @param version symbol version
 * @param has_symbol query result
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_has_symbol(FimoContext context, FimoModuleLoadingSet *module_set, const char *name,
                                      const char *ns, FimoVersion version, bool *has_symbol);

/**
 * Adds a status callback to the module set.
 *
 * Adds a set of callbacks to report a successful or failed loading of
 * a module. The `on_success` callback wil be called if the set has
 * was able to load all requested modules, whereas the `on_error` callback
 * will be called immediately after the failed loading of the module. Since
 * the module set can be in a partially loaded state at the time of calling
 * this function, the `on_errro` callback may be invoked immediately. The
 * callbacks will be provided with a user-specified data pointer, which they
 * are in charge of cleaning up. If the requested module `module_name` does
 * not exist, this function will return an error.
 *
 * @param context the context
 * @param module_set set of modules
 * @param module_name module to query
 * @param on_success success callback
 * @param on_error error callback
 * @param user_data callback user data
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_append_callback(FimoContext context, FimoModuleLoadingSet *module_set,
                                           const char *module_name, FimoModuleLoadingSuccessCallback on_success,
                                           FimoModuleLoadingErrorCallback on_error, void *user_data);

/**
 * Adds a freestanding module to the module set.
 *
 * Adds a freestanding module to the set, so that it may be loaded
 * by a future call to `fimo_module_set_finish`. Trying to include
 * an invalid module, a module with duplicate exports or duplicate
 * name will result in an error. Unlike `fimo_module_set_append_modules`,
 * this function allows for the loading of dynamic modules, i.e.
 * modules that are created at runtime, like non-native modules,
 * which may require a runtime to be executed in. To ensure that
 * the binary of the module calling this function is not unloaded
 * while the new module is instantiated, the new module inherits
 * a strong reference to the same binary as the caller's module.
 * Note that the new module is not setup to automatically depend
 * on `module`, but may prevent it from being unloaded while
 * the set exists.
 *
 * @param module owner of the export
 * @param module_set set of modules
 * @param module_export module to append to the set
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_append_freestanding_module(const FimoModule *module, FimoModuleLoadingSet *module_set,
                                                      const FimoModuleExport *module_export);

/**
 * Adds modules to the module set.
 *
 * Opens up a module binary to select which modules to load.
 * The binary path `module_path` must be encoded as `UTF-8`,
 * and point to the binary that contains the modules.  If the
 * path is `NULL`, it iterates over the exported modules of the
 * current binary. Each exported module is then passed to the
 * `filter`, along with the provided `filter_data`, which can
 * then filter which modules to load. This function may skip
 * invalid module exports. Trying to include a module with duplicate
 * exports or duplicate name will result in an error. This function
 * signals an error, if the binary does not contain the symbols
 * necessary to query the exported modules, but does not return
 * in an error, if it does not export any modules. The necessary
 * symbols are setup automatically, if the binary was linked with
 * the fimo library. In case of an error, no modules are appended
 * to the set.
 *
 * @param context the context
 * @param module_set set of modules
 * @param module_path path to the binary to inspect
 * @param filter filter function
 * @param filter_data custom data to pass to the filter function
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_append_modules(FimoContext context, FimoModuleLoadingSet *module_set,
                                          const char *module_path, FimoModuleLoadingFilter filter, void *filter_data);

/**
 * Destroys the module set without loading any modules.
 *
 * It is not possible to dismiss a module set that is currently
 * being loaded.
 *
 * @param context the context
 * @param module_set the module set to destroy
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_dismiss(FimoContext context, FimoModuleLoadingSet *module_set);

/**
 * Destroys the module set and loads the modules contained in it.
 *
 * After successfully calling this function, the modules contained
 * in the set are loaded, and their symbols are available to all
 * other modules. If the construction of one module results in an
 * error, or if a dependency can not be satisfied, this function
 * rolls back the loading of all modules contained in the set
 * and returns an error. It is not possible to load a module set,
 * while another set is being loaded.
 *
 * @param context the context
 * @param module_set a set of modules to load
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_set_finish(FimoContext context, FimoModuleLoadingSet *module_set);

/**
 * Searches for a module by it's name.
 *
 * Queries a module by its unique name. The returned `FimoModuleInfo`
 * will have its reference count increased.
 *
 * @param context context
 * @param name module name
 * @param module resulting module.
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_find_by_name(FimoContext context, const char *name, const FimoModuleInfo **module);

/**
 * Searches for a module by a symbol it exports.
 *
 * Queries the module that exported the specified symbol. The returned
 * `FimoModuleInfo` will have its reference count increased.
 *
 * @param context context
 * @param name symbol name
 * @param ns symbol namespace
 * @param version symbol version
 * @param module resulting module
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_find_by_symbol(FimoContext context, const char *name, const char *ns, FimoVersion version,
                                      const FimoModuleInfo **module);

/**
 * Checks for the presence of a namespace in the module backend.
 *
 * A namespace exists, if at least one loaded module exports
 * one symbol in said namespace.
 *
 * @param context context
 * @param ns namespace to query
 * @param exists query result
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_exists(FimoContext context, const char *ns, bool *exists);

/**
 * Includes a namespace by the module.
 *
 * Once included, the module gains access to the symbols
 * of its dependencies that are exposed in said namespace.
 * A namespace can not be included multiple times.
 *
 * @param module module of the caller
 * @param ns namespace to include
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_include(const FimoModule *module, const char *ns);

/**
 * Removes a namespace include from the module.
 *
 * Once excluded, the caller guarantees to relinquish
 * access to the symbols contained in said namespace.
 * It is only possible to exclude namespaces that were
 * manually added, whereas static namespace includes
 * remain valid until the module is unloaded.
 *
 * @param module module of the caller
 * @param ns namespace to exclude.
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_exclude(const FimoModule *module, const char *ns);

/**
 * Checks if a module includes a namespace.
 *
 * Checks if `module` specified that it includes the
 * namespace `ns`. In that case, the module is allowed access
 * to the symbols in the namespace. The result of the query
 * is stored in `is_included`. Additionally, this function also
 * queries whether the include is static, i.e., the include was
 * specified by the module at load time. The include type is
 * stored in `is_static`.
 *
 * @param module module of the caller
 * @param ns namespace to query
 * @param is_included result of the query
 * @param is_static resulting include type
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_namespace_included(const FimoModule *module, const char *ns, bool *is_included, bool *is_static);

/**
 * Acquires another module as a dependency.
 *
 * After acquiring a module as a dependency, the module
 * is allowed access to the symbols and protected parameters
 * of said dependency. Trying to acquire a dependency to a
 * module that is already a dependency, or to a module that
 * would result in a circular dependency will result in an
 * error.
 *
 * @param module module of the caller
 * @param dependency module to acquire as a dependency
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_acquire_dependency(const FimoModule *module, const FimoModuleInfo *dependency);

/**
 * Removes a module as a dependency.
 *
 * By removing a module as a dependency, the caller
 * ensures that it does not own any references to resources
 * originating from the former dependency, and allows for
 * the unloading of the module. A module can only relinquish
 * dependencies to modules that were acquired dynamically,
 * as static dependencies remain valid until the module is
 * unloaded.
 *
 * @param module module of the caller
 * @param dependency dependency to remove
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_relinquish_dependency(const FimoModule *module, const FimoModuleInfo *dependency);

/**
 * Checks if a module depends on another module.
 *
 * Checks if `other` is a dependency of `module`. In that
 * case `module` is allowed to access the symbols exported
 * by `other`. The result of the query is stored in
 * `has_dependency`. Additionally, this function also
 * queries whether the dependency is static, i.e., the
 * dependency was set by the module backend at load time.
 * The dependency type is stored in `is_static`.
 *
 * @param module module of the caller
 * @param other other module to check as a dependency
 * @param has_dependency result of the query
 * @param is_static resulting dependency type
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_has_dependency(const FimoModule *module, const FimoModuleInfo *other, bool *has_dependency,
                                      bool *is_static);

/**
 * Loads a symbol from the module backend.
 *
 * The caller can query the backend for a symbol of a loaded
 * module. This is useful for loading optional symbols, or
 * for loading symbols after the creation of a module. The
 * symbol, if it exists, is written into `symbol`, and can
 * be used until the module relinquishes the dependency to
 * the module that exported the symbol. This function fails,
 * if the module containing the symbol is not a dependency
 * of the module.
 *
 * @param module module that requires the symbol
 * @param name symbol name
 * @param ns symbol namespace
 * @param version symbol version
 * @param symbol resulting symbol
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_load_symbol(const FimoModule *module, const char *name, const char *ns, FimoVersion version,
                                   const FimoModuleRawSymbol **symbol);

/**
 * Unloads a module.
 *
 * If successful, this function unloads the module `module`.
 * To succeed, the module no other module may depend on the module.
 * This function automatically unloads cleans up unreferenced modules,
 * except if they are a pseudo module.
 *
 * Setting `module` to `NULL` only runs the cleanup of all loose modules.
 *
 * @param context the context
 * @param module module to unload
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_unload(FimoContext context, const FimoModuleInfo *module);

/**
 * Queries the info of a module parameter.
 *
 * This function can be used to query the datatype, the read access,
 * and the write access of a module parameter. This function fails,
 * if the parameter can not be found.
 *
 * @param context context
 * @param module_name name of the module containing the parameter
 * @param param parameter to query
 * @param type queried parameter datatype
 * @param read queried parameter read access
 * @param write queried parameter write access
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_query(FimoContext context, const char *module_name, const char *param,
                                   FimoModuleParamType *type, FimoModuleParamAccess *read,
                                   FimoModuleParamAccess *write);

/**
 * Sets a module parameter with public write access.
 *
 * Sets the value of a module parameter with public write access.
 * The operation fails, if the parameter does not exist, or if
 * the parameter does not allow writing with a public access.
 * The caller must ensure that `value` points to an instance of
 * the same datatype as the parameter in question.
 *
 * @param context context
 * @param value pointer to the value to store
 * @param type type of the value
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_public(FimoContext context, const void *value, FimoModuleParamType type,
                                        const char *module_name, const char *param);

/**
 * Reads a module parameter with public read access.
 *
 * Reads the value of a module parameter with public read access.
 * The operation fails, if the parameter does not exist, or if
 * the parameter does not allow reading with a public access.
 * The caller must ensure that `value` points to an instance of
 * the same datatype as the parameter in question.
 *
 * @param context context
 * @param value pointer where to store the value
 * @param type buffer where to store the type of the parameter
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_public(FimoContext context, void *value, FimoModuleParamType *type,
                                        const char *module_name, const char *param);

/**
 * Sets a module parameter with dependency write access.
 *
 * Sets the value of a module parameter with dependency write
 * access. The operation fails, if the parameter does not exist,
 * or if the parameter does not allow writing with a dependency
 * access. The caller must ensure that `value` points to an
 * instance of the same datatype as the parameter in question.
 *
 * @param module module of the caller
 * @param value pointer to the value to store
 * @param type type of the value
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_dependency(const FimoModule *module, const void *value, FimoModuleParamType type,
                                            const char *module_name, const char *param);

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
 * @param type buffer where to store the type of the parameter
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_dependency(const FimoModule *module, void *value, FimoModuleParamType *type,
                                            const char *module_name, const char *param);

/**
 * Setter for a module parameter.
 *
 * If the setter produces an error, the parameter won't be modified.
 *
 * @param module module of the caller
 * @param value value to write
 * @param type type of the value
 * @param param parameter to write
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_private(const FimoModule *module, const void *value, FimoModuleParamType type,
                                         FimoModuleParam *param);

/**
 * Getter for a module parameter.
 *
 * @param module module of the caller
 * @param value buffer where to store the parameter
 * @param type buffer where to store the type of the parameter
 * @param param parameter to load
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_private(const FimoModule *module, void *value, FimoModuleParamType *type,
                                         const FimoModuleParam *param);

/**
 * Internal setter for a module parameter.
 *
 * If the setter produces an error, the parameter won't be modified.
 *
 * @param module module of the caller
 * @param value value to write
 * @param type type of the value
 * @param param parameter to write
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_set_inner(const FimoModule *module, const void *value, FimoModuleParamType type,
                                       FimoModuleParamData *param);

/**
 * Internal getter for a module parameter.
 *
 * @param module module of the caller
 * @param value buffer where to store the parameter
 * @param type buffer where to store the type of the parameter
 * @param param parameter to load
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_module_param_get_inner(const FimoModule *module, void *value, FimoModuleParamType *type,
                                       const FimoModuleParamData *param);

#ifdef __cplusplus
}
#endif

#endif // FIMO_MODULE_H
