#pragma once
#include "llm.hpp"
#include <string>

class LLMHandler {
public:
    LLMHandler(
        LLM& llm
    );

    void operator()(
        const std::string& prompt
    );

private:
    LLM& llm;
    const char* stop_reason_to_string(GenerationStopReason reason);
};
