#pragma once
#include <string>
#include <fstream>
#include <sstream>
#include <iostream>

inline std::string read_file(const std::filesystem::path& path) {
    std::ifstream f(path, std::ios::in | std::ios::binary);
    
    if (!f.is_open()) {
        std::cerr << "[ERROR] Could not open file: " 
                  << path << std::endl;
        return "";
    }
    
    std::stringstream ss;
    ss << f.rdbuf();
    std::string content = ss.str();

    if (content.empty()) {
        std::cerr << "[WARNING] File is empty: " << path << std::endl;
    }

    return content;
}

/*
std::string read_required_file(
    const std::filesystem::path& path
) {
    std::ifstream f(path, std::ios::binary);

    if (!f) {
        throw std::runtime_error(
            "[ERROR] Could not open file: "
            + path.string()
        );
    }

    std::stringstream ss;
    ss << f.rdbuf();
    std::string content = ss.str();

    if (content.empty()) {
        throw std::runtime_error(
            "[ERROR] Required file is empty: "
            + path.string()
        );
    }

    return content;
}
*/