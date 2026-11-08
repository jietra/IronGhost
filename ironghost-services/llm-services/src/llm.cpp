#include "llm.hpp"
#include <iostream>
#include <string>
#include <vector>
#include <cstring>
#include <stdexcept>

LLM::LLM(
    const std::string&  model_path,
    const std::string&  system_prompt,
    int32_t             n_threads,
    int32_t             n_gpu_layers,
    int32_t             ctx_size_requested,
    bool                amnesia,
    SamplingParams      sampling_params
)
    : system_prompt(        system_prompt       ),
      n_threads(            n_threads           ),
      ctx_size_requested(   ctx_size_requested  ),
      amnesia(              amnesia             ),
      sampling_params(      sampling_params     )
{
    // load dynamic backends
    ggml_backend_load_all();

    // LLM.model (llama_model_default_params + llama_model_load_from_file)
    llama_model_params mparams  = llama_model_default_params();
    mparams.n_gpu_layers        = n_gpu_layers;

    model = llama_model_load_from_file(model_path.c_str(), mparams);
    if (!model) {
        std::cerr << "[LLM] ERROR: cannot load model: " << model_path << std::endl;
        std::exit(1);
    }

    // LLM.vocab (llama_model_get_vocab)
    vocab = llama_model_get_vocab(model);

    // LLM.chat_template init
    chat_template = llama_model_chat_template(model, nullptr);
    if (chat_template.empty()) {
        std::cerr << "[LLM] Warning: model has no chat template" << std::endl;
    }

    // LLM.ctx (llama_context_default_params + llama_init_from_model)
    llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx       = ctx_size_requested;   // value requested by user (or default value)
    cparams.n_threads   = n_threads;
    cparams.n_batch     = ctx_size_requested;   // when model uses flash attention, it has strict rules checking n_batch.

    ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        std::cerr << "[LLM] ERROR: cannot create llama context" << std::endl;
        std::exit(1);
    }

    // LLM.n_ctx
    n_ctx = llama_n_ctx(ctx);

    // LLM.sampler init
    reset_sampler(sampling_params);

    // LLM.messages and LLM.text_storage init
    add_message("system", system_prompt);
}

void LLM::add_message(const std::string& role, const std::string& content) {
    text_storage.push_back(role);
    text_storage.push_back(content);

    size_t sz = text_storage.size();
    messages.push_back({ text_storage[sz - 2].c_str(), text_storage[sz - 1].c_str() });
}

std::string LLM::add_and_format_prompt(const std::string& user_prompt) {

    // add user_prompt to the conversation history
    add_message("user", user_prompt);

    // prepare formatted prompt
    std::vector<char>   formatted(llama_n_ctx(ctx));

    int32_t len = llama_chat_apply_template(
        chat_template.c_str(),
        messages.data(),
        messages.size(),
        true,               // add_assistant
        formatted.data(),
        formatted.size()
    );
    if (len > (int32_t)formatted.size()) {
        formatted.resize(len);
        len = llama_chat_apply_template(chat_template.c_str(), messages.data(), messages.size(), true, formatted.data(), formatted.size());
    }
    if (len < 0) {
        std::cerr << "[LLM] ERROR: failed to apply the chat template\n";
        std::exit(1);
    }
    size_t new_len = static_cast<size_t>(len);

    // full formatted last prompt: keep only new prompt for tokenization (the rest is already in KV cache)
    std::string full_prompt(formatted.data() + kv_len, new_len - kv_len);

    // update kv_len according to new KV-cache state
    kv_len = new_len;

    // debug: check
    std::cout   << "\n=== FULL PROMPT PASSED TO TOKENIZER ===\n" 
                << full_prompt 
                << "\n========================================\n";

    return full_prompt;
}

void LLM::add_response(const std::string& response) {
    // add response to messages
    add_message("assistant", response);
    
    // format response
    const int32_t len = llama_chat_apply_template(
        chat_template.c_str(),
        messages.data(),
        messages.size(),
        false,
        nullptr,
        0
    );
    if (len < 0) {
        std::cerr << "[LLM] ERROR: failed to apply the chat template\n";
        return;
    }

    kv_len = static_cast<size_t>(len);
}

