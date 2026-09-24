#pragma once
#include <string>
#include <fstream>
#include <sstream>
#include <iostream>

inline std::string read_file(const std::string& path) {
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