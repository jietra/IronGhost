#include "config.hpp"

#include <yaml-cpp/yaml.h>

#include <filesystem>
#include <stdexcept>
#include <string>

namespace {

    template <typename T>
    T value_or(
        const YAML::Node&   node,
        const char*         key,
        const T&            fallback
    ) {
        const YAML::Node value = node[key];

        if (!value) {
            return fallback;
        }

        return value.as<T>();
    }

    template <typename T>
    T required(
        const YAML::Node&   node,
        const char*         key,
        const std::string&  context
    ) {
        const YAML::Node value = node[key];

        if (!value) {
            throw std::runtime_error(
                "missing required config field '"
                + std::string(key)
                + "' in "
                + context
            );
        }

        return value.as<T>();
    }

    std::filesystem::path resolve_path(
        const std::filesystem::path&    config_dir,
        const std::string&              raw_path
    ) {
        std::filesystem::path path(raw_path);

        if (path.is_absolute()) {
            return path.lexically_normal();
        }

        return (
            config_dir / path
        ).lexically_normal();
    }

    void apply_runtime(
        RuntimeParams&      output,
        const YAML::Node&   node
    ) {
        if (!node) {
            return;
        }

        output.threads = value_or<int32_t>(
            node,
            "threads",
            output.threads
        );

        output.gpu_layers = value_or<int32_t>(
            node,
            "gpu_layers",
            output.gpu_layers
        );

        output.context_size = value_or<int32_t>(
            node,
            "context_size",
            output.context_size
        );
    }

    void apply_sampling(
        SamplingParams&     output,
        const YAML::Node&   node
    ) {
        if (!node) {
            return;
        }

        output.temperature = value_or<float>(
            node,
            "temperature",
            output.temperature
        );

        output.top_p = value_or<float>(
            node,
            "top_p",
            output.top_p
        );

        output.top_k = value_or<int32_t>(
            node,
            "top_k",
            output.top_k
        );

        output.min_p = value_or<float>(
            node,
            "min_p",
            output.min_p
        );

        output.penalty_last_n = value_or<int32_t>(
            node,
            "penalty_last_n",
            output.penalty_last_n
        );

        output.penalty_repeat = value_or<float>(
            node,
            "penalty_repeat",
            output.penalty_repeat
        );

        output.penalty_freq = value_or<float>(
            node,
            "penalty_freq",
            output.penalty_freq
        );

        output.penalty_present = value_or<float>(
            node,
            "penalty_present",
            output.penalty_present
        );

        output.seed = value_or<uint32_t>(
            node,
            "seed",
            output.seed
        );

        output.max_tokens = value_or<int32_t>(
            node,
            "max_tokens",
            output.max_tokens
        );

    }

    void apply_behavior(
        BehaviorParams&     output,
        const YAML::Node&   node
    ) {
        if (!node) {
            return;
        }

        output.amnesia = value_or<bool>(
            node,
            "amnesia",
            output.amnesia
        );

        output.enable_debug = value_or<bool>(
            node,
            "enable_debug",
            output.enable_debug
        );

        output.print_prompt = value_or<bool>(
            node,
            "print_prompt",
            output.print_prompt
        );
    }

} // namespace

ServiceConfig load_service_config(
    const std::filesystem::path&    config_path,
    const std::string&              service_id
) {
    const std::filesystem::path absolute_config =
        std::filesystem::absolute(
            config_path
        ).lexically_normal();

    const std::filesystem::path config_dir =
        absolute_config.parent_path();

    YAML::Node root;

    try {
        root = YAML::LoadFile(
            absolute_config.string()
        );
    } catch (const YAML::Exception& error) {
        throw std::runtime_error(
            "failed to load config '"
            + absolute_config.string()
            + "': "
            + error.what()
        );
    }

    const YAML::Node services =
        root["services"];

    if (!services) {
        throw std::runtime_error(
            "missing 'services' section in config"
        );
    }

    const YAML::Node service =
        services[service_id];

    if (!service) {
        throw std::runtime_error(
            "unknown service id: "
            + service_id
        );
    }

    ServiceConfig config;

    config.id = service_id;

    // ----------------------------
    // Defaults
    // ----------------------------

    const YAML::Node defaults =
        root["defaults"];

    if (defaults) {
        apply_runtime(
            config.runtime,
            defaults["runtime"]
        );

        apply_sampling(
            config.sampling,
            defaults["sampling"]
        );

        apply_behavior(
            config.behavior,
            defaults["behavior"]
        );
    }

    // ----------------------------
    // Service overrides
    // ----------------------------

    apply_runtime(
        config.runtime,
        service["runtime"]
    );

    apply_sampling(
        config.sampling,
        service["sampling"]
    );

    apply_behavior(
        config.behavior,
        service["behavior"]
    );

    // ----------------------------
    // Required service fields
    // ----------------------------

    const std::string context =
        "service '" + service_id + "'";

    config.socket_path =
        required<std::string>(
            service,
            "socket",
            context
        );

    config.model_path =
        resolve_path(
            config_dir,
            required<std::string>(
                service,
                "model",
                context
            )
        );

    config.system_prompt_path =
        resolve_path(
            config_dir,
            required<std::string>(
                service,
                "system_prompt",
                context
            )
        );

    config.grammar_path =
        resolve_path(
            config_dir,
            required<std::string>(
                service,
                "grammar",
                context
            )
        );

    // ----------------------------
    // Basic validation
    // ----------------------------

    if (config.runtime.threads <= 0) {
        throw std::runtime_error(
            context
            + ": runtime.threads must be > 0"
        );
    }

    if (config.runtime.context_size <= 0) {
        throw std::runtime_error(
            context
            + ": runtime.context_size must be > 0"
        );
    }

    if (config.socket_path.empty()) {
        throw std::runtime_error(
            context
            + ": socket must not be empty"
        );
    }

    if (config.sampling.max_tokens <= 0) {
        throw std::runtime_error(
            context + ": sampling.max_tokens must be > 0"
        );
    }

    return config;
}