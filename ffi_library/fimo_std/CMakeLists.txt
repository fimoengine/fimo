cmake_minimum_required(VERSION 3.25 FATAL_ERROR)
cmake_policy(SET CMP0141 NEW)
project(
        fimo_std
        VERSION 0.1.0
        LANGUAGES C
)
enable_language(C)

find_package(Threads REQUIRED)

# Third party libraries
add_subdirectory(third_party)

set(FIMO_STD_PUBLIC_HEADERS
        # Public headers
        include/fimo_std/array_list.h
        include/fimo_std/context.h
        include/fimo_std/error.h
        include/fimo_std/graph.h
        include/fimo_std/integers.h
        include/fimo_std/memory.h
        include/fimo_std/module.h
        include/fimo_std/path.h
        include/fimo_std/refcount.h
        include/fimo_std/time.h
        include/fimo_std/tracing.h
        include/fimo_std/utils.h
        include/fimo_std/version.h
        include/fimo_std/vtable.h
)

set(FIMO_STD_IMPL_HEADERS
        # Public implementation headers
        include/fimo_std/impl/integers/add_overflow.h
        include/fimo_std/impl/integers/count_ones.h
        include/fimo_std/impl/integers/integers_base.h
        include/fimo_std/impl/integers/mul_overflow.h
        include/fimo_std/impl/integers/next_power_of_two.h
        include/fimo_std/impl/integers/saturating_add.h
        include/fimo_std/impl/integers/saturating_sub.h
        include/fimo_std/impl/integers/sub_overflow.h
        include/fimo_std/impl/macros/assert.h
        include/fimo_std/impl/macros/concat.h
        include/fimo_std/impl/macros/container_of.h
        include/fimo_std/impl/macros/export.h
        include/fimo_std/impl/macros/has_builtin.h
        include/fimo_std/impl/macros/inline.h
        include/fimo_std/impl/macros/msvc_sal.h
        include/fimo_std/impl/macros/must_use.h
        include/fimo_std/impl/macros/pragma.h
        include/fimo_std/impl/macros/print_format.h
        include/fimo_std/impl/macros/stringify.h
        include/fimo_std/impl/macros/type_eq.h
        include/fimo_std/impl/macros/var.h
        include/fimo_std/impl/module.h
        include/fimo_std/impl/tracing.h
)

set(FIMO_STD_PRIVATE_HEADERS
        # Internal headers
        include/fimo_std/internal/context.h
        include/fimo_std/internal/module.h
        include/fimo_std/internal/path.h
        include/fimo_std/internal/tracing.h
)

set(FIMO_STD_SRC
        # Internal header implementations
        src/internal/context.c
        src/internal/module.c
        src/internal/tracing.c

        # Public implementation header implementations
        src/impl/module.c
        src/impl/tracing.c

        # Public header implementations
        src/array_list.c
        src/context.c
        src/error.c
        src/graph.c
        src/memory.c
        src/module.c
        src/path.c
        src/refcount.c
        src/time.c
        src/tracing.c
        src/version.c
)

# Visualizers
set(FIMO_STD_NATVIS
        ${CMAKE_CURRENT_SOURCE_DIR}/visualizers/natvis/path.natvis
)

set(FIMO_STD_GDB_SCRIPTS
        ${CMAKE_CURRENT_SOURCE_DIR}/visualizers/gdb/path.py
)

# Generate source files for the visualizers
if (UNIX AND NOT APPLE)
    file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/include/fimo_std/impl/gdb_scripts")
    foreach (GDB_SCRIPT IN LISTS FIMO_STD_GDB_SCRIPTS)
        get_filename_component(GDB_SCRIPT_NAME ${GDB_SCRIPT} NAME_WE)
        add_custom_command(OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/include/fimo_std/impl/gdb_scripts/${GDB_SCRIPT_NAME}.h"
                COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/visualizers/gdb/generate.sh ${GDB_SCRIPT} > "${CMAKE_CURRENT_BINARY_DIR}/include/fimo_std/impl/gdb_scripts/${GDB_SCRIPT_NAME}.h"
                DEPENDS ${GDB_SCRIPT} "${CMAKE_CURRENT_SOURCE_DIR}/visualizers/gdb/generate.sh"
                VERBATIM
        )
        list(APPEND FIMO_STD_IMPL_HEADERS "${CMAKE_CURRENT_BINARY_DIR}/include/fimo_std/impl/gdb_scripts/${GDB_SCRIPT_NAME}.h")
    endforeach ()
endif ()

