#ifndef FIMO_MODULE_H
#define FIMO_MODULE_H

#include <assert.h>
#include <stdalign.h>
#include <stdbool.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/version.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
// With the MSVC we have no way to get the start and end of
// a section, so we use three different sections. According
// to the documentation, the linker orders the entries with
// the same section prefix by the section name. Therefore,
// we can get the same result by allocating all entries in
// the middle section.
#pragma section("fi_mod$a", read)
#pragma section("fi_mod$u", read)
#pragma section("fi_mod$z", read)

/**
 * Name of the section where the modules will be stored to.
 */
#define FIMO_MODULE_SECTION "fi_mod$u"
#elif __APPLE__
/**
 * Name of the section where the modules will be stored to.
 */
#define FIMO_MODULE_SECTION "__DATA,__fimo_module"
#else
/**
 * Name of the section where the modules will be stored to.
 */
#define FIMO_MODULE_SECTION "fimo_module"
#endif

/**
 * Constructs a new `FimoU8` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_U8_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                \
        .type = FIMO_MODULE_PARAM_TYPE_U8,                                           \
        .read_access = (pread),                                                      \
        .write_access = (pwrite),                                                    \
        .setter = (psetter),                                                         \
        .getter = (pgetter),                                                         \
        .name = (pname),                                                             \
        .default_value = { .u8 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoU16` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_U16_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                 \
        .type = FIMO_MODULE_PARAM_TYPE_U16,                                           \
        .read_access = (pread),                                                       \
        .write_access = (pwrite),                                                     \
        .setter = (psetter),                                                          \
        .getter = (pgetter),                                                          \
        .name = (pname),                                                              \
        .default_value = { .u16 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoU32` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_U32_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                 \
        .type = FIMO_MODULE_PARAM_TYPE_U32,                                           \
        .read_access = (pread),                                                       \
        .write_access = (pwrite),                                                     \
        .setter = (psetter),                                                          \
        .getter = (pgetter),                                                          \
        .name = (pname),                                                              \
        .default_value = { .u32 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoU64` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_U64_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                 \
        .type = FIMO_MODULE_PARAM_TYPE_U64,                                           \
        .read_access = (pread),                                                       \
        .write_access = (pwrite),                                                     \
        .setter = (psetter),                                                          \
        .getter = (pgetter),                                                          \
        .name = (pname),                                                              \
        .default_value = { .u64 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoI8` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_I8_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                \
        .type = FIMO_MODULE_PARAM_TYPE_I8,                                           \
        .read_access = (pread),                                                      \
        .write_access = (pwrite),                                                    \
        .setter = (psetter),                                                         \
        .getter = (pgetter),                                                         \
        .name = (pname),                                                             \
        .default_value = { .i8 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoI16` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_I16_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                 \
        .type = FIMO_MODULE_PARAM_TYPE_U16,                                           \
        .read_access = (pread),                                                       \
        .write_access = (pwrite),                                                     \
        .setter = (psetter),                                                          \
        .getter = (pgetter),                                                          \
        .name = (pname),                                                              \
        .default_value = { .i16 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoI32` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_I32_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                 \
        .type = FIMO_MODULE_PARAM_TYPE_U32,                                           \
        .read_access = (pread),                                                       \
        .write_access = (pwrite),                                                     \
        .setter = (psetter),                                                          \
        .getter = (pgetter),                                                          \
        .name = (pname),                                                              \
        .default_value = { .i32 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoI64` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 * @param psetter parameter setter
 * @param pgetter parameter getter
 */
#define FIMO_MODULE_PARAM_I64_COMPLEX(pname, pvalue, pread, pwrite, psetter, pgetter) \
    {                                                                                 \
        .type = FIMO_MODULE_PARAM_TYPE_U64,                                           \
        .read_access = (pread),                                                       \
        .write_access = (pwrite),                                                     \
        .setter = (psetter),                                                          \
        .getter = (pgetter),                                                          \
        .name = (pname),                                                              \
        .default_value = { .i64 = (pvalue) },                                         \
    }

