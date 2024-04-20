include(GNUInstallDirs)
include(CMakeDependentOption)

macro(fimo_declare_bindings NAME ENABLE)
    if (NOT DEFINED FIMO_BINDINGS)
        set(FIMO_BINDINGS "")
    endif ()
    if (${NAME} IN_LIST FIMO_BINDINGS)
        message(FATAL_ERROR "Bindings ${NAME} were already declared")
    endif ()
    list(APPEND FIMO_BINDINGS ${NAME})
    cmake_dependent_option(FIMO_ENABLE_BINDINGS_${NAME} "Enables the ${NAME} bindings" ${ENABLE} "NOT FIMO_DISABLE_BINDINGS" OFF)
endmacro()

macro(fimo_declare_module NAME ENABLE)
    if (NOT DEFINED FIMO_MODULES)
        set(FIMO_MODULES "")
    endif ()
    if (${NAME} IN_LIST FIMO_MODULES)
        message(FATAL_ERROR "Module ${NAME} was already declared")
    endif ()
    list(APPEND FIMO_MODULES ${NAME})
    cmake_dependent_option(FIMO_ENABLE_MODULE_${NAME} "Enables the ${NAME} module" ${ENABLE} "NOT FIMO_DISABLE_MODULES" OFF)
endmacro()

function(fimo_add_module)
    if (NOT DEFINED FIMO_CURRENT_MODULE)
        message(FATAL_ERROR "Not currently building a module")
    endif ()

    if (NOT TARGET fimo_all)
        add_custom_target(fimo_all)
    endif ()

    # We always require the fimo_std bindings
    fimo_require_bindings(fimo_std)
    # Add a new MODULE library that links to fimo_std
    add_library(${FIMO_CURRENT_MODULE_TARGET} MODULE)
    add_dependencies(fimo_all ${FIMO_CURRENT_MODULE_TARGET})
    target_link_libraries(${FIMO_CURRENT_MODULE_TARGET} PRIVATE fimo_std)
    target_compile_definitions(
            ${FIMO_CURRENT_MODULE_TARGET} PRIVATE
            FIMO_CURRENT_MODULE_NAME="${FIMO_CURRENT_MODULE}"
    )

    if (WIN32)
        target_link_options(
                ${FIMO_CURRENT_MODULE_TARGET} PRIVATE
                "/INCLUDE:fimo_impl_module_export_iterator"
        )
    elseif (UNIX AND NOT APPLE)
        target_link_options(
                ${FIMO_CURRENT_MODULE_TARGET} PRIVATE
                "LINKER:--undefined=fimo_impl_module_export_iterator"
        )
    elseif (APPLE)
        target_link_options(
                ${FIMO_CURRENT_MODULE_TARGET} PRIVATE
                "LINKER:-u"
                "LINKER:_fimo_impl_module_export_iterator"
        )
    endif ()


    # On Windows we embed the debug information and utilize the shared runtime library.
    set_property(TARGET ${FIMO_CURRENT_MODULE_TARGET} PROPERTY
            MSVC_RUNTIME_LIBRARY "MultiThreadedDLL"
    )
    set_property(TARGET ${FIMO_CURRENT_MODULE_TARGET} PROPERTY
            MSVC_DEBUG_INFORMATION_FORMAT "$<$<CONFIG:Debug,RelWithDebInfo>:Embedded>"
    )

    # Use the .dylib extension on macOS
    if (APPLE)
        set_property(TARGET ${FIMO_CURRENT_MODULE_TARGET} PROPERTY SUFFIX ".dylib")
    endif()

    # On the other Unixes we set the rpath to point to the module root.
    if (UNIX AND NOT APPLE)
        set_target_properties(${FIMO_CURRENT_MODULE_TARGET} PROPERTIES INSTALL_RPATH "$ORIGIN/")
    elseif (UNIX)
        set_target_properties(${FIMO_CURRENT_MODULE_TARGET} PROPERTIES INSTALL_RPATH "@loader_path/")
    endif ()
    set_target_properties(${FIMO_CURRENT_MODULE_TARGET} PROPERTIES BUILD_WITH_INSTALL_RPATH TRUE)

    # Move the built module to a known location.
    add_custom_command(
            TARGET ${FIMO_CURRENT_MODULE_TARGET} POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_BINARY_DIR}/_modules
            COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_CURRENT_BINARY_DIR}/$<TARGET_FILE_NAME:${FIMO_CURRENT_MODULE_TARGET}>
            ${FIMO_CURRENT_MODULE_BUILD_DIR}/module$<TARGET_FILE_SUFFIX:${FIMO_CURRENT_MODULE_TARGET}>
    )

    if (FIMO_INSTALL_MODULES)
        install(TARGETS ${FIMO_CURRENT_MODULE_TARGET} DESTINATION ${FIMO_CURRENT_MODULE_INSTALL_DIR})
        install(
                CODE
                "file(
                    RENAME
                    $<INSTALL_PREFIX>/${FIMO_CURRENT_MODULE_INSTALL_DIR}/$<TARGET_FILE_NAME:${FIMO_CURRENT_MODULE_TARGET}>
                    $<INSTALL_PREFIX>/${FIMO_CURRENT_MODULE_INSTALL_DIR}/module$<TARGET_FILE_SUFFIX:${FIMO_CURRENT_MODULE_TARGET}>
                )"
        )
    endif ()
