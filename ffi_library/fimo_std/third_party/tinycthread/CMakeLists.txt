cmake_minimum_required(VERSION 3.15)
project(TinyCThread)

find_package(Threads REQUIRED)

add_library(tinycthread STATIC source/tinycthread.c)
target_link_libraries(tinycthread ${CMAKE_THREAD_LIBS_INIT})
target_include_directories(tinycthread PUBLIC include)
