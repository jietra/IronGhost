cmake_minimum_required(VERSION 3.16)
project(agent_strategist LANGUAGES CXX)

# ============================
#   General
# ============================
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ============================
#   Paths
# ============================
# llama.cpp
set(LLAMA_DIR "${CMAKE_SOURCE_DIR}/../../llama.cpp")

# models
set(MODEL_DIR "${CMAKE_SOURCE_DIR}/../models")

# prompts
set(PROMPT_DIR "${CMAKE_SOURCE_DIR}/src/prompts")

# logs
set(LOG_DIR "${CMAKE_SOURCE_DIR}/logs")

# ============================
#   Apple Silicon Flags
# ============================
if(APPLE)
    message(STATUS "Building for Apple Silicon (Metal acceleration enabled)")
    add_definitions(-DLLAMA_METAL=1)
endif()

# ============================
#   Sources
# ============================
set(SOURCES
    src/main.cpp
    src/llm.cpp
    src/llm_handler.cpp
    src/uds_server.cpp
    src/utils.cpp
)

# ============================
#   Include dirs
# ============================
include_directories(
    ${LLAMA_DIR}
    ${LLAMA_DIR}/include
    ${CMAKE_SOURCE_DIR}/include
)

# ============================
#   Build
# ============================
add_subdirectory(${LLAMA_DIR} ${CMAKE_BINARY_DIR}/llama_build)

# ============================
#   Final binaries
# ============================
add_executable(agent_strategist ${SOURCES})

target_link_libraries(agent_strategist PRIVATE llama)

# ============================
#   Definitions for code
# ============================
add_definitions(
    -DMODEL_DIR="${MODEL_DIR}"
    -DPROMPT_DIR="${PROMPT_DIR}"
    -DLOG_DIR="${LOG_DIR}"
)

# ============================
#   Messages
# ============================
message(STATUS "Service ready.")
message(STATUS "Model directory: ${MODEL_DIR}")
message(STATUS "Prompt directory: ${PROMPT_DIR}")
message(STATUS "Log directory: ${LOG_DIR}")
