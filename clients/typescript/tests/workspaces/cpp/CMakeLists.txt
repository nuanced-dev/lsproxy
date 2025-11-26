cmake_minimum_required(VERSION 3.16)
project(toy_cpp_ws LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
add_executable(toy_cpp_ws
  src/main.cpp
  src/fizzbuzz.cpp
  src/util.cpp
)
target_include_directories(toy_cpp_ws PRIVATE src)
