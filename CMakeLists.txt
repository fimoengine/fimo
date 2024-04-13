cmake_minimum_required(VERSION 3.28 FATAL_ERROR)
project(
        fimo
        VERSION 0.1.0
)

list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/cmake/cmocka)
list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/cmake/fimo)
include(FimoUtils)
include(CMakeDependentOption)

option(DISABLE_BINDINGS "Don't build bindings" OFF)
cmake_dependent_option(
        ENABLE_ALL_BINDINGS "Build all included bindings" ON
        "NOT DISABLE_BINDINGS" OFF
)

option(DISABLE_MODULES "Don't build modules" OFF)
cmake_dependent_option(
        ENABLE_ALL_MODULES "Build all included modules" ON
        "NOT DISABLE_MODULES;NOT DISABLE_BINDINGS" OFF
)

# Modules
new_fimo_module(PYTHON_MODULE_LOADER OFF)

option(TEST_MODULES "Enable the testing of modules" On)
option(TEST_BINDINGS "Enable the testing of the module bindings" On)

option(INSTALL_MODULES "Install the modules" ON)
option(INSTALL_BINDINGS "Install the c bindings for the modules" ON)

if (TEST_MODULES OR TEST_BINDINGS)
    find_package(cmocka 1.1.0 REQUIRED)
    include(AddCMockaTest)
endif ()

if (NOT ${DISABLE_BINDINGS})
    add_subdirectory(ffi_library)
endif ()

if (NOT ${DISABLE_MODULES})
    add_subdirectory(modules)
endif ()
