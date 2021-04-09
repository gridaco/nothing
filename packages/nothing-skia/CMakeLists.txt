cmake_minimum_required(VERSION 3.17)
project(nothing_skia)

set(CMAKE_CXX_STANDARD 14)

add_library(nothing_skia SHARED library.cpp library.h)