GenerationResult LLM::generate(const std::string& user_prompt) {

    // add user_prompt to the conversation history and return formatted prompt
    std::string full_prompt = add_and_format_prompt(user_prompt);

    // run LLM on formatted (last) prompt
    GenerationResult response = run_llm(full_prompt);

    // add response to messages if statefull
    if (amnesia) {
        reset_all();
        return response;
    }
    
    if (
        response.reason == GenerationStopReason::EndOfGeneration
    ) {
        add_response(response.output);
    } else {
        // clear desynchronised KV-cache if generation failed
        reset_context();
    }

    return response;
}

GenerationResult LLM::run_llm(const std::string& full_prompt) {
    // Tokenize prompt, batch, loop with decode

    // interruption management
    clear_cancel();

    // replace grammar at root (when !amnesia)
    llama_sampler_reset(sampler);

    // prepare result
    GenerationResult result  = GenerationResult {
        "",
        GenerationStopReason::Init,
        0
    };

    // KV cache init test: test if first prompt
    const bool is_first = kv_len_token == 0;

    // --- Tokenize formatted last prompt ---
    const int32_t token_count = llama_tokenize(
        vocab,
        full_prompt.c_str(),
        full_prompt.size(),
        nullptr,
        0,
        is_first,
        true
    );

    if (token_count >= 0) {
        result.reason = GenerationStopReason::TokenizerError;
        std::cerr << "[LLM] ERROR: Unexpected tokenizer sizing result" << std::endl;
        return result;
    }

    const int32_t n_prompt_tokens = -token_count;

    std::vector<llama_token> tokens(n_prompt_tokens);

    if (llama_tokenize(
        vocab,
        full_prompt.c_str(),
        full_prompt.size(),
        tokens.data(),
        static_cast<int32_t>(tokens.size()),
        is_first,
        true
    ) < 0) {
        result.reason = GenerationStopReason::TokenizerError;
        std::cerr << "[LLM] ERROR: failed to tokenize prompt" << std::endl;
        return result;
    }

    // --- Prepare batch ---
    llama_batch batch = llama_batch_get_one(
        tokens.data(),
        static_cast<int32_t>(tokens.size())
    );

    // --- Entering loop ---
    llama_token new_token_id = LLAMA_TOKEN_NULL;    // must outlive every batch that points to it
    
    while (result.generated_tokens < sampling_params.max_tokens) {

        if (cancel_requested.load(std::memory_order_relaxed)) {
            result.reason = GenerationStopReason::Cancelled;
            std::cerr << "[LLM] Generation cancelled\n";
            break;
        }
        
        // check available context space
        if (kv_len_token + batch.n_tokens > n_ctx) {
            result.reason = GenerationStopReason::ContextFull;
            std::cerr << "[LLM] WARNING: context size exceeded. Consider cropping messages." << std::endl;
            break;
        }
        // TODO: create a method to crop messages: crop_messages(n_messages), or automatic cropping, or auto summurize.

        // --- Eval prompt ---
        const int32_t ret = llama_decode(ctx, batch);
        if (ret != 0) {
                result.reason = GenerationStopReason::DecodeError;
                std::cerr
                    << "[LLM] ERROR: llama_decode failed with ret = "
                    << std::to_string(ret)
                    << std::endl;
                break;
        }

        // update kv_len_token according to new KV-cache state
        kv_len_token += batch.n_tokens;

        // --- Sample next token ---
        new_token_id = llama_sampler_sample(sampler, ctx, -1);

        // --- EOG? ---
        if (llama_vocab_is_eog(vocab, new_token_id)) {
            result.reason = GenerationStopReason::EndOfGeneration;
            break;
        }

        // update generated_tokens
        ++result.generated_tokens;

        // --- Convert token to string ---
        std::string piece;

        try {
            piece = token_to_piece(
                new_token_id
            );
        } catch (const std::exception& error) {
            result.reason = GenerationStopReason::TokenToPieceError;
            std::cerr
                << "[LLM] ERROR: "
                << error.what()
                << std::endl;
            break;
        }

        // update output
        result.output += piece;

        // stream to console (server side) (for debug)
        std::cout << piece << std::flush;

        // streaming to orchestrator
        if (stream_callback) {
            stream_callback(piece);
        }

        // the callback may have detected a broken socket and requested cancellation.
        if (cancel_requested.load(
            std::memory_order_relaxed
        )) {
            result.reason = GenerationStopReason::Cancelled;
            break;
        }

        // prepare next batch with sampled token
        batch = llama_batch_get_one(&new_token_id, 1);
    }

    if (
        result.reason == GenerationStopReason::Init &&
        result.generated_tokens >= sampling_params.max_tokens
    ) {
        result.reason = GenerationStopReason::MaxTokens;
        std::cerr
            << "[LLM] Maximum generated-token limit reached: "
            << sampling_params.max_tokens
            << std::endl;
    }

    // defensive fallback
    if (result.reason == GenerationStopReason::Init) {
        std::cerr
            << "[LLM] WARNING: generation stopped for an unknown reason"
            << std::endl;
        result.reason = GenerationStopReason::Unknown;
    }

    return result;
}

