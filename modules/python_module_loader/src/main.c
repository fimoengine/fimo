#include <fimo_std/utils.h>

#include <stdio.h>
#include <stdlib.h>

FIMO_PRAGMA_MSVC(warning(push))
FIMO_PRAGMA_MSVC(warning(disable : 4244))
#include <Python.h>
FIMO_PRAGMA_MSVC(warning(pop))

int main(int argc, char *argv[]) {
    (void)argc;

    PyConfig config;
    PyConfig_InitIsolatedConfig(&config);

    PyStatus status = PyConfig_SetString(&config, &config.home, L".");
    if (PyStatus_Exception(status)) {
        goto init_error;
    }

    wchar_t *program = Py_DecodeLocale(argv[0], NULL);
    if (program == NULL) {
        fprintf(stderr, "Fatal error: cannot decode argv[0]\n");
        exit(1);
    }
    status = PyConfig_SetString(&config, &config.program_name, program);
    PyMem_RawFree(program);
    if (PyStatus_Exception(status)) {
        goto init_error;
    }

#ifdef _WIN32
#else
    status = PyWideStringList_Append(&config.module_search_paths, L"./Lib");
    if (PyStatus_Exception(status)) {
        goto init_error;
    }

    status = PyWideStringList_Append(&config.module_search_paths, L"./Lib/lib-dynload");
    if (PyStatus_Exception(status)) {
        goto init_error;
    }
    config.module_search_paths_set = 1;
#endif

    status = Py_InitializeFromConfig(&config);
    if (PyStatus_Exception(status)) {
        goto init_error;
    }
    PyConfig_Clear(&config);

    return PyRun_InteractiveLoop(stdin, "<stdin>");

init_error:
    PyConfig_Clear(&config);
    if (PyStatus_IsExit(status)) {
        return status.exitcode;
    }
    Py_ExitStatusException(status);
}
