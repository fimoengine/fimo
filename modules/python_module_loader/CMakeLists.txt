cmake_minimum_required(VERSION 3.15 FATAL_ERROR)
project(
        fimo_python_module_loader
        VERSION 0.1.0
        LANGUAGES C
)
enable_language(C)

include(ExternalProject)
add_subdirectory(third_party)
add_subdirectory(../../ffi_library ${CMAKE_CURRENT_BINARY_DIR}/ffi_library)

install(DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/third_party/cpython/Lib/ DESTINATION "Lib")
if (WIN32)
    install(DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/third_party/cpython/PCbuild/amd64/
            DESTINATION "."
            FILES_MATCHING PATTERN "*.dll" PATTERN "*.pyd"
    )
endif ()