cmake_minimum_required(VERSION 3.10)

project(ProtGraphCpp VERSION 1.0)

if(DEFINED SET_DEBUG)
    set(CMAKE_CXX_FLAGS_DEBUG  "-g -pthread")
    set(CMAKE_BUILD_TYPE Debug)
else()
    set(CMAKE_BUILD_TYPE Release)
    set(CMAKE_CXX_FLAGS_RELEASE "-DNDEBUG -O3 -funroll-loops -march=native -msse2 -pthread") # -Wall  for all warnings/errors!
    message(${CMAKE_CXX_FLAGS_RELEASE})
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED True)

add_executable(protgraphtraverseintvarlimitter main.cpp)
target_sources(protgraphtraverseintvarlimitter PRIVATE
    graph_loader_binary.cpp
    graph_loader.hpp
    protein_graph.hpp
    protein_graph.cpp
)
