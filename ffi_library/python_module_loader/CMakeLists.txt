cmake_minimum_required(VERSION 3.25 FATAL_ERROR)
project(
        fimo_python_module_loader
        VERSION 0.1.0
        LANGUAGES C
)
enable_language(C)

add_library(fimo_python_module_loader INTERFACE)
target_sources(fimo_python_module_loader INTERFACE
        FILE_SET public_headers
        TYPE HEADERS
        FILES include/fimo_python_module_loader/loader.h
)
target_link_libraries(fimo_python_module_loader INTERFACE fimo_std)
target_include_directories(fimo_python_module_loader INTERFACE include)

if (FIMO_INSTALL_BINDINGS)
    include(GNUInstallDirs)
    install(TARGETS fimo_python_module_loader
            ARCHIVE
            FILE_SET public_headers
            DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/fimo_python_module_loader
    )
endif ()