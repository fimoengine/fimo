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
    include/fimo_std/error.h
    include/fimo_std/utils.h
)

set(FIMO_STD_SRC
    src/error.c
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