/**
 * Constructs a new `FimoU8` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_U8(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_U8_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoU16` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_U16(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_U16_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoU32` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_U32(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_U32_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoU64` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_U64(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_U64_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI8` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_I8(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_I8_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI16` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_I16(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_I16_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI32` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_I32(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_I32_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new `FimoI64` parameter declaration.
 *
 * @param pname parameter name
 * @param pvalue default value
 * @param pread read access group
 * @param pwrite write access group
 */
#define FIMO_MODULE_PARAM_I64(pname, pvalue, pread, pwrite) \
    FIMO_MODULE_PARAM_I64_COMPLEX(pname, pvalue, pread, pwrite, fimo_module_param_set_inner, fimo_module_param_get_inner)

/**
 * Constructs a new namespace import declaration.
 */
#define FIMO_MODULE_IMPORT_NAMESPACE(ns) \
    {                                    \
        .name = (ns)                     \
    }

/**
 * Constructs a new symbol import declaration.
 *
 * @param sname symbol name
 * @param sns symbol namespace
 * @param sversion symbol version
 */
#define FIMO_MODULE_IMPORT_SYMBOL_NS(sname, sns, sversion)  \
    {                                                       \
        .version = (sversion), .name = (sname), .ns = (sns) \
    }

/**
 * Constructs a new symbol import declaration.
 *
 * @param sname symbol name
 * @param sversion symbol version
 */
#define FIMO_MODULE_IMPORT_SYMBOL(sname, sversion) \
    FIMO_MODULE_IMPORT_SYMBOL_NS(sname, "", sversion)

/**
 * Constructs a new symbol export declaration for a variable.
 *
 * @param sname symbol name
 * @param sns symbol namespace
 * @param svar variable to export
 * @param sversion symbol version
 */
#define FIMO_MODULE_EXPORT_SYMBOL_VAR_NS(sname, sns, svar, sversion)                 \
    {                                                                                \
        .symbol = (void*)&(svar), .version = (version), .name = (sname), .ns = (sns) \
    }

/**
 * Constructs a new symbol export declaration for a function.
 *
 * @param sname symbol name
 * @param sns symbol namespace
 * @param sfunc function to export
 * @param sversion symbol version
 */
#define FIMO_MODULE_EXPORT_SYMBOL_FUNC_NS(sname, sns, sfunc, sversion)               \
    {                                                                                \
        .symbol = (void*)(sfunc), .version = (version), .name = (sname), .ns = (sns) \
    }

/**
 * Constructs a new symbol export declaration for a variable.
 *
 * @param svar variable to export
 * @param sversion symbol version
 * @param sname symbol name
 */
#define FIMO_MODULE_EXPORT_SYMBOL_VAR(sname, svar, sversion) \
    FIMO_MODULE_EXPORT_SYMBOL_VAR_NS(sname, "", svar, sversion)

/**
 * Constructs a new symbol export declaration for a function.
 *
 * @param sname symbol name
 * @param sfunc function to export
 * @param sversion symbol version
 */
#define FIMO_MODULE_EXPORT_SYMBOL_FUNC(sname, sfunc, sversion) \
    FIMO_MODULE_EXPORT_SYMBOL_FUNC_NS(sname, "", sfunc, sversion)

/**
 * Constructs a new dynamic symbol export declaration.
 *
 * @param sname symbol name
 * @param sns symbol namespace
 * @param sversion symbol version
 * @param sconstr constructor
 * @param sdestr destructor
 */
#define FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL_NS(sname, sns, sversion, sconstr, sdestr)                           \
    {                                                                                                         \
        .constructor = (sconstr), .destructor = (sdestr), .version = (sversion), .name = (sname), .ns = (sns) \
    }

/**
 * Constructs a new dynamic symbol export declaration.
 *
 * @param sname symbol name
 * @param sversion symbol version
 * @param sconstr constructor
 * @param sdestr destructor
 */
#define FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL(sname, sversion, sconstr, sdestr) \
    FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL_NS(sname, "", sversion, sconstr, sdestr)

/**
 * ABI version of the current module export.
 */
#define FIMO_MODULE_EXPORT_ABI 0

#ifdef _WIN32
#define FIMO_MODULE_EXPORT_MODULE__(VAR)      \
    __declspec(allocate(FIMO_MODULE_SECTION)) \
        const FimoModuleExport* FIMO_VAR(VAR) \
        = &VAR;