endfunction()

function(fimo_add_bindings_test)
    if (NOT FIMO_TEST_BINDINGS)
        return()
    endif ()

    if (NOT TARGET fimo_all)
        add_custom_target(fimo_all)
    endif ()

    enable_testing()

    set(oneValueArgs NAME)
    set(multiValueArgs
            SOURCES
            COMPILE_OPTIONS
            LINK_LIBRARIES
            LINK_OPTIONS
    )
    cmake_parse_arguments(
            ARGS
            ""
            "${oneValueArgs}"
            "${multiValueArgs}"
            ${ARGN}
    )

    if (NOT DEFINED ARGS_NAME)
        message(FATAL_ERROR "No name provided for the test")
    endif ()
    if (NOT DEFINED ARGS_SOURCES)
        message(FATAL_ERROR "No sources provided for the test ${ARGS_NAME}")
    endif ()

    add_executable(fimo_bindings_test_${ARGS_NAME} ${ARGS_SOURCES})
    add_dependencies(fimo_bindings_test_${ARGS_NAME} fimo_all)
    target_link_libraries(fimo_bindings_test_${ARGS_NAME} PRIVATE Catch2::Catch2WithMain)
    target_compile_features(fimo_bindings_test_${ARGS_NAME} PRIVATE cxx_std_23)
    target_compile_definitions(fimo_bindings_test_${ARGS_NAME} PRIVATE FIMO_MODULES_DIR="${CMAKE_BINARY_DIR}/_modules")

    if (DEFINED ARGS_COMPILE_OPTIONS)
        target_compile_definitions(
                fimo_bindings_test_${ARGS_NAME}
                PRIVATE ${ARGS_COMPILE_OPTIONS}
        )
    endif ()

    if (DEFINED ARGS_LINK_LIBRARIES)
        target_link_libraries(
                fimo_bindings_test_${ARGS_NAME}
                PRIVATE ${ARGS_LINK_LIBRARIES}
        )
    endif ()

    if (DEFINED ARGS_LINK_OPTIONS)
        set_target_properties(
                fimo_bindings_test_${ARGS_NAME}
                PROPERTIES LINK_FLAGS
                ${ARGS_LINK_OPTIONS}
        )
    endif ()

    catch_discover_tests(fimo_bindings_test_${ARGS_NAME})
endfunction()

