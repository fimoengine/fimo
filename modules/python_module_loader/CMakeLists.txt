fimo_require_bindings(${FIMO_CURRENT_MODULE})
add_subdirectory(third_party)

fimo_add_module()
target_sources(${FIMO_CURRENT_MODULE_TARGET} PRIVATE src/main.c)
target_compile_features(${FIMO_CURRENT_MODULE_TARGET} PRIVATE c_std_17)
target_link_libraries(${FIMO_CURRENT_MODULE_TARGET} PRIVATE cpython fimo_python_module_loader)

add_subdirectory(tests)