#else
#define FIMO_MODULE_EXPORT_MODULE__(VAR)                    \
    const FimoModuleExport* FIMO_VAR(VAR)                   \
        __attribute__((used, section(FIMO_MODULE_SECTION))) \
        = &VAR;
#endif

#define FIMO_MODULE_EXPORT_MODULE_(VAR, NAME, DESC, AUTHOR, LICENSE, ...) \
    FimoModuleExport VAR = {                                              \
        .type = FIMO_STRUCT_TYPE_MODULE_EXPORT,                           \
        .next = NULL,                                                     \
        .export_abi = FIMO_MODULE_EXPORT_ABI,                             \
        .name = NAME,                                                     \
        .description = DESC,                                              \
        .author = AUTHOR,                                                 \
        .license = LICENSE,                                               \
        __VA_ARGS__                                                       \
    };                                                                    \
    FIMO_MODULE_EXPORT_MODULE__(VAR)

/**
 * Exports a new module.
 *
 * @param export_info module export info
 */
#define FIMO_MODULE_EXPORT_MODULE(NAME, DESC, AUTHOR, LICENSE, ...) \
    FIMO_MODULE_EXPORT_MODULE_(FIMO_VAR(fimo_module_export_private), NAME, DESC, AUTHOR, LICENSE, __VA_ARGS__)

/**
 * Specifies the module parameters.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param _param_list array of `FimoModuleParamDecl`
 */
#define FIMO_MODULE_EXPORT_MODULE_PARAMS(_param_list) \
    .parameters = _param_list,                        \
    .parameters_count = (FimoU32)(sizeof(_param_list) / sizeof(FimoModuleParamDecl))

/**
 * Specifies the module namespace imports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param _ns_list array of `FimoModuleNamespaceImport`
 */
#define FIMO_MODULE_EXPORT_MODULE_NAMESPACES(_ns_list) \
    .namespace_imports = _ns_list,                     \
    .namespace_imports_count = (FimoU32)(sizeof(_ns_list) / sizeof(FimoModuleNamespaceImport))

/**
 * Specifies the module symbol imports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param _symbol_list array of `FimoModuleSymbolImport`
 */
#define FIMO_MODULE_EXPORT_MODULE_SYMBOL_IMPORTS(_symbol_list) \
    .symbol_imports = _symbol_list,                            \
    .symbol_imports_count = (FimoU32)(sizeof(_symbol_list) / sizeof(FimoModuleSymbolImport))

/**
 * Specifies the static module symbol exports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param _symbol_list array of `FimoModuleSymbolExport`
 */
#define FIMO_MODULE_EXPORT_MODULE_SYMBOL_EXPORTS(_symbol_list) \
    .symbol_exports = _symbol_list,                            \
    .symbol_exports_count = (FimoU32)(sizeof(_symbol_list) / sizeof(FimoModuleSymbolExport))

/**
 * Specifies the dynamic module symbol exports.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param _symbol_list array of `FimoModuleDynamicSymbolExport`
 */
#define FIMO_MODULE_EXPORT_MODULE_DYNAMIC_SYMBOL_EXPORTS(_symbol_list) \
    .dynamic_symbol_exports = _symbol_list,                            \
    .dynamic_symbol_exports_count = (FimoU32)(sizeof(_symbol_list) / sizeof(FimoModuleDynamicSymbolExport))

/**
 * Specifies the constructor and destructor of a module.
 *
 * Must be called as a parameter of `FIMO_MODULE_EXPORT_MODULE`.
 *
 * @param _constructor constructor function
 * @param _destructor destructor function
 */
#define FIMO_MODULE_EXPORT_MODULE_CONSTRUCTOR(_constructor, _destructor) \
    .module_constructor = _constructor,                                  \
    .module_destructor = _destructor,

/**
 * Declares a new parameter table.
 *
 * For compatibility with C++, the table must contain at least one
 * element. Use `FIMO_MODULE_PARAM_TABLE_EMPTY` in that case. Otherwise,
 * the members of the struct must be declared with calls to
 * `FIMO_MODULE_PARAM_TABLE_PARAM`. The params must be declared in the
 * same order as they are specified in the module.
 *
 * @param name parameter table name
 * @param param_count number of parameters in the table
 * @param declaration table struct declaration
 */
