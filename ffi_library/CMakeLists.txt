cmake_minimum_required(VERSION 3.15 FATAL_ERROR)
project(
    fimo
    VERSION 0.1.0
)

# Add CMocka CMake modules
list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../cmake/cmocka)

option(ENABLE_TESTS "Enable the compilation of tests" On)

if (ENABLE_TESTS)
    find_package(cmocka 1.1.0 REQUIRED)
    include(AddCMockaTest)
endif()

add_subdirectory(fimo_std)