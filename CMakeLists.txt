cmake_minimum_required(VERSION 3.28 FATAL_ERROR)
cmake_policy(SET CMP0087 NEW)
cmake_policy(SET CMP0141 NEW)
project(
        fimo
        VERSION 0.1.0
)
enable_language(C)
enable_language(CXX)

list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/cmake/fimo)
include(FimoUtils)
include(CMakeDependentOption)

option(FIMO_DISABLE_BINDINGS "Don't build bindings" OFF)
cmake_dependent_option(
        FIMO_ENABLE_ALL_BINDINGS "Build all included bindings" ON
        "NOT FIMO_DISABLE_BINDINGS" OFF
)

option(FIMO_DISABLE_MODULES "Don't build modules" OFF)
cmake_dependent_option(
        FIMO_ENABLE_ALL_MODULES "Build all included modules" ON
        "NOT FIMO_DISABLE_MODULES;NOT FIMO_DISABLE_BINDINGS" OFF
)

option(FIMO_TEST_MODULES "Enable the testing of modules" On)
option(FIMO_TEST_BINDINGS "Enable the testing of the module bindings" On)

option(FIMO_INSTALL_MODULES "Install the modules" ON)
option(FIMO_INSTALL_BINDINGS "Install the c bindings for the modules" ON)

# Bindings
fimo_declare_bindings(fimo_std ON)
fimo_declare_bindings(python_module_loader ON)

# Modules
fimo_declare_module(python_module_loader ON)

if (FIMO_TEST_MODULES OR FIMO_TEST_BINDINGS)
    Include(FetchContent)
    FetchContent_Declare(
            Catch2
            GIT_REPOSITORY https://github.com/catchorg/Catch2.git
            GIT_TAG v3.5.4
    )
    FetchContent_MakeAvailable(Catch2)
    list(APPEND CMAKE_MODULE_PATH ${catch2_SOURCE_DIR}/extras)
    set_property(GLOBAL PROPERTY CTEST_TARGETS_ADDED 1)
    include(CTest)
    include(Catch)
endif ()

add_subdirectory(ffi_library)
add_subdirectory(modules)
