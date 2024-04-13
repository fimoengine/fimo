cmake_minimum_required(VERSION 3.25 FATAL_ERROR)
cmake_policy(SET CMP0141 NEW)
project(
        fimo_python_module_loader
        VERSION 0.1.0
        LANGUAGES C
)
enable_language(C)

fimo_require_bindings(PYTHON_MODULE_LOADER)

set(MODULE_INSTALL_DIR "${FIMO_MODULES_INSTALL_DIR}/python_module_loader")
add_subdirectory(third_party)

add_executable(python_test src/main.c)
target_link_libraries(python_test PRIVATE cpython fimo_std)
set_property(TARGET python_test PROPERTY
        MSVC_DEBUG_INFORMATION_FORMAT "$<$<CONFIG:Debug,RelWithDebInfo>:Embedded>")

if (UNIX AND NOT APPLE)
    set_target_properties(python_test PROPERTIES INSTALL_RPATH "$ORIGIN/")
elseif (UNIX)
    set_target_properties(python_test PROPERTIES INSTALL_RPATH "@executable_path/")
endif ()

if (INSTALL_MODULES)
    install(TARGETS python_test DESTINATION ${MODULE_INSTALL_DIR})
endif ()