fimo_add_bindings_test(
        NAME std_context
        SOURCES context.cpp
        LINK_LIBRARIES fimo_std
)

fimo_add_bindings_test(
        NAME std_graph
        SOURCES graph.cpp
        LINK_LIBRARIES fimo_std
)

fimo_add_bindings_test(
        NAME std_module_subsystem
        SOURCES module_subsystem.cpp
        LINK_LIBRARIES fimo_std
)

fimo_add_bindings_test(
        NAME std_path
        SOURCES path.cpp
        LINK_LIBRARIES fimo_std
)