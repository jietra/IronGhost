#pragma once
#include "llm.hpp"
#include <string>

class LLMHandler {
public:
    LLMHandler(LLM& llm);

    void operator()(const std::string& prompt);

private:
    LLM& llm;
};
