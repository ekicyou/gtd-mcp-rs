//! Validation helper functions for GTD MCP server
//!
//! This module contains validation logic for status filters, date parsing,
//! and reference validation (projects and contexts).

use crate::gtd::{GtdData, NotaStatus};
use chrono::NaiveDate;
use mcp_attr::Result as McpResult;

/// Parse and validate status filter parameter
///
/// # Arguments
/// * `status_str` - Status string to parse
///
/// # Returns
/// Result containing parsed NotaStatus or error
pub fn parse_status_filter(status_str: &str) -> McpResult<NotaStatus> {
    status_str.parse::<NotaStatus>().map_err(|_| {
        mcp_attr::Error::new(mcp_attr::ErrorCode::INVALID_PARAMS).with_message(
            format!(
                "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, reference, trash, project, context",
                status_str
            ),
            true,
        )
    })
}

/// Parse and validate date filter parameter
///
/// # Arguments
/// * `date_str` - Date string in YYYY-MM-DD format
///
/// # Returns
/// Result containing parsed NaiveDate or error
pub fn parse_date_filter(date_str: &str) -> McpResult<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
        mcp_attr::Error::new(mcp_attr::ErrorCode::INVALID_PARAMS).with_message(
            format!(
                "Invalid date format '{}'. Use YYYY-MM-DD (e.g., '2025-03-15')",
                date_str
            ),
            true,
        )
    })
}

/// Format an error message for invalid project reference with available projects list
///
/// # Arguments
/// * `project_id` - The invalid project ID that was provided
/// * `data` - Reference to GtdData to get available projects
///
/// # Returns
/// A formatted error message including the list of available projects
pub fn format_invalid_project_error(project_id: &str, data: &GtdData) -> String {
    let projects = data.projects();
    if projects.is_empty() {
        format!(
            "Project '{}' does not exist. No projects have been created yet. Create a project first using inbox() with status='project'.",
            project_id
        )
    } else {
        let project_list: Vec<String> = projects.keys().cloned().collect();
        format!(
            "Project '{}' does not exist.\nAvailable projects: {}",
            project_id,
            project_list.join(", ")
        )
    }
}

/// Format an error message for invalid context reference with available contexts list
///
/// # Arguments
/// * `context_name` - The invalid context name that was provided
/// * `data` - Reference to GtdData to get available contexts
///
/// # Returns
/// A formatted error message including the list of available contexts
pub fn format_invalid_context_error(context_name: &str, data: &GtdData) -> String {
    let contexts = data.contexts();
    if contexts.is_empty() {
        format!(
            "Context '{}' does not exist. No contexts have been created yet. Create a context first using inbox() with status='context'.",
            context_name
        )
    } else {
        let context_list: Vec<String> = contexts.keys().cloned().collect();
        format!(
            "Context '{}' does not exist.\nAvailable contexts: {}",
            context_name,
            context_list.join(", ")
        )
    }
}

/// Normalize task ID by returning it as-is (no transformation)
///
/// This helper function previously added '#' prefix for backwards compatibility,
/// but now task IDs are arbitrary strings chosen by the MCP client.
///
/// # Arguments
/// * `task_id` - The task ID (e.g., "task-1", "meeting-prep")
///
/// # Returns
/// The task ID unchanged
///
/// # Examples
/// ```
/// # use gtd_mcp::validation::normalize_task_id;
/// // normalize_task_id("task-1") -> "task-1"
/// // normalize_task_id("meeting-prep") -> "meeting-prep"
/// ```
pub fn normalize_task_id(task_id: &str) -> String {
    task_id.trim().to_string()
}

#[cfg(test)]
/// Extract ID from response message
///
/// Helper function for tests to extract ID from response messages.
/// Response format: "Item created with ID: <id> (type: task)"
///
/// # Arguments
/// * `response` - The response message from inbox() or similar operations
///
/// # Returns
/// The extracted ID
pub fn extract_id_from_response(response: &str) -> String {
    // Parse "Item created with ID: <id> (type: ...)"
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
