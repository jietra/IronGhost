#include "llm_handler.hpp"
#include <chrono>
#include <iostream>

LLMHandler::LLMHandler(LLM& llm): llm(llm) {}

void LLMHandler::operator()(const std::string& prompt) {
    const auto start = std::chrono::high_resolution_clock::now();

    llm.generate(prompt);

    const auto end = std::chrono::high_resolution_clock::now();

    std::cout << "Generation took "
              << std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count()
              << " ms\n";
    
    if (llm.stream_callback) {
        llm.stream_callback("\n=== END_OF_STREAM ===");
    }
}
