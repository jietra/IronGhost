#include "llm.hpp"
#include "llm_handler.hpp"
#include "uds_server.hpp"
#include "utils.hpp"

#include <iostream>
#include <fstream>
#include <unordered_map>

static std::unordered_map<std::string, std::string> load_env(const std::string& path) {
    std::unordered_map<std::string, std::string> env;
    std::ifstream f(path);
    std::string line;

    auto trim = [](std::string& s) {
        s.erase(0, s.find_first_not_of(" \t\r\n"));
        s.erase(s.find_last_not_of(" \t\r\n") + 1);
    };

    while (std::getline(f, line)) {
        if (line.empty() || line[0] == '#') continue;
        auto pos = line.find('=');
        if (pos == std::string::npos) continue;

        //env[line.substr(0, pos)] = line.substr(pos + 1);

        std::string key = line.substr(0, pos);
        std::string val = line.substr(pos + 1);

        trim(key);
        trim(val);

        if (!key.empty()) {
            env[key] = val;
        }
    }
    return env;
}

int main() {
    auto env = load_env("../.env"); // launched from ceo/build

    std::string sys_prompt = read_file(env["SYSTEM_PROMPT"]);
    std::string grammar_str = read_file(env["GRAMMAR_STR"]);

    std::cout << "[DEBUG] Loaded system prompt length: " << sys_prompt.size() << " chars\n";
    if (sys_prompt.empty()) {
        std::cerr << "[WARNING] System prompt is empty! Check file path.\n";
    }
    std::cout << "[DEBUG] Loaded grammar file, of length: " << grammar_str.size() << " chars\n";
    if (sys_prompt.empty()) {
        std::cerr << "[WARNING] grammar is empty! Check file path.\n";
    }

    std::cout << "SYSTEM_PROMPT=\n"         << sys_prompt  << "\n";
    std::cout << "GRAMMAR_STR  =\n"         << grammar_str << "\n";

    std::cout << "MODEL_PATH=["             << env["MODEL_PATH"]             << "]\n";
    std::cout << "N_THREADS=["              << env["N_THREADS"]              << "]\n";
    std::cout << "N_GPU_LAYERS=["           << env["N_GPU_LAYERS"]           << "]\n";
    std::cout << "CONTEXT_SIZE_REQUESTED=[" << env["CONTEXT_SIZE_REQUESTED"] << "]\n";

    std::cout << "TEMPERATURE=["            << env["TEMPERATURE"]            << "]\n";
    std::cout << "TOP_P=["                  << env["TOP_P"]                  << "]\n";
    std::cout << "TOP_K=["                  << env["TOP_K"]                  << "]\n";
    std::cout << "MIN_P=["                  << env["MIN_P"]                  << "]\n";
    std::cout << "PENALTY_LAST_N=["         << env["PENALTY_LAST_N"]         << "]\n";
    std::cout << "PENALTY_REPEAT=["         << env["PENALTY_REPEAT"]         << "]\n";
    std::cout << "PENALTY_FREQ=["           << env["PENALTY_FREQ"]           << "]\n";
    std::cout << "PENALTY_PRESENT=["        << env["PENALTY_PRESENT"]        << "]\n";

    LLM llm(
        env["MODEL_PATH"],
        sys_prompt,
        std::stoi(env["N_THREADS"]),
        std::stoi(env["N_GPU_LAYERS"]),
        std::stoi(env["CONTEXT_SIZE_REQUESTED"]),
        SamplingParams::from_env(env)
    );

    LLMHandler handler(llm);

    UDSServer server(
        llm,
        env["SOCKET_PATH"],
        handler
    );

    server.run();
}