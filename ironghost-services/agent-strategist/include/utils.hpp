#pragma once
#include <string>
#include <fstream>
#include <sstream>

inline std::string read_file(const std::string& path) {
    std::ifstream f(path);
    std::stringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

/*
bool is_valid_utf8(const std::string& s);
std::string sanitize_utf8_piece(const std::string& s);
*/