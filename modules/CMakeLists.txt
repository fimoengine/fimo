include(ExternalProject)

if (ENABLE_ALL_MODULES OR ENABLE_PYTHON_MODULE_LOADER)
    ExternalProject_Add(python_module_loader
            BUILD_ALWAYS ON
            PREFIX ${CMAKE_CURRENT_BINARY_DIR}/python_module_loader
            SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/python_module_loader
            CMAKE_ARGS -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/python_module_loader/build -DCMOCKA_PATH:PATH=${CMOCKA_PATH} -DTEST_MODULES=${TEST_MODULES}
    )
    if (INSTALL_MODULES)
        install(DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/python_module_loader/build/ DESTINATION "modules/python_module_loader")
    endif ()
endif ()