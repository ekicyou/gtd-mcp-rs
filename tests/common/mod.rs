//! Common test utilities for integration tests

/// Extract task ID from inbox() response message
/// Response format: "Nota created with ID: <id> (type: task)"
pub fn extract_id_from_response(response: &str) -> String {
    // Parse "Nota created with ID: <id> (type: ...)"
    if let Some(start) = response.find("ID: ") {
        let id_part = &response[start + 4..];
        if let Some(end) = id_part.find(" (") {
            return id_part[..end].trim().to_string();
        }
    }
    // Fallback: try to get last whitespace-separated token without parentheses
    response
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_end_matches(')')
        .to_string()
}
