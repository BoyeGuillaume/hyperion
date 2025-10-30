# Find sources under ${CMAKE_CURRENT_SOURCE_DIR}/src
# and create library target hycore
file(GLOB HYCORE_SOURCES CONFIGURE_DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/src/**/*.cpp"
)
file(GLOB HYCORE_HEADERS CONFIGURE_DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/include/**/*.h"
    "${CMAKE_CURRENT_SOURCE_DIR}/include/**/*.hpp"
)
message(STATUS "HYCORE_SOURCES: ${HYCORE_SOURCES}")
message(STATUS "HYCORE_HEADERS: ${HYCORE_HEADERS}")

# Create the library (shared or static based on HY_BUILD_SHARED option)
if(HY_BUILD_SHARED)
    add_library(hycore SHARED)
    message(STATUS "Building hycore as a shared library")
else()
    add_library(hycore STATIC)
    message(STATUS "Building hycore as a static library")
endif()
add_library(hy::core ALIAS hycore)

target_sources(hycore PUBLIC
    ${HYCORE_SOURCES}
)

# Additional information about the binary
set_target_properties(hycore
    PROPERTIES
    PUBLIC_HEADER "${HYCORE_HEADERS}"
    VERSION ${HY_SIMPLE_VERSION}
    SOVERSION ${HY_SIMPLE_VERSION}
)

target_compile_options(hycore PRIVATE ${HY_COMPILE_OPTIONS})

target_compile_definitions(hycore
    PRIVATE
    $<IF:$<CONFIG:Debug>,HY_DEBUG_ASSERTS,${HY_DEBUG_ASSERTS}>
    HYCORE_EXPORTS
)

target_include_directories(hycore
    PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)
