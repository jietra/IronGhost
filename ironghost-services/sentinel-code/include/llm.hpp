#pragma once

#include "utils.hpp"

#include <string>
#include <functional>
#include <llama.h>
#include <vector>
#include <deque>

/// @brief Struct for model sampling parameters
/// Parameters are set with default values, so that providing user values is optional.
struct SamplingParams {
    float       temperature = 0.8f;
    float       top_p       = 0.95f;
    int32_t     top_k       = 40;
    float       min_p       = 0.05f;
    uint32_t    seed        = LLAMA_DEFAULT_SEED;

    int32_t penalty_last_n  = 64;
    float   penalty_repeat  = 1.1f;
    float   penalty_freq    = 0.0f;
    float   penalty_present = 0.0f;

    std::string grammar_str = "";

    /// @brief Build SamplingParams from env map
    static SamplingParams from_env(const std::unordered_map<std::string, std::string>& env) {
        SamplingParams params;

        auto get_float = [&](const std::string& key, float default_val) {
            auto it = env.find(key);
            return (it != env.end() && !it->second.empty()) ? std::stof(it->second) : default_val;
        };

        auto get_int = [&](const std::string& key, int32_t default_val) {
            auto it = env.find(key);
            return (it != env.end() && !it->second.empty()) ? std::stoi(it->second) : default_val;
        };

        // int/float params
        params.temperature     = get_float("TEMPERATURE", params.temperature);
        params.top_p           = get_float("TOP_P", params.top_p);
        params.top_k           = get_int("TOP_K", params.top_k);
        params.min_p           = get_float("MIN_P", params.min_p);
        params.penalty_last_n  = get_int("PENALTY_LAST_N", params.penalty_last_n);
        params.penalty_repeat  = get_float("PENALTY_REPEAT", params.penalty_repeat);
        params.penalty_freq    = get_float("PENALTY_FREQ", params.penalty_freq);
        params.penalty_present = get_float("PENALTY_PRESENT", params.penalty_present);

        // grammar
        auto it_gbnf = env.find("GRAMMAR_STR");
        if (it_gbnf != env.end() && !it_gbnf->second.empty()) {
            params.grammar_str = read_file(it_gbnf->second);
        }

        return params;
    }
};

/// @brief LLM model Class based on llama.cpp.
/// Handle model loading, context, sampler and generation.
class LLM {
public:
    /// @brief LLM Constructor.
    /// @param model_path 
    /// @param system_prompt 
    /// @param n_threads        Context parameter n_threads.
    /// @param n_gpu_layers     Model parameter n_gpu_layers.
    /// @param params           Sampling parameters of type SamplingParams.
    LLM(
        const std::string&  model_path,
        const std::string&  system_prompt,
        int32_t             n_threads,
        int32_t             n_gpu_layers,
        int32_t             ctx_size_requested,
        SamplingParams      sampling_params = SamplingParams()
    );

    /// @brief Public attribute for streaming callback function
    std::function<void(const std::string&)> stream_callback = nullptr;
    
    /// @brief Method setting streaming callback
    /// @param cb 
    void set_stream_callback(std::function<void(const std::string&)> cb) {
        stream_callback = cb;
    }

    /// @brief Generate response based on full prompt.
    /// @param full_prompt Full formatted prompt.
    /// @return llm response
    std::string run_llm(const std::string& full_prompt);

    /// @brief Generate response based on raw user prompt.
    /// Apply chat template, call run_llm on the full prompt and update conversation history.
    /// @param user_prompt 
    /// @return llm response
    std::string generate(const std::string& user_prompt);

    /// @brief Reset sampler with optional parameters.
    /// Allows resetting model parameters while keeping conversation history.
    /// @param params Sampling parameters (optional).
    void reset_sampler(const SamplingParams & params = SamplingParams());

    /// @brief Reset conversation history and KV cache.
    void reset_messages();

    /// @brief Reset sampler and conversation history.
    /// Usefull for stateless behavior.
    void reset_all();

    /// @brief Reset KV cache only.
    void reset_kv_cache();

    /// @brief Reset context.
    void reset_context();

private:
    std::string         system_prompt;
    int32_t             n_threads;          // context parameter n_threads
    int32_t             ctx_size_requested; // context parameter n_ctx
    SamplingParams      sampling_params;

    llama_model*        model   = nullptr;
    const llama_vocab*  vocab   = nullptr;

    llama_sampler*      sampler = nullptr;  // next token selection rule
    
    llama_context*      ctx     = nullptr;  // for déjà-vue (embeddings, KV cache...)
    int32_t             n_ctx;              // actual context size (can differ from ctx size requested)
    
    std::string         chat_template;
    
    // messages management: use a double-ended queue to allocate stable memory blocks
    std::deque<std::string>         text_storage;   // physical memory (stable addresses)
    std::vector<llama_chat_message> messages;       // light C container for conversation history
    
    // KV-cache management
    size_t              kv_len       = 0;   // nb of chars  encoded in KV-cache
    int32_t             kv_len_token = 0;   // nb of tokens encoded in KV-cache

    /// @brief Add a message to the conversation history
    /// @param role 
    /// @param content 
    void add_message(const std::string& role, const std::string& content);

    /// @brief Add user prompt to conversation history and return formatted prompt
    /// @param user_prompt 
    /// @return Formatted prompt
    std::string add_and_format_prompt(const std::string& user_prompt);

    /// @brief Add llm response to conversation history
    /// @param response
    void add_response(const std::string& response);

};