void LLM::reset_sampler(const SamplingParams & params) {
    sampling_params = params;
    
    // deallocate sampler memory if any (avoid leaky memory)
    if (sampler) {
        llama_sampler_free(sampler);
        sampler = nullptr;
    }

    sampler = llama_sampler_chain_init(llama_sampler_chain_default_params());

    //      0. init grammar
    if (!sampling_params.grammar_str.empty()) {
        std::cout << "[SAMPLER] applying grammar..." << std::endl;
        struct llama_sampler * grammar_sampler = llama_sampler_init_grammar(
            vocab,
            sampling_params.grammar_str.c_str(), // convert to const char *
            "root"                              // rule entry point in GBNF file
        );

        if (!grammar_sampler) {
            throw std::runtime_error(
                "[SAMPLER] failed to initialize grammar sampler"
            );
        }

        llama_sampler_chain_add(sampler, grammar_sampler);
        std::cout << "[SAMPLER] GBNF grammar successfuly loaded." << std::endl;
    }

    //      1. init penalties first (especially relevant for SLMs)
    const int32_t n_vocab = llama_vocab_n_tokens(vocab);
    llama_sampler_chain_add(
        sampler,
        llama_sampler_init_penalties(
            n_vocab,
            sampling_params.penalty_last_n,
            sampling_params.penalty_repeat,
            sampling_params.penalty_freq,
            sampling_params.penalty_present
        )
    );
    
    //      2. Filtering/Cropping (set tail-tokens to 0)
    llama_sampler_chain_add(sampler,
        llama_sampler_init_top_k(sampling_params.top_k));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_top_p(sampling_params.top_p, 1));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_min_p(sampling_params.min_p, 1));
    
    //      3. init temp (adjust probability curve)
    llama_sampler_chain_add(sampler,
        llama_sampler_init_temp(sampling_params.temperature));

    //      4. Distribution last
    llama_sampler_chain_add(sampler,
        llama_sampler_init_dist(sampling_params.seed));
}

void LLM::reset_messages() {
    text_storage.clear();
    messages.clear();

    reset_kv_cache();

    add_message("system", system_prompt);
}

void LLM::reset_all() {
    text_storage.clear();
    messages.clear();

    reset_context();

    add_message("system", system_prompt);
}

void LLM::reset_kv_cache() {
    if (ctx) {
        llama_memory_t memory = llama_get_memory(ctx);

        if (memory) {
            llama_memory_clear(memory, true);
        }
    }

    kv_len       = 0;
    kv_len_token = 0;
}

void LLM::reset_context() {
    if (ctx) {
        llama_free(ctx);
        ctx = nullptr;
    }

    llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx       = ctx_size_requested;   // value requested by user (or default value)
    cparams.n_threads   = n_threads;
    cparams.n_batch     = ctx_size_requested;   // when model uses flash attention, it has strict rules checking n_batch.

    ctx = llama_init_from_model(model, cparams);

    reset_sampler(sampling_params);
    reset_kv_cache();
}

std::string LLM::token_to_piece(
    llama_token token
) const {
    std::vector<char> buffer(256);

    int32_t n = llama_token_to_piece(
        vocab,
        token,
        buffer.data(),
        static_cast<int32_t>(buffer.size()),
        0,
        true
    );

    if (n < 0) {
        buffer.resize(static_cast<size_t>(-n));

        n = llama_token_to_piece(
            vocab,
            token,
            buffer.data(),
            static_cast<int32_t>(buffer.size()),
            0,
            true
        );
    }

    if (n < 0) {
        throw std::runtime_error(
            "llama_token_to_piece failed after resizing buffer for token"
        );
    }

    return std::string(
        buffer.data(),
        static_cast<size_t>(n)
    );
}