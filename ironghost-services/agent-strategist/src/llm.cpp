#include "llm.hpp"
#include <iostream>
#include <string>
#include <vector>
#include <cstring>

LLM::LLM(
    const std::string&      model_path,
        const std::string&  system_prompt,
        int32_t             n_threads,
        int32_t             n_gpu_layers,
        int32_t             ctx_size_requested,
        SamplingParams      sampling_params
    )
    : system_prompt(        system_prompt       ),
      n_threads(            n_threads           ),
      ctx_size_requested(   ctx_size_requested  ),
      sampling_params(      sampling_params     )
{
    // load dynamic backends
    ggml_backend_load_all();

    // LLM.model (llama_model_default_params + llama_model_load_from_file)
    llama_model_params mparams  = llama_model_default_params();
    mparams.n_gpu_layers        = n_gpu_layers;

    model = llama_model_load_from_file(model_path.c_str(), mparams);
    if (!model) {
        std::cerr << "ERROR: cannot load model: " << model_path << std::endl;
        std::exit(1);
    }

    // LLM.vocab (llama_model_get_vocab)
    vocab = llama_model_get_vocab(model);

    // LLM.chat_template init
    chat_template = llama_model_chat_template(model, nullptr);
    if (chat_template.empty()) {
        std::cerr << "Warning: model has no chat template" << std::endl;
    }

    // LLM.ctx (llama_context_default_params + llama_init_from_model)
    llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx       = ctx_size_requested;   // value requested by user (or default value)
    cparams.n_threads   = n_threads;
    cparams.n_batch     = ctx_size_requested;   // when model uses flash attention, it has strict rules checking n_batch.

    ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        std::cerr << "ERROR: cannot create llama context" << std::endl;
        std::exit(1);
    }

    // LLM.n_ctx
    n_ctx = llama_n_ctx(ctx);

    // LLM.sampler init
    sampler = llama_sampler_chain_init( llama_sampler_chain_default_params() );
    llama_sampler_chain_add(sampler,
        llama_sampler_init_top_k(sampling_params.top_k));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_top_p(sampling_params.top_p, 1));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_min_p(sampling_params.min_p, 1));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_temp(sampling_params.temperature));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_dist(sampling_params.seed));

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
        std::cerr << "ERROR: failed to apply the chat template\n";
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
    int32_t len = llama_chat_apply_template(
        chat_template.c_str(),
        messages.data(),
        messages.size(),
        false,
        nullptr,
        0
    );
    if (len < 0) {
        std::cerr << "ERROR: failed to apply the chat template\n";
        std::exit(1);
    }
}

std::string LLM::generate(const std::string& user_prompt) {

    // add user_prompt to the conversation history and return formatted prompt
    std::string full_prompt = add_and_format_prompt(user_prompt);

    // run LLM on formatted (last) prompt
    std::string response = run_llm(full_prompt);

    // add response to messages
    add_response(response);

    return response;
}

std::string LLM::run_llm(const std::string& full_prompt) {
    // Tokenize prompt, batch, loop with decode

    // KV cache init test: test if first prompt
    const bool is_first = kv_len_token == 0;

    // --- Tokenize formatted last prompt ---
    int32_t n_prompt_tokens = -llama_tokenize(
        vocab,
        full_prompt.c_str(),
        full_prompt.size(),
        nullptr,
        0,
        is_first,
        true
    );

    std::vector<llama_token> tokens(n_prompt_tokens);

    if (llama_tokenize(
        vocab,
        full_prompt.c_str(),
        full_prompt.size(),
        tokens.data(),
        tokens.size(),
        is_first,
        true
    ) < 0) {
        std::cerr << "ERROR: failed to tokenize prompt" << std::endl;
        std::exit(1);
    }

    // --- Prepare batch ---
    llama_batch batch = llama_batch_get_one(
        tokens.data(),
        tokens.size()
    );
    llama_token new_token_id;
    std::string output;
    std::string utf8_buffer;

    // --- Entering loop ---
    while (true) {
        // check available context space
        if (kv_len_token + batch.n_tokens > n_ctx) {
            std::cerr << "WARNING: context size exceeded. Consider cropping messages." << std::endl;
            std::exit(1);
        }
        // TODO: create a method to crop messages: crop_messages(n_messages), or automatic cropping, or auto summurize.

        // --- Eval prompt ---
        int32_t ret = llama_decode(ctx, batch);
        if (ret != 0) {
            std::cerr << "ERROR: llama_decode failed with ret = " << ret << std::endl;
            std::exit(1);
        }

        // update kv_len_token according to new KV-cache state
        kv_len_token += batch.n_tokens;

        // --- Sample next token ---
        new_token_id = llama_sampler_sample(sampler, ctx, -1);

        // --- EOG? ---
        if (llama_vocab_is_eog(vocab, new_token_id)) {
            std::cout << std::endl;
            break;
        }

        // --- Convert token to string ---
        char buf[256];
        int32_t  n = llama_token_to_piece(
            vocab,
            new_token_id,
            buf,
            sizeof(buf),
            0,
            true
        );
        if (n < 0) {
            std::cerr << "ERROR: failed to convert token to piece" << std::endl;
            std::exit(1);
        }
        std::string piece(buf, n);

        // stream to console (server side) (for debug)
        std::cout << piece << std::flush;

        // streaming to orchestrator
        if (stream_callback) {
            stream_callback(piece);
            /*
            utf8_buffer += piece;
            // Check wether buffer is a valid UTF-8
            if (is_valid_utf8(utf8_buffer)) {
                //std::string clean = sanitize_utf8_piece(utf8_buffer);
                stream_callback(utf8_buffer);
                utf8_buffer.clear();
            }
            */
        }
        
        output += piece;

        // prepare next batch with sampled token
        batch = llama_batch_get_one(&new_token_id, 1);
    }

    return output;
}

void LLM::reset_sampler(const SamplingParams & params) {
    sampling_params = params;
    
    if (sampler) {
        llama_sampler_free(sampler);
    }
    sampler = llama_sampler_chain_init(llama_sampler_chain_default_params());

    llama_sampler_chain_add(sampler,
        llama_sampler_init_top_k(sampling_params.top_k));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_top_p(sampling_params.top_p, 1));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_min_p(sampling_params.min_p, 1));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_temp(sampling_params.temperature));
    llama_sampler_chain_add(sampler,
        llama_sampler_init_dist(sampling_params.seed));
}

void LLM::reset_messages() {
    text_storage.clear();
    messages.clear();
    
    add_message("system", system_prompt);

    reset_kv_cache();
}

void LLM::reset_all() {
    reset_sampler();
    reset_messages();
}

void LLM::reset_kv_cache() {
    kv_len       = 0;
    kv_len_token = 0;
}