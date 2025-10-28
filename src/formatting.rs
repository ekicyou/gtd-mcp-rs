//! Formatting helper functions for GTD MCP server
//!
//! This module contains formatting logic for displaying notas and other output.

use crate::gtd::{Nota, NotaStatus};
use chrono::NaiveDate;

/// Apply date filtering to notas (only affects calendar status items)
///
/// # Arguments
/// * `notas` - Mutable slice of notas to filter
/// * `filter_date` - Date to filter by
///
/// # Description
/// Filters calendar status items to only show those with start_date <= filter_date.
/// Non-calendar items are not affected by date filtering.
pub fn apply_date_filter(notas: &mut Vec<Nota>, filter_date: NaiveDate) {
    notas.retain(|nota| {
        // Only apply date filtering to calendar status tasks
        if nota.status == NotaStatus::calendar {
            // Keep tasks where start_date is not set OR start_date <= filter_date
            // This hides tasks scheduled for future dates
            nota.start_date
                .is_none_or(|task_date| task_date <= filter_date)
        } else {
            // For non-calendar tasks, keep all
            true
        }
    });
}

/// Apply keyword filtering (case-insensitive search in id, title, and notes)
///
/// # Arguments
/// * `notas` - Mutable slice of notas to filter
/// * `keyword` - Keyword to search for (case-insensitive)
pub fn apply_keyword_filter(notas: &mut Vec<Nota>, keyword: &str) {
    let keyword_lower = keyword.to_lowercase();
    notas.retain(|nota| {
        // Search in id
        let id_matches = nota.id.to_lowercase().contains(&keyword_lower);

        // Search in title
        let title_matches = nota.title.to_lowercase().contains(&keyword_lower);

        // Search in notes if present
        let notes_matches = nota
            .notes
            .as_ref()
            .map(|n| n.to_lowercase().contains(&keyword_lower))
            .unwrap_or(false);

        id_matches || title_matches || notes_matches
    });
}

/// Apply project filtering
///
/// # Arguments
/// * `notas` - Mutable slice of notas to filter
/// * `project_id` - Project ID to filter by
pub fn apply_project_filter(notas: &mut Vec<Nota>, project_id: &str) {
    notas.retain(|nota| {
        nota.project
            .as_ref()
            .map(|p| p == project_id)
            .unwrap_or(false)
    });
}

/// Apply context filtering
///
/// # Arguments
/// * `notas` - Mutable slice of notas to filter
/// * `context_name` - Context name to filter by
pub fn apply_context_filter(notas: &mut Vec<Nota>, context_name: &str) {
    notas.retain(|nota| {
        nota.context
            .as_ref()
            .map(|c| c == context_name)
            .unwrap_or(false)
    });
}

/// Format notas into a display string
///
/// # Arguments
/// * `notas` - Vector of notas to format
/// * `exclude_notes` - Whether to exclude notes from output
///
/// # Returns
/// Formatted string representation of the notas
pub fn format_notas(notas: Vec<Nota>, exclude_notes: bool) -> String {
    if notas.is_empty() {
        return "No items found".to_string();
    }

    let mut result = format!("Found {} item(s):\n\n", notas.len());
    for nota in notas {
        let nota_type = if nota.is_context() {
            "context"
        } else if nota.is_project() {
            "project"
        } else {
            "task"
        };

        result.push_str(&format!(
            "- [{}] {} (status: {:?}, type: {})\n",
            nota.id, nota.title, nota.status, nota_type
        ));

        if let Some(ref proj) = nota.project {
            result.push_str(&format!("  Project: {}\n", proj));
        }
        if let Some(ref ctx) = nota.context {
            result.push_str(&format!("  Context: {}\n", ctx));
        }
        if !exclude_notes && let Some(ref n) = nota.notes {
            result.push_str(&format!("  Notes: {}\n", n));
        }
        if let Some(ref date) = nota.start_date {
            result.push_str(&format!("  Start date: {}\n", date));
        }
        // Display timestamps
        result.push_str(&format!("  Created: {}\n", nota.created_at));
        result.push_str(&format!("  Updated: {}\n", nota.updated_at));
    }

    result
}
