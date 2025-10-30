# Project configuration for CMake
cmake_minimum_required(VERSION 4.0.0)

# Setup project version and language
project(
    hyperion
    LANGUAGES CXX
)

include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/GitVersion.cmake)

get_version_from_git()

# Configure C++ standard and other options
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Option configuring the build
option(HY_BUILD_SHARED "Build Hyperion as a shared library" ON)

