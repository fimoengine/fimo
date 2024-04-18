#include <fimo_python_module_loader/loader.h>

#include <fimo_std/memory.h>
#include <fimo_std/tracing.h>
#include <fimo_std/utils.h>

#include <stdio.h>
#include <stdlib.h>

FIMO_PRAGMA_MSVC(warning(push))
FIMO_PRAGMA_MSVC(warning(disable : 4244))
#include <Python.h>
FIMO_PRAGMA_MSVC(warning(pop))

#ifdef _WIN32
#define MODULE_FILE_NAME "module.dll"
#elif __APPLE__
#define MODULE_FILE_NAME "module.dylib"
#else
#define MODULE_FILE_NAME "module.so"
#endif

static FimoError construct_module_(const FimoModule *module, FimoModuleLoadingSet *set, void **data);
static void destroy_module_(const FimoModule *module, void *data);
static FimoError run_string_(void *data, const char *code, const char *home);
static FimoError construct_run_string_(const FimoModule *module, void **symbol);
static void destroy_run_string_(void *symbol);

static FimoModuleResourceDecl module_resources[] = {
        FIMO_MODULE_RESOURCE(""),
        FIMO_MODULE_RESOURCE(MODULE_FILE_NAME),
#ifndef _WIN32
        FIMO_MODULE_RESOURCE("Lib"),
        FIMO_MODULE_RESOURCE("Lib/lib-dynload"),
#endif
};
#ifdef WIN32
FIMO_MODULE_RESOURCE_TABLE(
        struct ResourceTable, 2, struct ResourceTable {
            FIMO_MODULE_RESOURCE_TABLE_PARAM(home);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(module_path);
        })
#else
FIMO_MODULE_RESOURCE_TABLE(
        struct ResourceTable, 4, struct ResourceTable {
            FIMO_MODULE_RESOURCE_TABLE_PARAM(home);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(module_path);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(lib_path);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(dynload_path);
        })
#endif
static FimoModuleDynamicSymbolExport module_dynamic_exports[] = {FIMO_MODULE_EXPORT_DYNAMIC_SYMBOL_NS(
        FIPY_SYMBOL_NAME_RUN_STRING, FIPY_SYMBOL_NAMESPACE, FIPY_SYMBOL_VERSION_MAJOR_RUN_STRING,
        FIPY_SYMBOL_VERSION_MINOR_RUN_STRING, FIPY_SYMBOL_VERSION_PATCH_RUN_STRING, construct_run_string_,
        destroy_run_string_)};

FIMO_MODULE_EXPORT_MODULE(FIMO_CURRENT_MODULE_NAME, "Loader for Python modules", "fimo", "MIT + APACHE 2.0",
                          FIMO_MODULE_EXPORT_MODULE_RESOURCES(module_resources),
                          FIMO_MODULE_EXPORT_MODULE_DYNAMIC_SYMBOL_EXPORTS(module_dynamic_exports),
                          FIMO_MODULE_EXPORT_MODULE_CONSTRUCTOR(construct_module_, destroy_module_))

static FimoError construct_module_(const FimoModule *module, FimoModuleLoadingSet *set, void **data) {
    FIMO_TRACING_EMIT_TRACE_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "initializing module")
    (void)set;

    const struct ResourceTable *resource_table = module->resources;

    PyConfig config;
    PyConfig_InitIsolatedConfig(&config);

    PyStatus status = PyConfig_SetBytesString(&config, &config.home, resource_table->home);
    if (PyStatus_Exception(status)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "could not set home path");
        goto cleanup_config;
    }

    status = PyConfig_SetBytesString(&config, &config.program_name, resource_table->module_path);
    if (PyStatus_Exception(status)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not set program name");
        goto cleanup_config;
    }

#ifndef _WIN32
    wchar_t *path = Py_DecodeLocale(resource_table->lib_path, NULL);
    if (path == NULL) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not decode the library path");
        goto cleanup_config;
    }

    status = PyWideStringList_Append(&config.module_search_paths, path);
    PyMem_RawFree(path);
    if (PyStatus_Exception(status)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not append to module search path");
        goto cleanup_config;
    }

    path = Py_DecodeLocale(resource_table->dynload_path, NULL);
    if (path == NULL) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not decode the library dynload path");
        goto cleanup_config;
    }

    status = PyWideStringList_Append(&config.module_search_paths, path);
    PyMem_RawFree(path);
    if (PyStatus_Exception(status)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not append to module search path");
        goto cleanup_config;
    }
    config.module_search_paths_set = 1;
#endif

    status = Py_InitializeFromConfig(&config);
    if (PyStatus_Exception(status)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not initialize the python interpreter");
        goto cleanup_config;
    }
    PyConfig_Clear(&config);

    // Release the GIL and save the main thread state.
    PyThreadState *state = PyEval_SaveThread();
    *data = state;

    FIMO_TRACING_EMIT_TRACE_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "module initialized")

    return FIMO_EOK;

cleanup_config:
    PyConfig_Clear(&config);
    return FIMO_EUNKNOWN;
}

