find_package(Python COMPONENTS Interpreter REQUIRED)

if (NOT CMAKE_BUILD_TYPE STREQUAL "DEBUG" AND NOT CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(cpython_RELEASE "")
    set(RELEASE_COMMAND --release)
else ()
endif ()

include(FetchContent)
FetchContent_Declare(
        cpython
        GIT_REPOSITORY "https://github.com/python/cpython.git"
        GIT_TAG "3.13"
        GIT_SHALLOW TRUE
)
FetchContent_GetProperties(cpython)
if (NOT cpython_POPULATED)
    message(STATUS "Fetching CPython from git")
    FetchContent_Populate(cpython)
    message(STATUS "Configuring CPython")
    execute_process(
            COMMAND ${Python_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/configure.py --binary_dir ${cpython_BINARY_DIR} ${RELEASE_COMMAND}
            WORKING_DIRECTORY ${cpython_SOURCE_DIR}
            RESULT_VARIABLE cpython_CONFIGURE_RESULT
    )
    if (NOT ${cpython_CONFIGURE_RESULT} EQUAL 0)
        message(FATAL_ERROR "Could not configure CPython, status: ${cpython_CONFIGURE_RESULT}")
    endif ()
    if (WIN32)
        set(cpython_LIBRARY_NAME "python313.dll")
        set(cpython_LIBRARY_PATH "${cpython_BINARY_DIR}/build/${cpython_LIBRARY_NAME}")
        set(cpython_IMP_LIBRARY_PATH "${cpython_BINARY_DIR}/build/python313.lib")
    else ()
        if (NOT DEFINED cpython_RELEASE)
            set(cpython_LIBRARY_NAME "libpython3.13d${CMAKE_SHARED_LIBRARY_SUFFIX}")
        else ()
            set(cpython_LIBRARY_NAME "libpython3.13${CMAKE_SHARED_LIBRARY_SUFFIX}")
        endif ()
        set(cpython_LIBRARY_PATH "${cpython_BINARY_DIR}/build/${cpython_LIBRARY_NAME}")
        set(cpython_IMP_LIBRARY_PATH "")
    endif ()
    message(STATUS "CPython library: ${cpython_LIBRARY_PATH}")
    if (DEFINED cpython_IMP_LIBRARY_PATH)
        message(STATUS "CPython implementation library: ${cpython_IMP_LIBRARY_PATH}")
    endif ()
endif ()

add_custom_command(
        OUTPUT ${cpython_LIBRARY_PATH} ${cpython_IMP_LIBRARY_PATH}
        COMMAND
        ${Python_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/build.py --binary_dir ${cpython_BINARY_DIR} ${RELEASE_COMMAND}
        COMMAND
        ${CMAKE_COMMAND} -E copy_directory ${cpython_BINARY_DIR}/build/
        ${FIMO_CURRENT_MODULE_RESOURCE_BUILD_DIR}
        WORKING_DIRECTORY ${cpython_SOURCE_DIR}
        VERBATIM
)
add_custom_target(cpython_build DEPENDS ${cpython_LIBRARY_PATH})

add_library(cpython SHARED IMPORTED GLOBAL)
add_dependencies(cpython cpython_build)
set_target_properties(
        cpython PROPERTIES
        IMPORTED_LOCATION ${cpython_LIBRARY_PATH}
        INTERFACE_INCLUDE_DIRECTORIES ${cpython_BINARY_DIR}/include/cpython
)
if (WIN32)
    set_target_properties(cpython PROPERTIES IMPORTED_IMPLIB ${cpython_IMP_LIBRARY_PATH})
elseif (APPLE)
    set_target_properties(cpython PROPERTIES
            IMPORTED_SONAME "@rpath/${cpython_LIBRARY_NAME}"
    )
endif ()

if (FIMO_INSTALL_MODULES)
    install(
            DIRECTORY ${cpython_BINARY_DIR}/build/
            DESTINATION ${FIMO_CURRENT_MODULE_RESOURCE_INSTALL_DIR}
            PATTERN "*.lib" EXCLUDE
    )
endif ()