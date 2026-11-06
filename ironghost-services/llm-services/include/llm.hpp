#pragma once

#include <string>
#include <functional>
#include <llama.h>
#include <vector>
#include <deque>
#include <atomic>

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

    int32_t max_tokens      = 2048;

    std::string grammar_str = "";
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
        bool                amnesia         = true,
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

    void request_cancel() noexcept {
        cancel_requested.store(
            true,
            std::memory_order_relaxed
        );
    }

    void clear_cancel() noexcept {
        cancel_requested.store(
            false,
            std::memory_order_relaxed
        );
    }

private:
    std::string         system_prompt;
    int32_t             n_threads;          // context parameter n_threads
    int32_t             ctx_size_requested; // context parameter n_ctx
    bool                amnesia;

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

    std::atomic<bool> cancel_requested{false};

};