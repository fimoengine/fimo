macro(fimo_declare_rust_bindings)
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

    if (NOT DEFINED FIMO_RUST_BINDINGS)
        set(FIMO_RUST_BINDINGS "")
    endif ()
    if (${ARGS_NAME} IN_LIST FIMO_RUST_BINDINGS)
        message(FATAL_ERROR "Rust bindings ${NAME} were already declared")
    endif ()
    list(APPEND FIMO_RUST_BINDINGS ${ARGS_NAME})
    cmake_dependent_option(FIMO_RUST_ENABLE_BINDINGS_${ARGS_NAME}
            "Enables the ${ARGS_NAME} bindings" ${ARGS_ENABLED}
            "NOT FIMO_DISABLE_BINDINGS" OFF
    )
endmacro()

function(fimo_add_rust_bindings NAME)
    if (NOT TARGET fimo_all)
        add_custom_target(fimo_all)
    endif ()

    # Build the rust crate
    add_custom_target(fimo_rust_bindings_${NAME}
            COMMAND cargo build
            --manifest-path ${CMAKE_SOURCE_DIR}/Cargo.toml
            --target-dir ${CMAKE_BINARY_DIR}/target
            --package ${NAME}
            $<$<NOT:$<CONFIG:Debug>>:--release>
    )

    # Add the tests
    if (FIMO_TEST_BINDINGS)
        enable_testing()
        add_custom_target(fimo_rust_bindings_test_${NAME}
                COMMAND ${CMAKE_COMMAND} -E env MODULES_DIR=${CMAKE_BINARY_DIR}/_modules
                cargo test
                --manifest-path ${CMAKE_SOURCE_DIR}/Cargo.toml
                --target-dir ${CMAKE_BINARY_DIR}/target
                --package ${NAME}
                $<$<NOT:$<CONFIG:Debug>>:--release>
        )
        add_dependencies(fimo_rust_bindings_test_${NAME} fimo_all)
        add_dependencies(fimo_rust_bindings_test_${NAME} fimo_rust_bindings_${NAME})
        add_test(NAME fimo_rust_bindings_test_${NAME}
                COMMAND ${CMAKE_COMMAND}
                --build ${CMAKE_BINARY_DIR}
                --config $<CONFIG>
                --target fimo_rust_bindings_test_${NAME}
        )
    endif ()
endfunction()

function(fimo_add_rust_module)
    if (NOT DEFINED FIMO_CURRENT_MODULE)
        message(FATAL_ERROR "Not currently building a module")
    endif ()

    if (NOT TARGET fimo_all)
        add_custom_target(fimo_all)
    endif ()

    # We always require the fimo_std bindings
    fimo_require_rust_bindings(fimo_std)

    # Build the rust crate
    add_custom_target(${FIMO_CURRENT_MODULE_TARGET}
            COMMAND cargo build -Z unstable-options
            --manifest-path ${CMAKE_SOURCE_DIR}/Cargo.toml
            --target-dir ${CMAKE_BINARY_DIR}/target
            --out-dir ${FIMO_CURRENT_MODULE_INSTALL_DIR}
            --package ${FIMO_CURRENT_MODULE}
            $<$<NOT:$<CONFIG:Debug>>:--release>
    )
    add_dependencies(fimo_all ${FIMO_CURRENT_MODULE_TARGET})

    # Add the tests
    if (FIMO_TEST_MODULES)
        enable_testing()
        add_custom_target(fimo_module_test_${FIMO_CURRENT_MODULE}
                COMMAND ${CMAKE_COMMAND} -E env MODULES_DIR=${CMAKE_BINARY_DIR}/_modules
                cargo test
                --manifest-path ${CMAKE_SOURCE_DIR}/Cargo.toml
                --target-dir ${CMAKE_BINARY_DIR}/target
                --package ${FIMO_CURRENT_MODULE}
                $<$<NOT:$<CONFIG:Debug>>:--release>
        )
        add_dependencies(fimo_module_test_${FIMO_CURRENT_MODULE} fimo_all)
        add_test(NAME fimo_module_test_${FIMO_CURRENT_MODULE}
                COMMAND ${CMAKE_COMMAND}
                --build ${CMAKE_BINARY_DIR}
                --config $<CONFIG>
                --target fimo_module_test_${FIMO_CURRENT_MODULE}
        )
    endif ()
endfunction()

function(fimo_rust_include_enabled_bindings)
    foreach (NAME IN LISTS FIMO_RUST_BINDINGS)
        if (FIMO_RUST_ENABLE_ALL_BINDINGS OR FIMO_RUST_ENABLE_BINDINGS_${NAME})
            message(STATUS "Building bindings: ${NAME}")
            add_subdirectory(${NAME})
        endif ()
    endforeach ()
endfunction()

macro(fimo_rust_bindings_enabled NAME VAR)
    if (NOT ${NAME} IN_LIST FIMO_RUST_BINDINGS)
        message(FATAL_ERROR "No Rust bindings with the name ${NAME} were declared")
    endif ()
    if (FIMO_RUST_ENABLE_ALL_BINDINGS OR FIMO_RUST_ENABLE_BINDINGS_${NAME})
        set(${VAR} TRUE)
    else ()
        set(${VAR} FALSE)
    endif ()
endmacro()

function(fimo_require_rust_bindings NAME)
    fimo_rust_bindings_enabled(${NAME} ENABLED)
    if (NOT ENABLED)
        message(FATAL_ERROR "Rust bindings for module ${NAME} are not enabled.")
    endif ()
endfunction()