# Add interface target for PUBLIC configs
add_library(fimo_std__public_config INTERFACE)
target_include_directories(fimo_std__public_config INTERFACE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>  # <prefix>/include
)
if (UNIX AND NOT APPLE)
    target_include_directories(fimo_std__public_config INTERFACE
            $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/include>
            $<INSTALL_INTERFACE:include>  # <prefix>/include
    )
endif()

if (MSVC)
    if (NOT CMAKE_C_COMPILER_ID STREQUAL "Clang")
        target_compile_options(fimo_std__public_config INTERFACE /experimental:c11atomics)
    endif ()

    target_compile_options(fimo_std__public_config INTERFACE /W4 /WX /utf-8 /Brepro)

    foreach (NATVIS_FILE IN LISTS FIMO_STD_NATVIS)
        target_link_options(fimo_std__public_config INTERFACE /NATVIS:${NATVIS_FILE})
    endforeach ()
else ()
    target_compile_options(fimo_std__public_config INTERFACE -Wno-documentation-unknown-command)
    target_compile_options(fimo_std__public_config INTERFACE -Wno-declaration-after-statement)
    target_compile_options(fimo_std__public_config INTERFACE -Wall -Wextra -pedantic -Werror)
    target_compile_options(fimo_std__public_config INTERFACE -fexec-charset=UTF-8)
    target_compile_options(fimo_std__public_config INTERFACE -finput-charset=UTF-8)
endif ()

# Add interface target for PRIVATE configs
add_library(fimo_std__private_config INTERFACE)
target_sources(fimo_std__private_config INTERFACE
        FILE_SET fimo_std_private_headers
        TYPE HEADERS
        FILES ${FIMO_STD_PRIVATE_HEADERS}
)
target_sources(fimo_std__private_config INTERFACE ${FIMO_STD_SRC})
target_compile_features(fimo_std__private_config INTERFACE c_std_17)
if (WIN32)
    target_link_libraries(fimo_std__private_config INTERFACE Pathcch)
endif ()

add_library(fimo_std STATIC)
target_link_libraries(fimo_std PUBLIC fimo_std__public_config)
target_link_libraries(fimo_std PRIVATE fimo_std__private_config)
target_sources(fimo_std PUBLIC
        FILE_SET fimo_std_public_headers
        TYPE HEADERS
        BASE_DIRS ${CMAKE_CURRENT_SOURCE_DIR} ${CMAKE_CURRENT_BINARY_DIR}
        FILES ${FIMO_STD_PUBLIC_HEADERS} ${FIMO_STD_IMPL_HEADERS}
)
target_link_libraries(fimo_std PRIVATE btree hashmap ${CMAKE_THREAD_LIBS_INIT})
set_property(TARGET fimo_std PROPERTY POSITION_INDEPENDENT_CODE ON)
set_property(TARGET fimo_std PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")
set_property(TARGET fimo_std PROPERTY
        MSVC_DEBUG_INFORMATION_FORMAT "$<$<CONFIG:Debug,RelWithDebInfo>:Embedded>")

add_library(fimo_std_shared SHARED)
target_link_libraries(fimo_std_shared PUBLIC fimo_std__public_config)
target_link_libraries(fimo_std_shared PRIVATE fimo_std__private_config)
target_compile_definitions(fimo_std_shared PUBLIC FIMO_STD_BUILD_SHARED)
target_compile_definitions(fimo_std_shared PRIVATE FIMO_STD_EXPORT_SYMBOLS)
target_sources(fimo_std_shared PUBLIC ${FIMO_STD_PUBLIC_HEADERS} ${FIMO_STD_IMPL_HEADERS})
target_link_libraries(fimo_std_shared PRIVATE btree hashmap ${CMAKE_THREAD_LIBS_INIT})
set_property(TARGET fimo_std_shared PROPERTY POSITION_INDEPENDENT_CODE ON)
set_property(TARGET fimo_std_shared PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")
set_property(TARGET fimo_std_shared PROPERTY
        MSVC_DEBUG_INFORMATION_FORMAT "$<$<CONFIG:Debug,RelWithDebInfo>:Embedded>")

if (APPLE)
    target_link_libraries(fimo_std PRIVATE tinycthread)
    target_link_libraries(fimo_std_shared PRIVATE tinycthread)
endif ()

if (FIMO_TEST_BINDINGS)
    add_subdirectory(tests)
endif ()

if (FIMO_INSTALL_BINDINGS)
    include(GNUInstallDirs)
    install(TARGETS fimo_std
            ARCHIVE
            FILE_SET fimo_std_public_headers
            DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/fimo_std
    )
    install(TARGETS fimo_std_shared
            ARCHIVE
            RUNTIME
    )
endif ()
