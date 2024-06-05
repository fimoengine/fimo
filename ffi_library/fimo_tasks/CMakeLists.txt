cmake_minimum_required(VERSION 3.25 FATAL_ERROR)
project(
        tasks
        VERSION 0.1.0
        LANGUAGES C
)
enable_language(C)

set(TASKS_HEADERS
        include/fimo_tasks/tasks.h
)

add_library(fimo_tasks INTERFACE ${TASKS_HEADERS})
target_sources(fimo_tasks INTERFACE
        FILE_SET public_headers
        TYPE HEADERS
        FILES ${TASKS_HEADERS}
)
target_link_libraries(fimo_tasks INTERFACE fimo_std)
target_include_directories(fimo_tasks INTERFACE include)
set_target_properties(fimo_tasks PROPERTIES LINKER_LANGUAGE C)

if (FIMO_INSTALL_BINDINGS)
    include(GNUInstallDirs)
    install(TARGETS fimo_tasks
            ARCHIVE
            FILE_SET public_headers
            DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/fimo_tasks
    )
endif ()