# Add CMocka CMake modules
list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../cmake/cmocka)

if (TEST_MODULES OR TEST_BINDINGS)
    find_package(cmocka 1.1.0 REQUIRED)
    include(AddCMockaTest)
endif ()

add_subdirectory(fimo_std)