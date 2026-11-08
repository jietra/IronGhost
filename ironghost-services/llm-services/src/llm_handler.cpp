#include "llm_handler.hpp"
#include <chrono>
#include <iostream>

LLMHandler::LLMHandler(LLM& llm): llm(llm) {}

void LLMHandler::operator()(const std::string& prompt) {
    const auto start = std::chrono::high_resolution_clock::now();

    const GenerationResult result = llm.generate(prompt);

    const auto end = std::chrono::high_resolution_clock::now();

    std::cout << "Generation took "
              << std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count()
              << " ms\n";
    
    if (llm.stream_callback) {
        const std::string event = std::string(R"({"event":"end","reason":")")
            + stop_reason_to_string(result.reason)
            + R"("})";

        llm.stream_callback(event);

        //llm.stream_callback("\n=== END_OF_STREAM ===");
    }
}

const char* LLMHandler::stop_reason_to_string(
    GenerationStopReason reason
) {
    switch (reason) {
        case GenerationStopReason::EndOfGeneration:
            return "eog";

        case GenerationStopReason::MaxTokens:
            return "max_tokens";

        case GenerationStopReason::Cancelled:
            return "cancelled";

        case GenerationStopReason::ContextFull:
            return "context_full";

        case GenerationStopReason::TokenizerError:
            return "tokenizer_error";

        case GenerationStopReason::DecodeError:
            return "decode_error";

        case GenerationStopReason::TokenToPieceError:
            return "token_to_piece_error";

        case GenerationStopReason::Unknown:
            return "unknown";

        case GenerationStopReason::Init:
            return "invalid_internal_state";
    }

    return "unknown";
}
