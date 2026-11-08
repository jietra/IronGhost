cmake_minimum_required(VERSION 3.16)
project(ironghost_llm_service LANGUAGES CXX)

# ============================
#   General
# ============================
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ============================
#   llama.cpp
# ============================
set(LLAMA_DIR "${CMAKE_SOURCE_DIR}/../../llama.cpp")

add_subdirectory(
    ${LLAMA_DIR}
    ${CMAKE_BINARY_DIR}/llama_build
)

# ============================
#   yaml-cpp
# ============================
include(FetchContent)

FetchContent_Declare(
    yaml-cpp
    GIT_REPOSITORY https://github.com/jbeder/yaml-cpp.git
    GIT_TAG yaml-cpp-0.9.0
)

set(YAML_CPP_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(YAML_CPP_BUILD_TOOLS OFF CACHE BOOL "" FORCE)

FetchContent_MakeAvailable(yaml-cpp)

# ============================
#   IronGhost service
# ============================
add_executable(
    ironghost_llm_service

    src/main.cpp
    src/config.cpp
    src/llm.cpp
    src/llm_handler.cpp
    src/uds_server.cpp
    src/utils.cpp
    src/uds_protocol.cpp
)

target_include_directories(
    ironghost_llm_service
    PRIVATE
    ${LLAMA_DIR}
    ${LLAMA_DIR}/include
    ${CMAKE_SOURCE_DIR}/include
)

target_link_libraries(
    ironghost_llm_service
    PRIVATE
    llama
    yaml-cpp::yaml-cpp
)

# ============================
#   Apple Silicon Flags
# ============================
if(APPLE)
    message(
        STATUS
        "Building on Apple platform"
    )
    add_definitions(-DLLAMA_METAL=1)
endif()

# ============================
#   Messages
# ============================
message(STATUS "IronGhost LLM service ready.")
