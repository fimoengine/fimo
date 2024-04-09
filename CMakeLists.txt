cmake_minimum_required(VERSION 3.28 FATAL_ERROR)
project(
        fimo
        VERSION 0.1.0
)

option(ENABLE_ALL_MODULES "Build all included modules" ON)
option(ENABLE_PYTHON_MODULE_LOADER "Enables the Python module loader module" ON)

option(TEST_MODULES "Enable the testing of modules" On)
option(TEST_BINDINGS "Enable the testing of the module bindings" On)

option(INSTALL_MODULES "Install the modules" ON)
option(INSTALL_BINDINGS "Install the c bindings for the modules" ON)

add_subdirectory(ffi_library)
add_subdirectory(modules)