function(fimo_add_module_test)
    if (NOT FIMO_TEST_MODULES)
        return()
    endif ()

    if (NOT TARGET fimo_all)
        add_custom_target(fimo_all)
    endif ()

    enable_testing()

    set(oneValueArgs NAME)
    set(multiValueArgs
            SOURCES
            COMPILE_OPTIONS
            LINK_LIBRARIES
            LINK_OPTIONS
    )
    cmake_parse_arguments(
            ARGS
            ""
            "${oneValueArgs}"
            "${multiValueArgs}"
            ${ARGN}
    )

    if (NOT DEFINED ARGS_NAME)
        message(FATAL_ERROR "No name provided for the test")
    endif ()
    if (NOT DEFINED ARGS_SOURCES)
        message(FATAL_ERROR "No sources provided for the test ${ARGS_NAME}")
    endif ()

    add_executable(fimo_module_test_${ARGS_NAME} ${ARGS_SOURCES})
    add_dependencies(fimo_module_test_${ARGS_NAME} fimo_all)
    target_link_libraries(fimo_module_test_${ARGS_NAME} PRIVATE Catch2::Catch2WithMain)
    target_compile_features(fimo_module_test_${ARGS_NAME} PRIVATE cxx_std_23)
    target_compile_definitions(fimo_module_test_${ARGS_NAME} PRIVATE FIMO_MODULES_DIR="${CMAKE_BINARY_DIR}/_modules")

    if (DEFINED ARGS_COMPILE_OPTIONS)
        target_compile_definitions(
                fimo_module_test_${ARGS_NAME}
                PRIVATE ${ARGS_COMPILE_OPTIONS}
        )
    endif ()

    if (DEFINED ARGS_LINK_LIBRARIES)
        target_link_libraries(
                fimo_module_test_${ARGS_NAME}
                PRIVATE ${ARGS_LINK_LIBRARIES}
        )
    endif ()

    if (DEFINED ARGS_LINK_OPTIONS)
        set_target_properties(
                fimo_module_test_${ARGS_NAME}
                PROPERTIES LINK_FLAGS
                ${ARGS_LINK_OPTIONS}
        )
    endif ()

    catch_discover_tests(fimo_module_test_${ARGS_NAME})
endfunction()

function(fimo_include_enabled_bindings)
    foreach (NAME IN LISTS FIMO_BINDINGS)
        if (FIMO_ENABLE_ALL_BINDINGS OR FIMO_ENABLE_BINDINGS_${NAME})
            message(STATUS "Building bindings: ${NAME}")
            add_subdirectory(${NAME})
        endif ()
    endforeach ()
endfunction()

function(fimo_include_enabled_modules)
    foreach (NAME IN LISTS FIMO_MODULES)
        if (FIMO_ENABLE_ALL_MODULES OR FIMO_ENABLE_MODULE_${NAME})
            message(STATUS "Building module: ${NAME}")
            set(FIMO_CURRENT_MODULE ${NAME})
            set(FIMO_CURRENT_MODULE_TARGET fimo_module_${NAME})
            set(FIMO_CURRENT_MODULE_INSTALL_DIR "modules/${NAME}")
            set(FIMO_CURRENT_MODULE_RESOURCE_INSTALL_DIR "modules/${NAME}")
            set(FIMO_CURRENT_MODULE_BUILD_DIR ${CMAKE_BINARY_DIR}/_modules/${NAME})
            set(FIMO_CURRENT_MODULE_RESOURCE_BUILD_DIR ${CMAKE_BINARY_DIR}/_modules/${NAME})
            add_subdirectory(${NAME})
            unset(FIMO_CURRENT_MODULE_RESOURCE_BUILD_DIR)
            unset(FIMO_CURRENT_MODULE_BUILD_DIR)
            unset(FIMO_CURRENT_MODULE_RESOURCE_INSTALL_DIR)
            unset(FIMO_CURRENT_MODULE_INSTALL_DIR)
            unset(FIMO_CURRENT_MODULE_TARGET)
            unset(FIMO_CURRENT_MODULE)
        endif ()
    endforeach ()
endfunction()

macro(fimo_bindings_enabled NAME VAR)
    if (NOT ${NAME} IN_LIST FIMO_BINDINGS)
        message(FATAL_ERROR "No bindings with the name ${NAME} were declared")
    endif ()
    if (FIMO_ENABLE_ALL_BINDINGS OR FIMO_ENABLE_BINDINGS_${NAME})
        set(${VAR} TRUE)
    else ()
        set(${VAR} FALSE)
    endif ()
endmacro()

function(fimo_require_bindings NAME)
    fimo_bindings_enabled(${NAME} ENABLED)
    if (NOT ENABLED)
        message(FATAL_ERROR "Bindings for module ${NAME} are not enabled.")
    endif ()
endfunction()