#define FIMO_MODULE_PARAM_TABLE(name, param_count, declaration)                                            \
    declaration;                                                                                           \
    static_assert(alignof(name) == alignof(FimoModuleParam*), "Unexpected padding in module param table"); \
    static_assert(sizeof(name) > 0, "Unexpected size of module param table");                              \
    static_assert(sizeof(name) == param_count * sizeof(FimoModuleParam*), "Unexpected size of module param table");

/**
 * Declares a placeholder param.
 */
#define FIMO_MODULE_PARAM_TABLE_EMPTY \
    FimoModuleParam* empty__;

/**
 * Declares a new parameter for the param table.
 *
 * @param name name of the parameter
 */
#define FIMO_MODULE_PARAM_TABLE_PARAM(name) \
    FimoModuleParam* name;

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
 * @param name symbol table name
 * @param symbol_count number of symbols in the table
 * @param declaration table struct declaration
 */
#define FIMO_MODULE_SYMBOL_TABLE(name, symbol_count, declaration)                                      \
    declaration;                                                                                       \
    static_assert(alignof(name) == alignof(const void*), "Unexpected padding in module symbol table"); \
    static_assert(sizeof(name) > 0, "Unexpected size of module symbol table");                         \
    static_assert(sizeof(name) == symbol_count * sizeof(const void*), "Unexpected size of module symbol table");

/**
 * Declares a placeholder symbol.
 */
#define FIMO_MODULE_SYMBOL_TABLE_EMPTY \
    const void* empty__;

/**
 * Declares a new variable symbol for the symbol table.
 *
 * @param name name of the symbol in the symbol table
 * @param type type of the symbol
 */
#define FIMO_MODULE_SYMBOL_TABLE_VAR(name, type)                                         \
    const type* name;                                                                    \
    static_assert(sizeof(const type*) == sizeof(const void*), "Unexpected symbol size"); \
    static_assert(alignof(const type*) == alignof(const void*), "Unexpected symbol alignment");

/**
 * Declares a new function symbol for the symbol table.
 *
 * @param name name of the symbol in the symbol table
 * @param ret function return type
 * @param args function parameter list
 */
#define FIMO_MODULE_SYMBOL_TABLE_FUNC(name, ret, ...)                                                 \
    ret (*const name)(__VA_ARGS__);                                                                   \
    static_assert(sizeof(ret(*const)(__VA_ARGS__)) == sizeof(const void*), "Unexpected symbol size"); \
    static_assert(alignof(ret(*const)(__VA_ARGS__)) == alignof(const void*), "Unexpected symbol alignment");

typedef struct FimoModule FimoModule;

/**
 * Constructor function for a dynamic symbol.
 *
 * The constructor is in charge of constructing an instance of
 * a symbol. To that effect, it is provided  an instance to the
 * module. The resulting symbol is written into the last argument.
 *
 * @param arg0 pointer to the module
 * @param arg1 reserved for future use
 * @param arg2 pointer to the resulting symbol
 *
 * @return Status code.
 */
typedef FimoError (*FimoModuleDynamicSymbolConstructor)(const FimoModule*, void*, void**);

/**
 * Destructor function for a dynamic symbol.
 *
 * The destructor is safe to assume, that the symbol is no longer
 * used by any other module. During its destruction, a symbol is
 * not allowed to access the module backend.
 *
 * @param arg0 symbol to destroy
 * @param arg1 reserved for future use
 */
typedef void (*FimoModuleDynamicSymbolDestructor)(void*, void*);

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
 * @param arg2 reserved for future use
 * @param arg3 pointer to the resulting module data
 *
 * @return Status code.
 */
typedef FimoError (*FimoModuleConstructor)(const FimoModule*, FimoModuleLoadingSet*,
    void*, void**);

/**
 * Destructor function for a module.
 *
 * During its destruction, a module is not allowed to access the
 * module backend.
 *
 * @param arg0 pointer to the module
 * @param arg1 reserved for future use
 * @param arg2 module data to destroy
 */
