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

if(MSVC)
    set(HY_COMPILE_OPTIONS /W4 /WX)
else()
    set(HY_COMPILE_OPTIONS -Wall -Wextra -Wpedantic -Werror)
endif()

# Option configuring the build
option(HY_BUILD_SHARED "Build Hyperion as a shared library" ON)
option(HY_DEBUG_ASSERTS "Enable debug assertions" OFF)

# Write configuration using version info
message("Generating generated_version.h")
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/version.h.in
    ${CMAKE_CURRENT_SOURCE_DIR}/hycore/include/hycore/generated_version.h
)

# Add subdirectories
add_subdirectory(hycore)
