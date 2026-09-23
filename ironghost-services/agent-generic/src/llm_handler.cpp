#include "llm_handler.hpp"
#include <chrono>
#include <iostream>

LLMHandler::LLMHandler(LLM& llm) : llm(llm) {}

//std::string LLMHandler::operator()(const std::string& prompt) {
void LLMHandler::operator()(const std::string& prompt) {
    auto start = std::chrono::high_resolution_clock::now();
    //auto result = llm.generate(prompt);
    llm.generate(prompt);
    auto end = std::chrono::high_resolution_clock::now();

    std::cout << "Generation took "
              << std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count()
              << " ms\n";
    /*
    if (llm.stream_callback) {
        return "\n=== END_OF_STREAM ===";
    }
    return result;
    */
    if (llm.stream_callback) {
        llm.stream_callback("\n=== END_OF_STREAM ===");
    }
}