typedef void (*FimoModuleDestructor)(const FimoModule*, void*, void*);

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
    FIMO_MODULE_CONFIG_ACCESS_PUBLIC,
    FIMO_MODULE_CONFIG_ACCESS_DEPENDENCY,
    FIMO_MODULE_CONFIG_ACCESS_PRIVATE,
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
typedef FimoError (*FimoModuleParamSet)(const FimoModule*, const void*, FimoModuleParamType,
    FimoModuleParamData*);

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
typedef FimoError (*FimoModuleParamGet)(const FimoModule*, void*, FimoModuleParamType*,
    const FimoModuleParamData*);

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
     */
    const char* name;
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
 * Declaration of a module namespace import.
 */
typedef struct FimoModuleNamespaceImport {
    const char* name;
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
     */
    const char* name;
    /**
     * Symbol namespace.
     */
    const char* ns;
} FimoModuleSymbolImport;

/**
 * Declaration of a static module symbol export.
 */
typedef struct FimoModuleSymbolExport {
    /**
     * Pointer to the symbol.
     */
    const void* symbol;
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     */
    const char* name;
    /**
     * Symbol namespace.
     */
    const char* ns;
} FimoModuleSymbolExport;

/**
 * Declaration of a dynamic module symbol export.
 */
typedef struct FimoModuleDynamicSymbolExport {
    /**
     * Symbol constructor.
     */
    FimoModuleDynamicSymbolConstructor constructor;
    /**
     * Symbol destructor.
     */
    FimoModuleDynamicSymbolDestructor destructor;
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     */
    const char* name;
    /**
     * Symbol namespace.
     */
    const char* ns;
} FimoModuleDynamicSymbolExport;

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
    const struct FimoBaseStructIn* next;
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
    const char* name;
    /**
     * Module description.
     *
     * Must not be `NULL`.
     */
    const char* description;
    /**
     * Module author.
     *
     * Must not be `NULL`.
     */
    const char* author;
    /**
     * Module license.
     *
     * Must not be `NULL`.
     */
    const char* license;
    /**
     * List of parameters exposed by the module.
     *
     * A module is not allowed to expose duplicate parameters.
     */
    const FimoModuleParamDecl* parameters;
    /**
     * Number of parameters exposed by the module.
     */
    FimoU32 parameters_count;
    /**
     * List of namespaces to import by the module.
     *
     * A module is only allowed to import and export symbols
     * from/to an imported namespace. It is an error to specify
     * a namespace, that does not exist, without exporting to
     * that namespace.
     */
    const FimoModuleNamespaceImport* namespace_imports;
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
    const FimoModuleSymbolImport* symbol_imports;
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
    const FimoModuleSymbolExport* symbol_exports;
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
    const FimoModuleDynamicSymbolExport* dynamic_symbol_exports;
    /**
     * Number of dynamic symbols exported by the module.
     */
    FimoU32 dynamic_symbol_exports_count;
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
 * The import table is equivalent to an array of `const void*`,
 * where each entry represents one symbol of the module symbol
 * import list. The symbols are ordered in declaration order.
 */
typedef void FimoModuleSymbolImportTable;

/**
 * Opaque type for a symbol export table of a module.
 *
 * The export table is equivalent to an array of `const void*`,
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
    const struct FimoBaseStructIn* next;
    /**
     * Module name.
     *
     * Must not be `NULL`.
     */
    const char* name;
    /**
     * Module description.
     *
     * Must not be `NULL`.
     */
    const char* description;
    /**
     * Module author.
     *
     * Must not be `NULL`.
     */
    const char* author;
    /**
     * Module license.
     *
     * Must not be `NULL`.
     */
    const char* license;
    /**
     * Path to the module binary.
     *
     * Must not be `NULL`.
     */
    const char* module_path;
    /**
     * Opaque data of the module.
     */
    void* internal_data;
} FimoModuleInfo;

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
    const FimoModuleParamTable* parameters;
    /**
     * Module resource table.
     */
    const FimoModuleResourceTable* resources;
    /**
     * Module symbol import table.
     */
    const FimoModuleSymbolImportTable* imports;
    /**
     * Module symbol export table.
     */
    const FimoModuleSymbolExportTable* exports;
    /**
     * Module info.
     */
    const FimoModuleInfo* module_info;
    /**
     * Context that loaded the module.
     */
    FimoContext context;
    /**
     * Private data of the module.
     */
    void* module_data;
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
typedef bool (*FimoModuleLoadingFilter)(const FimoModuleExport*, void*);

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
typedef void (*FimoModuleLoadingSuccessCallback)(const FimoModuleInfo*, void*);

