cmake_minimum_required(VERSION 3.15)
project(Hashmap)

add_library(hashmap STATIC hashmap.c)
set_property(TARGET hashmap PROPERTY LINKER_LANGUAGE C)
set_property(TARGET hashmap PROPERTY C_STANDARD 17)
set_property(TARGET hashmap PROPERTY C_STANDARD_REQUIRED ON)
set_property(TARGET hashmap PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")
target_include_directories(hashmap PUBLIC include)