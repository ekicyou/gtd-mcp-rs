//! Line ending normalization functions
//!
//! This module handles normalization of line endings in string fields
//! to ensure consistent formatting across different platforms.

use super::legacy_types::{Context, Project, Task};
use std::collections::HashMap;

/// Normalize line endings in a string to LF (\n)
///
/// This handles cases where TOML files contain \r escape sequences that get
/// unescaped to CR bytes during deserialization. We normalize these to LF
/// so they can be properly converted to OS-native format on save.
///
/// # Arguments
///
/// * `s` - String to normalize
///
/// # Returns
///
/// String with normalized line endings
pub fn normalize_string_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Normalize line endings in all string fields of tasks
///
/// # Arguments
///
/// * `tasks` - Mutable reference to a slice of tasks
pub fn normalize_task_line_endings(tasks: &mut [Task]) {
    for task in tasks.iter_mut() {
        if let Some(notes) = &task.notes {
            task.notes = Some(normalize_string_line_endings(notes));
        }
    }
}

/// Normalize line endings in all string fields of projects
///
/// # Arguments
///
/// * `projects` - Mutable reference to the projects HashMap
pub fn normalize_project_line_endings(projects: &mut HashMap<String, Project>) {
    for project in projects.values_mut() {
        if let Some(notes) = &project.notes {
            project.notes = Some(normalize_string_line_endings(notes));
        }
    }
}

/// Normalize line endings in all string fields of contexts
///
/// # Arguments
///
/// * `contexts` - Mutable reference to the contexts HashMap
pub fn normalize_context_line_endings(contexts: &mut HashMap<String, Context>) {
    for context in contexts.values_mut() {
        if let Some(notes) = &context.notes {
            context.notes = Some(normalize_string_line_endings(notes));
        }
    }
}