static void destroy_module_(const FimoModule *module, void *data) {
    FIMO_DEBUG_ASSERT(data)
    FIMO_TRACING_EMIT_TRACE_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "destroying module")
    (void)module;

    // Acquire the GIL using the main thread state.
    PyThreadState *state = data;
    PyEval_RestoreThread(state);

    const int result = Py_FinalizeEx();
    if (result != 0) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not finalize the Python interpreter");
    }
    FIMO_DEBUG_ASSERT_FALSE(result != 0)
}

static FimoError run_string_(void *data, const char *code, const char *home) {
    FIMO_DEBUG_ASSERT(data)
    const FimoModule *module = data;
    if (code == NULL) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "no code provided")
        return FIMO_EINVAL;
    }
    FIMO_TRACING_EMIT_TRACE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                            "executing string\nHome: %s\nCode:\n%s", home ? home : "no home set", code)

    // Create a new thread state and acquire the GIL.
    PyInterpreterState *main_interpreter = PyInterpreterState_Main();
    PyThreadState *state = PyThreadState_New(main_interpreter);
    PyEval_RestoreThread(state);

    // With the GIL we create a new subinterpreter.
    PyInterpreterConfig config = {
            .use_main_obmalloc = 0,
            .allow_fork = 0,
            .allow_exec = 0,
            .allow_threads = 1,
            .allow_daemon_threads = 0,
            .check_multi_interp_extensions = 1,
            .gil = PyInterpreterConfig_OWN_GIL,
    };
    PyThreadState *sub_state;
    PyStatus status = Py_NewInterpreterFromConfig(&sub_state, &config);
    if (PyStatus_Exception(status)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not create a new subinterpreter");
        goto destroy_main_state;
    }

    // Append the path if it was provided.
    if (home != NULL) {
        PyObject *path = PySys_GetObject("path");
        FIMO_DEBUG_ASSERT(path)

        // Create a string for the home path.
        PyObject *home_object = PyUnicode_FromString(home);
        if (home_object == NULL) {
            PyObject *ex = PyErr_Occurred();
            FIMO_DEBUG_ASSERT(ex)
            PyErr_DisplayException(ex);
            PyErr_Clear();
            FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                           "could not create the home path object");
            goto release_interpreter;
        }

        // Append the home path to the module search dirs.
        const int append_result = PyList_Append(path, home_object);
        Py_DecRef(home_object);
        if (append_result != 0) {
            PyObject *ex = PyErr_Occurred();
            FIMO_DEBUG_ASSERT(ex)
            PyErr_DisplayException(ex);
            PyErr_Clear();
            FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                           "could not append the home path to the module search dirs");
            goto release_interpreter;
        }
    }

    // Check that the code can be compiled.
    PyObject *compiled_code = Py_CompileString(code, "<string_eval>", Py_file_input);
    if (compiled_code == NULL) {
        PyObject *ex = PyErr_Occurred();
        FIMO_DEBUG_ASSERT(ex)
        PyErr_DisplayException(ex);
        PyErr_Clear();
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "could not compile code");
        goto release_interpreter;
    }

    // Execute the code.
    PyObject *code_module = PyImport_ExecCodeModule("__main__", compiled_code);
    Py_DecRef(compiled_code);
    if (code_module == NULL) {
        PyObject *ex = PyErr_Occurred();
        FIMO_DEBUG_ASSERT(ex)
        PyErr_DisplayException(ex);
        PyErr_Clear();
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "could not execute code");
        goto release_interpreter;
    }
    Py_DecRef(code_module);

    // Now that we finished our execution we destroy the interpreter.
    Py_EndInterpreter(sub_state);

    // Acquire the GIL of the main interpreter and destroy the thread state.
    PyEval_RestoreThread(state);
    PyThreadState_Clear(state);
    PyThreadState_DeleteCurrent();

    return FIMO_EOK;

release_interpreter:
    Py_EndInterpreter(sub_state);
destroy_main_state:
    PyEval_RestoreThread(state);
    PyThreadState_Clear(state);
    PyThreadState_DeleteCurrent();

    return FIMO_EUNKNOWN;
}

static FimoError construct_run_string_(const FimoModule *module, void **symbol) {
    FIMO_DEBUG_ASSERT(module && symbol)
    FIMO_TRACING_EMIT_TRACE_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME, "initializing 'run_string'")
    FimoError error;
    FipyRunString **run_string = (FipyRunString **)symbol;
    *run_string = fimo_malloc(sizeof(**run_string), &error);
    if (FIMO_IS_ERROR(error)) {
        FIMO_TRACING_EMIT_ERROR_SIMPLE(module->context, __func__, FIMO_CURRENT_MODULE_NAME,
                                       "could not allocate symbol");
    }
    **run_string = (FipyRunString){
            .data = (void *)module,
            .func = run_string_,
    };

    return FIMO_EOK;
}

static void destroy_run_string_(void *symbol) {
    FIMO_DEBUG_ASSERT(symbol);
    fimo_free(symbol);
}
