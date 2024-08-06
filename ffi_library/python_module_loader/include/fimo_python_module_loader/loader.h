#ifndef FIPY_LOADER_H
#define FIPY_LOADER_H

#include <fimo_std/module.h>

/**
 * Namespace of the symbols exposed by the bindings.
 */
#define FIPY_SYMBOL_NAMESPACE "fimo_python"

/**
 * Name of the `run_string` symbol.
 */
#define FIPY_SYMBOL_NAME_RUN_STRING "run_string"

/**
 * Major version of the `run_string` symbol.
 */
#define FIPY_SYMBOL_VERSION_MAJOR_RUN_STRING 0

/**
 * Minor version of the `run_string` symbol.
 */
#define FIPY_SYMBOL_VERSION_MINOR_RUN_STRING 1

/**
 * Patch version of the `run_string` symbol.
 */
#define FIPY_SYMBOL_VERSION_PATCH_RUN_STRING 0

/**
 * Symbol table entry of the `run_string` symbol.
 *
 * @param NAME name of the entry in the symbol table
 */
#define FIPY_SYMBOL_TABLE_ENTRY_RUN_STRING(NAME) FIMO_MODULE_SYMBOL_TABLE_FUNC(NAME, FipyRunString)

/**
 * Name of the `load_module` symbol.
 */
#define FIPY_SYMBOL_NAME_LOAD_MODULE "load_module"

/**
 * Major version of the `load_module` symbol.
 */
#define FIPY_SYMBOL_VERSION_MAJOR_LOAD_MODULE 0

/**
 * Minor version of the `load_module` symbol.
 */
#define FIPY_SYMBOL_VERSION_MINOR_LOAD_MODULE 1

/**
 * Patch version of the `load_module` symbol.
 */
#define FIPY_SYMBOL_VERSION_PATCH_LOAD_MODULE 0

/**
 * Symbol table entry of the `load_module` symbol.
 *
 * @param NAME name of the entry in the symbol table
 */
#define FIPY_SYMBOL_TABLE_ENTRY_LOAD_MODULE(NAME) FIMO_MODULE_SYMBOL_TABLE_VAR(NAME, FipyLoadModule)

/**
 * Symbol `run_string`.
 */
typedef struct FipyRunString {
    void *data;
    FimoResult (*func)(void *data, const char *code, const char *home);
} FipyRunString;

/**
 * Symbol `load_module`.
 */
typedef struct FipyLoadModule {
    void *data;
    FimoResult (*func)(void *data, FimoModuleLoadingSet *set, const char *filepath);
} FipyLoadModule;

/**
 * Executes the passed string in the embedded python interpreter.
 *
 * This function spawns a new isolated subinterpreter and executes the
 * provided string. The interpreter only has access to the builtin
 * Python modules. By setting `home` to a non-`NULL` value, the caller
 * can append a custom path to the module search path of the new
 * subinterpreter. This allows for the import of custom python modules.
 *
 * @param symbol the symbol to call
 * @param code code string to execute
 * @param home optional path to custom python modules
 *
 * @return Status code.
 */
static FIMO_INLINE_ALWAYS FimoResult fipy_run_string(const FipyRunString *symbol, const char *code, const char *home) {
    FIMO_DEBUG_ASSERT(symbol)
    return symbol->func(symbol->data, code, home);
}

/**
 * Adds a new python module to the module loading set.
 *
 * The module will be initialized in an isolated subinterpreter, where
 * the only modules available are the builtin Python modules, and the
 * own packaged modules.
 *
 * @param symbol the symbol to call
 * @param set pointer to the set that will load the module
 * @param filepath path to the python module entry file
 *
 * @return Status code.
 */
static FIMO_INLINE_ALWAYS FimoResult fipy_load_module(const FipyLoadModule *symbol, FimoModuleLoadingSet *set,
                                                      const char *filepath) {
    FIMO_DEBUG_ASSERT(symbol)
    return symbol->func(symbol->data, set, filepath);
}

#endif // FIPY_LOADER_H