/**
 * A callback for a module loading error.
 *
 * The callback function is called when the backend was not
 * successful in loading the requested module. The provider
 * of the function is then able to indicate to the backend,
 * whether or not to rollback the loading of the module set.
 *
 * @param arg0 module that caused the error
 * @param arg1 callback data
 *
 * @return `true`, if the loading should be reverted
 */
typedef bool (*FimoModuleLoadingErrorCallback)(const FimoModuleExport*, void*);

/**
 * Cleanup function for the callback data.
 *
 * Cleanup function for the user data passed to the
 * `FimoModuleLoadingSuccessCallback` and
 * `FimoModuleLoadingErrorCallback` callback functions.
 * Is called after the loading/rollback has been completed.
 *
 * @param arg0 user data to clean up
 */
typedef void (*FimoModuleLoadingCleanupCallback)(void*);

/**
 * Locks the module backend.
 *
 * The module backend is synchronized with a mutex. While the backend
 * is locked, the owner of the lock is allowed to have references to
 * modules it does not own.
 *
 * @param context the context.
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_lock(FimoContext context);

/**
 * Unlocks the module backend.
 *
 * The caller must ensure that they own a lock to the backend, and
 * that they don't have any references to symbols/modules that are
 * registered as dependencies of the module of the caller, as they
 * may be invalidated immediately after the unlock of the backend.
 *
 * @param context the context.
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_unlock(FimoContext context);

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
FIMO_MUST_USE FimoError fimo_module_pseudo_module_new(FimoContext context,
    const FimoModule** module);

/**
 * Destroys an existing pseudo module.
 *
 * By destroying the pseudo module, the caller ensures that they
 * relinquished all access to handles derived by the module backend.
 *
 * @param module pseudo module to destroy
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_pseudo_module_destroy(const FimoModule* module);

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
FIMO_MUST_USE FimoError fimo_module_set_new(FimoContext context,
    FimoModuleLoadingSet** module_set);

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
FIMO_MUST_USE FimoError fimo_module_set_has_module(FimoContext context,
    FimoModuleLoadingSet* module_set, const char* name, bool* has_module);

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
FIMO_MUST_USE FimoError fimo_module_set_has_symbol(FimoContext context,
    FimoModuleLoadingSet* module_set, const char* name, const char* ns,
    FimoVersion version, bool* has_symbol);

/**
 * Adds modules to the module set.
 *
 * Opens up a module binary to select which modules to load.
 * The binary path `module_path` must be encoded as `UTF-8`,
 * and point to the binary that contains the modules. The
 * binary path does not require to contain the file extension.
 * If the path is `Null`, it iterates over the exported modules
 * of the current binary. Each exported module is then passed to
 * the `filter`, along with the provided `filter_data`, which can
 * then filter which modules to load. This function may skip
 * invalid module exports. Trying to include a module with duplicate
 * exports will result in an error. This function signals an error,
 * if the binary does not contain the symbols necessary to query
 * the exported modules, but does not result in an error, if it
 * does not export any modules. The necessary symbols are setup
 * automatically, if the binary was linked with the fimo library.
 *
 * In addition, the caller can include status reporting callbacks
 * to the module set, that are called after the loading/dismissal
 * of the set.
 *
 * @param context the context
 * @param module_set set of modules to append to
 * @param module_path path to the binary to inspect
 * @param filter filter function
 * @param filter_data custom data to pass to the filter function
 * @param on_success success callback
 * @param on_error error callback
 * @param on_cleanup cleanup callback
 * @param user_data callback user data
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_set_append(FimoContext context,
    FimoModuleLoadingSet* module_set, const char* module_path,
    FimoModuleLoadingFilter filter, void* filter_data,
    FimoModuleLoadingSuccessCallback on_success,
    FimoModuleLoadingErrorCallback on_error,
    FimoModuleLoadingCleanupCallback on_cleanup, void* user_data);

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
FIMO_MUST_USE FimoError fimo_module_set_dismiss(FimoContext context,
    FimoModuleLoadingSet* module_set);

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
FIMO_MUST_USE FimoError fimo_module_set_finish(FimoContext context,
    FimoModuleLoadingSet* module_set);

/**
 * Searches for a module by it's name.
 *
 * Queries a module by its unique name.
 *
 * @param context context
 * @param name module name
 * @param module resulting module.
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_find_by_name(FimoContext context, const char* name,
    const FimoModuleInfo** module);

/**
 * Searches for a module by a symbol it exports.
 *
 * Queries the module that exported the specified symbol.
 *
 * @param context context
 * @param name symbol name
 * @param ns symbol namespace
 * @param version symbol version
 * @param module resulting module
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_find_by_symbol(FimoContext context, const char* name,
    const char* ns, FimoVersion version, const FimoModuleInfo** module);

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
FIMO_MUST_USE FimoError fimo_module_namespace_exists(FimoContext context, const char* ns,
    bool* exists);

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
FIMO_MUST_USE FimoError fimo_module_namespace_include(const FimoModule* module,
    const char* ns);

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
FIMO_MUST_USE FimoError fimo_module_namespace_exclude(const FimoModule* module,
    const char* ns);

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
FIMO_MUST_USE FimoError fimo_module_namespace_included(const FimoModule* module,
    const char* ns, bool* is_included, bool* is_static);

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
FIMO_MUST_USE FimoError fimo_module_acquire_dependency(const FimoModule* module,
    const FimoModuleInfo* dependency);

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
FIMO_MUST_USE FimoError fimo_module_relinquish_dependency(const FimoModule* module,
    const FimoModuleInfo* dependency);

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
FIMO_MUST_USE FimoError fimo_module_has_dependency(const FimoModule* module,
    const FimoModuleInfo* other, bool* has_dependency, bool* is_static);

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
FIMO_MUST_USE FimoError fimo_module_load_symbol(const FimoModule* module, const char* name,
    const char* ns, FimoVersion version, const void** symbol);

/**
 * Unloads a module.
 *
 * If successful, this function unloads the module `module` and all
 * modules that have it as a dependency. To succeed, the module
 * must have no dependencies left.
 *
 * @param context the context
 * @param module module to unload
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_module_unload(FimoContext context, const FimoModuleInfo* module);

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
FIMO_MUST_USE FimoError fimo_module_param_query(FimoContext context, const char* module_name,
    const char* param, FimoModuleParamType* type, FimoModuleParamAccess* read,
    FimoModuleParamAccess* write);

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
FIMO_MUST_USE FimoError fimo_module_param_set_public(FimoContext context, const void* value,
    FimoModuleParamType type, const char* module_name, const char* param);

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
FIMO_MUST_USE FimoError fimo_module_param_get_public(FimoContext context, void* value,
    FimoModuleParamType* type, const char* module_name, const char* param);

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
FIMO_MUST_USE FimoError fimo_module_param_set_dependency(const FimoModule* module, const void* value,
    FimoModuleParamType type, const char* module_name, const char* param);

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
FIMO_MUST_USE FimoError fimo_module_param_get_dependency(const FimoModule* module, void* value,
    FimoModuleParamType* type, const char* module_name, const char* param);

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
FIMO_MUST_USE FimoError fimo_module_param_set_private(const FimoModule* module, const void* value,
    FimoModuleParamType type, FimoModuleParam* param);

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
FIMO_MUST_USE FimoError fimo_module_param_get_private(const FimoModule* module, void* value,
    FimoModuleParamType* type, const FimoModuleParam* param);

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
FIMO_MUST_USE FimoError fimo_module_param_set_inner(const FimoModule* module, const void* value,
    FimoModuleParamType type, FimoModuleParamData* param);

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
FIMO_MUST_USE FimoError fimo_module_param_get_inner(const FimoModule* module, void* value,
    FimoModuleParamType* type, const FimoModuleParamData* param);

#ifdef __cplusplus
}
#endif

#endif // FIMO_MODULE_H
