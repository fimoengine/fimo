cmake_minimum_required(VERSION 3.15 FATAL_ERROR)
project(
    fimo_std
    VERSION 0.1.0
    LANGUAGES C
)
enable_language(C)

# Third party libraries
add_subdirectory(third_party)

set(FIMO_STD_HEADERS
    # Internal headers
    include/fimo_std/internal/context.h
    include/fimo_std/internal/module.h
    include/fimo_std/internal/tracing.h
    include/fimo_std/internal/unicode.h

    # Public headers
    include/fimo_std/array_list.h
    include/fimo_std/char.h
    include/fimo_std/context.h
    include/fimo_std/error.h
    include/fimo_std/graph.h
    include/fimo_std/memory.h
    include/fimo_std/module.h
    include/fimo_std/refcount.h
    include/fimo_std/time.h
    include/fimo_std/tracing.h
    include/fimo_std/utils.h
    include/fimo_std/version.h
)

set(FIMO_STD_SRC
    # Internal header implementations
    src/internal/context.c
    src/internal/module.c
    src/internal/tracing.c
    src/internal/unicode.c

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

option(FIMO_MACRO_HELPER_FUNCTIONS "Enable helper functions from macros" OFF)

add_library(fimo_std STATIC ${FIMO_STD_HEADERS} ${FIMO_STD_SRC})
target_link_libraries(fimo_std btree hashmap tinycthread)
set_property(TARGET fimo_std PROPERTY LINKER_LANGUAGE C)
set_property(TARGET fimo_std PROPERTY C_STANDARD 17)
set_property(TARGET fimo_std PROPERTY C_STANDARD_REQUIRED ON)
set_property(TARGET fimo_std PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")
target_include_directories(fimo_std PUBLIC 
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>  # <prefix>/include
)

if(FIMO_MACRO_HELPER_FUNCTIONS)
    target_compile_definitions(fimo_std PUBLIC FIMO_MACRO_HELPER_FUNCTIONS=TRUE)
endif()

if (MSVC)
    if (NOT CMAKE_C_COMPILER_ID STREQUAL "Clang")
        target_compile_options(fimo_std PUBLIC /experimental:c11atomics)
    endif()

    target_compile_options(fimo_std PUBLIC /W4 /WX /utf-8)
else()
    target_compile_options(fimo_std PUBLIC -Wno-documentation-unknown-command)
    target_compile_options(fimo_std PUBLIC -Wno-declaration-after-statement)
    target_compile_options(fimo_std PUBLIC -Wall -Wextra -pedantic -Werror)
    target_compile_options(fimo_std PUBLIC -pthread)
    target_compile_options(fimo_std PUBLIC -fexec-charset=UTF-8)
    target_compile_options(fimo_std PUBLIC -finput-charset=UTF-8)
endif()

if (ENABLE_TESTS)
    add_subdirectory(tests)
endif()

install(TARGETS fimo_std
    RUNTIME
    ARCHIVE
    PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/fimo
)