//! Common test utilities for integration tests

use gtd_mcp::{GtdServerHandler, Nota, NotaStatus};
use gtd_mcp::gtd::local_date_today;
use gtd_mcp::migration::Task;
use tempfile::NamedTempFile;
use chrono::NaiveDate;

/// Create a test handler with temporary storage
pub fn get_test_handler() -> (GtdServerHandler, NamedTempFile) {
    let temp_file = NamedTempFile::new().unwrap();
    let handler = GtdServerHandler::new(temp_file.path().to_str().unwrap(), false).unwrap();
    (handler, temp_file)
}

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

/// Create a test task with minimal fields
pub fn create_test_task(id: &str, title: &str, status: NotaStatus) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
    }
}

/// Create a test task with all fields
pub fn create_full_test_task(
    id: &str,
    title: &str,
    status: NotaStatus,
    project: Option<String>,
    context: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status,
        project,
        context,
        notes,
        start_date,
        created_at: local_date_today(),
        updated_at: local_date_today(),
    }
}
