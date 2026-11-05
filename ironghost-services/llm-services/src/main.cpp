#include "config.hpp"
#include "llm.hpp"
#include "llm_handler.hpp"
#include "uds_server.hpp"
#include "utils.hpp"

#include <iostream>

int main(
    int argc,
    char** argv
) {
    try {
        if (argc != 3) {
            std::cerr << "Usage:\n" << argv[0] << " <config.yaml> <service-id>\n";
            return 1;
        }

        ServiceConfig config = load_service_config(
            argv[1],
            argv[2]
        );

        const std::string sys_prompt= read_file(config.system_prompt_path);
        config.sampling.grammar_str = read_file(config.grammar_path);

        std::cout << "[DEBUG] Loaded system prompt length: " << sys_prompt.size() << " chars\n";
        if (sys_prompt.empty()) {
            std::cerr << "[WARNING] System prompt is empty! Check file path.\n";
        }
        std::cout << "[DEBUG] Loaded grammar file, of length: " << config.sampling.grammar_str.size() << " chars\n";
        if (config.sampling.grammar_str.empty()) {
            std::cerr << "[WARNING] grammar is empty! Check file path.\n";
        }

        std::cout
            << "[SERVICE] "         << config.id            << "\n"
            << "  MODEL_PATH =["    << config.model_path    << "]\n"
            << "  SOCKET_PATH=["    << config.socket_path   << "]\n\n";
        
        std::cout
            << "  -------------------------------------------------------------------\n"
            << "  [RUNTIME]\n"
            << "    N_THREADS=["              << config.runtime.threads         << "]\n"
            << "    N_GPU_LAYERS=["           << config.runtime.gpu_layers      << "]\n"
            << "    CONTEXT_SIZE_REQUESTED=[" << config.runtime.context_size    << "]\n"
            << "  -------------------------------------------------------------------\n\n";

        std::cout
            << "  -------------------------------------------------------------------\n"
            << "  [SAMPLING]\n"
            << "    TEMPERATURE=["      << config.sampling.temperature        << "]\n"
            << "    TOP_P=["            << config.sampling.top_p              << "]\n"
            << "    TOP_K=["            << config.sampling.top_k              << "]\n"
            << "    MIN_P=["            << config.sampling.min_p              << "]\n"
            << "    PENALTY_LAST_N=["   << config.sampling.penalty_last_n     << "]\n"
            << "    PENALTY_REPEAT=["   << config.sampling.penalty_repeat     << "]\n"
            << "    PENALTY_FREQ=["     << config.sampling.penalty_freq       << "]\n"
            << "    PENALTY_PRESENT=["  << config.sampling.penalty_present    << "]\n"
            << "  -------------------------------------------------------------------\n\n";

        if (config.behavior.print_prompt) {
            std::cout
                << "SYSTEM_PROMPT=\n" << sys_prompt                   << "\n"
                << "------------------------------------------------------\n\n"
                << "GRAMMAR_STR  =\n" << config.sampling.grammar_str  << "\n\n";
        }

        LLM llm(
            config.model_path.string(),
            sys_prompt,
            config.runtime.threads,
            config.runtime.gpu_layers,
            config.runtime.context_size,
            config.behavior.amnesia,
            config.sampling
        );

        LLMHandler handler(llm);

        UDSServer server(
            llm,
            config.socket_path,
            handler
        );

        server.run();

        return 0;

    } catch (const std::exception& error) {
        std::cerr
            << "[FATAL] "
            << error.what()
            << '\n';

        return 1;
    }
}