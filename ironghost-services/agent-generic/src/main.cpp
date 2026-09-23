#include "llm.hpp"
#include "llm_handler.hpp"
#include "uds_server.hpp"
#include "utils.hpp"
#include <iostream>
#include <fstream>
#include <unordered_map>
#include <vector>

std::unordered_map<std::string, std::string> load_env(const std::string& path) {
    std::unordered_map<std::string, std::string> env;
    std::ifstream f(path);
    std::string line;

    while (std::getline(f, line)) {
        if (line.empty() || line[0] == '#') continue;
        auto pos = line.find('=');
        if (pos == std::string::npos) continue;
        env[line.substr(0, pos)] = line.substr(pos + 1);
    }
    return env;
}

int main() {
    auto env = load_env("../.env"); // launched from ceo/build

    std::cout << "MODEL_PATH=["             << env["MODEL_PATH"]             << "]\n";
    std::cout << "SYSTEM_PROMPT=["          << env["SYSTEM_PROMPT"]          << "]\n";
    std::cout << "N_THREADS=["              << env["N_THREADS"]              << "]\n";
    std::cout << "N_GPU_LAYERS=["           << env["N_GPU_LAYERS"]           << "]\n";
    std::cout << "CONTEXT_SIZE_REQUESTED=[" << env["CONTEXT_SIZE_REQUESTED"] << "]\n";

    LLM llm(
        env["MODEL_PATH"],
        read_file(env["SYSTEM_PROMPT"]),
        std::stoi(env["N_THREADS"]),
        std::stoi(env["N_GPU_LAYERS"]),
        std::stoi(env["CONTEXT_SIZE_REQUESTED"])
    );

    LLMHandler handler(llm);

    UDSServer server(
        llm,
        env["SOCKET_PATH"],
        handler
    );

    server.run();
}

