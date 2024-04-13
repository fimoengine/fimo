find_package(Python COMPONENTS Interpreter REQUIRED)

if (NOT CMAKE_BUILD_TYPE STREQUAL "DEBUG" AND NOT CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(cpython_RELEASE OFF)
    set(RELEASE_COMMAND --release)
endif ()

include(FetchContent)
FetchContent_Declare(
        cpython
        GIT_REPOSITORY "https://github.com/python/cpython.git"
        GIT_TAG "main"
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
        if (NOT cpython_RELEASE)
            set(cpython_LIBRARY_PATH "${cpython_BINARY_DIR}/build/python313_d.dll")
            set(cpython_IMP_LIBRARY_PATH "${cpython_BINARY_DIR}/build/python313_d.lib")
        else ()
            set(cpython_LIBRARY_PATH "${cpython_BINARY_DIR}/build/python313.dll")
            set(cpython_IMP_LIBRARY_PATH "${cpython_BINARY_DIR}/build/python313.lib")
        endif ()
    else ()
        if (NOT cpython_RELEASE)
            set(cpython_LIBRARY_PATH "${cpython_BINARY_DIR}/build/libpython3.13d${CMAKE_SHARED_LIBRARY_SUFFIX}")
        else ()
            set(cpython_LIBRARY_PATH "${cpython_BINARY_DIR}/build/libpython3.13${CMAKE_SHARED_LIBRARY_SUFFIX}")
        endif ()
        set(cpython_IMP_LIBRARY_PATH "")
    endif ()
    message(STATUS "CPython library: ${cpython_LIBRARY_PATH}")
endif ()

add_custom_command(
        OUTPUT ${cpython_LIBRARY_PATH} ${cpython_IMP_LIBRARY_PATH}
        COMMAND ${Python_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/build.py --binary_dir ${cpython_BINARY_DIR} ${RELEASE_COMMAND}
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
endif ()

if (INSTALL_MODULES)
    install(
            DIRECTORY ${cpython_BINARY_DIR}/build/
            DESTINATION ${MODULE_INSTALL_DIR}
            PATTERN "*.lib" EXCLUDE
    )
endif ()