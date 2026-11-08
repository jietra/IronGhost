// src/llm/response_parser.rs

pub fn extract_json(response: &str) -> Option<&str> {
    if let Some(pos) = response.rfind("</think>") {
        return Some(response[pos + "</think>".len()..].trim());
    }

    Some(response.trim())
}

// no need for Option => simplify extract_json:
pub fn strip_thinking(
    response: &str
) -> &str {
    response
        .rsplit_once("</think>")
        .map(|(_, after)| after.trim())
        .unwrap_or_else(|| response.trim())
}
