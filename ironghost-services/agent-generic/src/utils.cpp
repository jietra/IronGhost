#include "utils.hpp"
/*
bool is_valid_utf8(const std::string& s) {
    int i = 0, len = s.size();
    while (i < len) {
        unsigned char c = s[i];
        int bytes = 
            (c < 0x80) ? 1 :
            (c < 0xE0) ? 2 :
            (c < 0xF0) ? 3 :
            (c < 0xF8) ? 4 : 0;

        if (bytes == 0 || i + bytes > len) return false;

        for (int j = 1; j < bytes; j++) {
            if ((s[i + j] & 0xC0) != 0x80) return false;
        }

        i += bytes;
    }
    return true;
}


std::string sanitize_utf8_piece(const std::string& s) {
    std::string out;
    out.reserve(s.size());

    for (size_t i = 0; i < s.size(); ) {
        unsigned char c = s[i];

        // 1. ASCII control chars except \n and \t
        if (c < 0x20) {
            if (c == '\n' || c == '\t') {
                out.push_back(c);
            }
            i++;
            continue;
        }

        // 2. Normalize exotic whitespace (ASCII)
        if (c == '\v' || c == '\f' || c == '\r') {
            out.push_back(' ');
            i++;
            continue;
        }

        // 3. Normalize UTF‑8 exotic spaces
        //    U+00A0 (NBSP) = C2 A0
        //    U+2009 (thin space) = E2 80 89
        //    U+202F (narrow NBSP) = E2 80 AF
        //    U+2007 (figure space) = E2 80 87
        //    U+2008 (punctuation space) = E2 80 88
        //    U+200A (hair space) = E2 80 8A
        if (c == 0xC2 && i + 1 < s.size() && (unsigned char)s[i+1] == 0xA0) {
            out.push_back(' ');
            i += 2;
            continue;
        }
        if (c == 0xE2 && i + 2 < s.size()) {
            unsigned char c1 = s[i+1];
            unsigned char c2 = s[i+2];

            if (c1 == 0x80 && (
                c2 == 0x89 || // thin space
                c2 == 0xAF || // narrow NBSP
                c2 == 0x87 || // figure space
                c2 == 0x88 || // punctuation space
                c2 == 0x8A    // hair space
            )) {
                out.push_back(' ');
                i += 3;
                continue;
            }
        }

        // 4. UTF‑8 multi‑byte sequences (2–4 bytes)
        if (c >= 0xC2) {
            size_t seq_len =
                (c < 0xE0) ? 2 :
                (c < 0xF0) ? 3 :
                (c < 0xF8) ? 4 : 1;

            if (i + seq_len <= s.size()) {
                out.append(s, i, seq_len);
                i += seq_len;
                continue;
            }

            break; // incomplete sequence -> ignore
        }

        // 5. ASCII normal
        out.push_back(c);
        i++;
    }

    return out;
}


std::string sanitize_utf8_piece(const std::string& s) {
    std::string out;
    out.reserve(s.size());

    for (size_t i = 0; i < s.size(); ) {
        unsigned char c = s[i];

        // 1. ASCII control chars → replace with space
        if (c < 0x20) {
            out.push_back(' ');
            i++;
            continue;
        }

        // 2. ASCII exotic whitespace → replace with space
        if (c == '\t' || c == '\v' || c == '\f' || c == '\r') {
            out.push_back(' ');
            i++;
            continue;
        }

        // 3. NBSP (C2 A0)
        if (c == 0xC2 && i + 1 < s.size() && (unsigned char)s[i+1] == 0xA0) {
            out.push_back(' ');
            i += 2;
            continue;
        }

        // 4. All U+2000–U+200A spaces + U+202F + U+205F
        if (c == 0xE2 && i + 2 < s.size()) {
            unsigned char c1 = s[i+1];
            unsigned char c2 = s[i+2];

            if (c1 == 0x80 && (c2 >= 0x80 && c2 <= 0x8A)) {
                out.push_back(' ');
                i += 3;
                continue;
            }
            if (c1 == 0x80 && c2 == 0xAF) { // narrow NBSP
                out.push_back(' ');
                i += 3;
                continue;
            }
            if (c1 == 0x81 && c2 == 0x9F) { // medium mathematical space
                out.push_back(' ');
                i += 3;
                continue;
            }
        }

        // 5. UTF‑8 multi‑byte sequences (2–4 bytes)
        if (c >= 0xC2) {
            size_t seq_len =
                (c < 0xE0) ? 2 :
                (c < 0xF0) ? 3 :
                (c < 0xF8) ? 4 : 1;

            if (i + seq_len <= s.size()) {
                out.append(s, i, seq_len);
                i += seq_len;
                continue;
            }

            break; // incomplete sequence
        }

        // 6. ASCII normal
        out.push_back(c);
        i++;
    }

    return out;
}
*/