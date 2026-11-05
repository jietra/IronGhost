#pragma once

#include "llm.hpp"

#include <cstdint>
#include <filesystem>
#include <string>

struct RuntimeParams {
    int32_t threads      = 8;
    int32_t gpu_layers   = 0;
    int32_t context_size = 32768;
};

struct BehaviorParams {
    bool amnesia      = true;
    bool enable_debug = false;
    bool print_prompt = false;
};

struct ServiceConfig {
    std::string id;

    std::filesystem::path model_path;
    std::filesystem::path system_prompt_path;
    std::filesystem::path grammar_path;

    std::string socket_path;

    RuntimeParams  runtime;
    SamplingParams sampling;
    BehaviorParams behavior;
};

ServiceConfig load_service_config(
    const std::filesystem::path& config_path,
    const std::string& service_id
);