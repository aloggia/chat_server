cmake_minimum_required(VERSION 3.21)
project(chat_server)

set(CMAKE_CXX_STANDARD 14)

include_directories(.)

add_executable(chat_server
        main.cpp
        main.h)
