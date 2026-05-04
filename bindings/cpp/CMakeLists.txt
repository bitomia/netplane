cmake_minimum_required(VERSION 3.16)
project(netplane_cpp_bindings CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(WORKSPACE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../..)

if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(CARGO_PROFILE_FLAG --release)
    set(CARGO_TARGET_SUBDIR release)
else()
    set(CARGO_PROFILE_FLAG)
    set(CARGO_TARGET_SUBDIR debug)
endif()

set(NETPLANE_LIB ${WORKSPACE_DIR}/target/${CARGO_TARGET_SUBDIR}/${CMAKE_STATIC_LIBRARY_PREFIX}netplane_client${CMAKE_STATIC_LIBRARY_SUFFIX})

add_custom_command(
    OUTPUT ${NETPLANE_LIB}
    COMMAND cargo build -p netplane_client ${CARGO_PROFILE_FLAG}
    WORKING_DIRECTORY ${WORKSPACE_DIR}
    COMMENT "Building netplane_client (cargo)"
    VERBATIM
)
add_custom_target(netplane_client_lib DEPENDS ${NETPLANE_LIB})

add_library(netplane_client STATIC IMPORTED GLOBAL)
set_target_properties(netplane_client PROPERTIES
    IMPORTED_LOCATION ${NETPLANE_LIB}
    INTERFACE_INCLUDE_DIRECTORIES ${CMAKE_CURRENT_SOURCE_DIR}
)
add_dependencies(netplane_client netplane_client_lib)

if(APPLE)
    target_link_libraries(netplane_client INTERFACE
        "-framework CoreFoundation"
        "-framework Security"
        "-framework SystemConfiguration"
    )
endif()

find_package(Threads REQUIRED)
target_link_libraries(netplane_client INTERFACE Threads::Threads ${CMAKE_DL_LIBS} m)

enable_testing()

add_executable(test_netplane test_netplane.cpp)
target_link_libraries(test_netplane PRIVATE netplane_client)

add_test(NAME test_netplane COMMAND test_netplane)
