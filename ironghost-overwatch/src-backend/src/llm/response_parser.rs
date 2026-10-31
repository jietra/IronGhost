// src/llm/response_parser.rs

pub fn extract_json(response: &str) -> Option<&str> {
    if let Some(pos) = response.rfind("</think>") {
        return Some(response[pos + "</think>".len()..].trim());
    }

    Some(response.trim())
}