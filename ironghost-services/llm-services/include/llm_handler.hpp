#pragma once
#include "llm.hpp"
#include <string>

class LLMHandler {
    public:
        LLMHandler(
            LLM& llm
        );

        GenerationResult operator()(
            const std::string& prompt
        );

    private:
        LLM& llm;
};
