cmake_minimum_required(VERSION 3.25 FATAL_ERROR)
cmake_policy(SET CMP0141 NEW)
project(
        fimo_std
        VERSION 0.1.0
        LANGUAGES C
)
enable_language(C)

find_package(Threads REQUIRED)

if (WIN32)
    set(fimo_std_zig_shared_LIBRARY_NAME "fimo_std_shared.dll")
    set(fimo_std_zig_shared_LIBRARY_PATH "${CMAKE_CURRENT_BINARY_DIR}/zig-out/bin/fimo_std_shared.dll")
    set(fimo_std_zig_shared_IMP_LIBRARY_PATH "${CMAKE_CURRENT_BINARY_DIR}/zig-out/lib/fimo_std_shared.lib")
    set(fimo_std_zig_LIBRARY_PATH "${CMAKE_CURRENT_BINARY_DIR}/zig-out/lib/fimo_std.lib")
else ()
    set(fimo_std_zig_shared_LIBRARY_NAME "libfimo_std_shared.so")
    set(fimo_std_zig_shared_LIBRARY_PATH "${CMAKE_CURRENT_BINARY_DIR}/zig-out/lib/libfimo_std_shared.so")
    set(fimo_std_zig_shared_IMP_LIBRARY_PATH "")
    set(fimo_std_zig_LIBRARY_PATH "${CMAKE_CURRENT_BINARY_DIR}/zig-out/lib/libfimo_std.a")
endif ()

add_custom_target(fimo_std_zig_build
        COMMAND zig build
        --prefix ${CMAKE_CURRENT_BINARY_DIR}/zig-out
        --cache-dir ${CMAKE_BINARY_DIR}/.zig-cache
        $<$<NOT:$<CONFIG:Debug>>:--release=safe>
        BYPRODUCTS ${CMAKE_CURRENT_BINARY_DIR}/zig-out/include
        ${fimo_std_zig_shared_LIBRARY_PATH}
        ${fimo_std_zig_shared_IMP_LIBRARY_PATH}
        ${fimo_std_zig_LIBRARY_PATH}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

add_library(fimo_std_zig STATIC IMPORTED GLOBAL)
add_dependencies(fimo_std_zig fimo_std_zig_build)
set_target_properties(
        fimo_std_zig PROPERTIES
        IMPORTED_LOCATION ${fimo_std_zig_LIBRARY_PATH}
)

add_library(fimo_std INTERFACE)
target_include_directories(fimo_std INTERFACE "${CMAKE_CURRENT_BINARY_DIR}/zig-out/include")
target_link_libraries(fimo_std INTERFACE fimo_std_zig)
if (WIN32)
    target_link_libraries(fimo_std INTERFACE "Pathcch" "ntdll")
endif ()

add_library(fimo_std_shared SHARED IMPORTED GLOBAL)
target_include_directories(fimo_std_shared INTERFACE "${CMAKE_CURRENT_BINARY_DIR}/zig-out/include")
add_dependencies(fimo_std_shared fimo_std_zig_build)
set_target_properties(
        fimo_std_shared PROPERTIES
        IMPORTED_LOCATION ${fimo_std_zig_shared_LIBRARY_PATH}
        INTERFACE_INCLUDE_DIRECTORIES ${CMAKE_CURRENT_BINARY_DIR}/zig-out/include
)
if (WIN32)
    set_target_properties(fimo_std_shared PROPERTIES IMPORTED_IMPLIB ${fimo_std_zig_shared_IMP_LIBRARY_PATH})
elseif (APPLE)
    set_target_properties(fimo_std_shared PROPERTIES
            IMPORTED_SONAME "@rpath/${fimo_std_zig_shared_LIBRARY_NAME}"
    )
endif ()

if (WIN32 AND NOT CMAKE_C_COMPILER_ID STREQUAL "Clang")
    target_compile_options(fimo_std INTERFACE /experimental:c11atomics)
    target_compile_options(fimo_std_shared INTERFACE /experimental:c11atomics)
endif ()

if (FIMO_TEST_BINDINGS)
    add_subdirectory(tests)
endif ()

if (FIMO_INSTALL_BINDINGS)
    install(DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/zig-out/" DESTINATION ${CMAKE_INSTALL_PREFIX})
endif ()

