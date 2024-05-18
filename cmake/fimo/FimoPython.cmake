find_package(Python COMPONENTS Interpreter REQUIRED)

macro(fimo_declare_python_bindings)
    cmake_parse_arguments(
            ARGS
            ""
            "NAME;ENABLED"
            ""
            ${ARGN}
    )

    if (NOT DEFINED ARGS_NAME)
        message(FATAL_ERROR "No name provided for the bindings")
    endif ()
    if (NOT DEFINED ARGS_ENABLED)
        message(FATAL_ERROR "Not provided if the bindings ${ARGS_NAME} should be built")
    endif ()

    if (NOT DEFINED FIMO_PYTHON_BINDINGS)
        set(FIMO_PYTHON_BINDINGS "")
    endif ()
    if (${ARGS_NAME} IN_LIST FIMO_PYTHON_BINDINGS)
        message(FATAL_ERROR "Python bindings ${NAME} were already declared")
    endif ()
    list(APPEND FIMO_PYTHON_BINDINGS ${ARGS_NAME})
    cmake_dependent_option(FIMO_PYTHON_ENABLE_BINDINGS_${ARGS_NAME}
            "Enables the ${ARGS_NAME} bindings" ${ARGS_ENABLED}
            "NOT FIMO_DISABLE_BINDINGS" OFF
    )
endmacro()

function(fimo_add_python_bindings NAME)
    if (NOT TARGET fimo_all)
        add_custom_target(fimo_all)
    endif ()

    # Build the python package
    add_custom_target(fimo_python_bindings_${NAME}
            COMMAND ${Python_EXECUTABLE} ${CMAKE_SOURCE_DIR}/cmake/fimo/PythonBuild.py build
            --src-dir ${CMAKE_CURRENT_SOURCE_DIR}
            --out-dir ${CMAKE_BINARY_DIR}/python_target/bindings
    )

    # Add the tests
    if (FIMO_TEST_BINDINGS)
        enable_testing()
        add_custom_target(fimo_python_bindings_test_${NAME}
                COMMAND ${CMAKE_COMMAND} -E env MODULES_DIR=${CMAKE_BINARY_DIR}/_modules
                ${Python_EXECUTABLE} ${CMAKE_SOURCE_DIR}/cmake/fimo/PythonBuild.py test
                --dist-dir ${CMAKE_BINARY_DIR}/python_target/bindings
                --package ${NAME}
        )
        add_dependencies(fimo_python_bindings_test_${NAME} fimo_all)
        add_dependencies(fimo_python_bindings_test_${NAME} fimo_python_bindings_${NAME})
        add_test(NAME fimo_python_bindings_test_${NAME}
                COMMAND ${CMAKE_COMMAND}
                --build ${CMAKE_BINARY_DIR}
                --config $<CONFIG>
                --target fimo_python_bindings_test_${NAME}
        )
    endif ()
endfunction()

function(fimo_python_include_enabled_bindings)
    foreach (NAME IN LISTS FIMO_PYTHON_BINDINGS)
        if (FIMO_PYTHON_ENABLE_ALL_BINDINGS OR FIMO_PYHTON_ENABLE_BINDINGS_${NAME})
            message(STATUS "Building bindings: ${NAME}")
            add_subdirectory(${NAME})
        endif ()
    endforeach ()
endfunction()

macro(fimo_python_bindings_enabled NAME VAR)
    if (NOT ${NAME} IN_LIST FIMO_PYTHON_BINDINGS)
        message(FATAL_ERROR "No Python bindings with the name ${NAME} were declared")
    endif ()
    if (FIMO_PYTHON_ENABLE_ALL_BINDINGS OR FIMO_PYTHON_ENABLE_BINDINGS_${NAME})
        set(${VAR} TRUE)
    else ()
        set(${VAR} FALSE)
    endif ()
endmacro()

function(fimo_require_python_bindings NAME)
    fimo_python_bindings_enabled(${NAME} ENABLED)
    if (NOT ENABLED)
        message(FATAL_ERROR "Python bindings for module ${NAME} are not enabled.")
    endif ()
endfunction()