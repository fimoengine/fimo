ExternalProject_Add(cpython
        BUILD_ALWAYS ON
        CONFIGURE_COMMAND ""
        BUILD_COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/cpython/PCbuild/get_externals.bat && ${CMAKE_CURRENT_SOURCE_DIR}/cpython/PCbuild/build.bat -e -c Release
        BUILD_BYPRODUCTS ${CMAKE_CURRENT_SOURCE_DIR}/cpython/PCbuild/amd64/python.lib
        INSTALL_COMMAND ""
        SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/cpython
        PREFIX "cpython"
        LOG_BUILD ON
        LOG_OUTPUT_ON_FAILURE ON
        BUILD_IN_SOURCE 1
)