cmake_minimum_required(VERSION 3.15 FATAL_ERROR)
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
        include/fimo_std/char.h
        include/fimo_std/context.h
        include/fimo_std/error.h
        include/fimo_std/graph.h
        include/fimo_std/integers.h
        include/fimo_std/memory.h
        include/fimo_std/module.h
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
        include/fimo_std/impl/unicode.h
)

set(FIMO_STD_PRIVATE_HEADERS
        # Internal headers
        include/fimo_std/internal/context.h
        include/fimo_std/internal/module.h
        include/fimo_std/internal/tracing.h
)

list(APPEND FIMO_STD_HEADERS ${FIMO_STD_PUBLIC_HEADERS} ${FIMO_STD_IMPL_HEADERS} ${FIMO_STD_PRIVATE_HEADERS})

set(FIMO_STD_SRC
        # Internal header implementations
        src/internal/context.c
        src/internal/module.c
        src/internal/tracing.c

        # Public implementation header implementations
        src/impl/module.c
        src/impl/unicode.c

        # Public header implementations
        src/array_list.c
        src/char.c
        src/context.c
        src/error.c
        src/graph.c
        src/memory.c
        src/module.c
        src/refcount.c
        src/time.c
        src/tracing.c
        src/version.c
)

add_library(fimo_std STATIC)
target_sources(fimo_std PUBLIC
        FILE_SET fimo_std_public_headers
        TYPE HEADERS
        FILES ${FIMO_STD_PUBLIC_HEADERS} ${FIMO_STD_IMPL_HEADERS}
)
target_sources(fimo_std PRIVATE
        FILE_SET fimo_std_private_headers
        TYPE HEADERS
        FILES ${FIMO_STD_PRIVATE_HEADERS}
)
target_sources(fimo_std PRIVATE ${FIMO_STD_SRC})
target_link_libraries(fimo_std PRIVATE btree hashmap tinycthread ${CMAKE_THREAD_LIBS_INIT})
set_property(TARGET fimo_std PROPERTY LINKER_LANGUAGE C)
set_property(TARGET fimo_std PROPERTY C_STANDARD 17)
set_property(TARGET fimo_std PROPERTY C_STANDARD_REQUIRED ON)
set_property(TARGET fimo_std PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")
target_include_directories(fimo_std PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>  # <prefix>/include
)
set_property(TARGET fimo_std PROPERTY
        MSVC_DEBUG_INFORMATION_FORMAT "$<$<CONFIG:Debug,RelWithDebInfo>:Embedded>")

if (WIN32)
    target_link_libraries(fimo_std PRIVATE Pathcch)
endif ()

if (MSVC)
    if (NOT CMAKE_C_COMPILER_ID STREQUAL "Clang")
        target_compile_options(fimo_std PUBLIC /std:c17)
        target_compile_options(fimo_std PUBLIC /Zc:preprocessor)
        target_compile_options(fimo_std PUBLIC /experimental:c11atomics)
    endif ()

    target_compile_options(fimo_std PUBLIC /W4 /WX /utf-8 /Brepro)
else ()
    target_compile_options(fimo_std PUBLIC -Wno-documentation-unknown-command)
    target_compile_options(fimo_std PUBLIC -Wno-declaration-after-statement)
    target_compile_options(fimo_std PUBLIC -Wall -Wextra -pedantic -Werror)
    target_compile_options(fimo_std PUBLIC -fexec-charset=UTF-8)
    target_compile_options(fimo_std PUBLIC -finput-charset=UTF-8)
endif ()

if (ENABLE_TESTS)
    add_subdirectory(tests)
endif ()

include(GNUInstallDirs)
install(TARGETS fimo_std
        ARCHIVE
        FILE_SET fimo_std_public_headers
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/fimo_std
)