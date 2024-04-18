fimo_add_module_test(
        NAME python_module_loader_run_string
        SOURCES run_string.cpp
        LINK_LIBRARIES fimo_std fimo_python_module_loader
)