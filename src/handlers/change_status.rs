//! Change status handler for GTD MCP server

use crate::GtdServerHandler;
use crate::gtd::{self, NotaStatus};
use crate::validation;
use chrono::NaiveDate;
use mcp_attr::{Result as McpResult, bail_public};

impl GtdServerHandler {
    /// **Organize/Do**: Move items through workflow stages as you process them.
    /// **When**: inbox→next_action(ready) | →waiting_for(blocked) | →done(complete) | →trash(discard).
    /// **Tip**: Use change_status to trash before empty_trash to permanently delete.
    /// **Batch**: Supports multiple IDs for efficient batch operations (e.g., weekly review).
    pub async fn handle_change_status(
        &self,
        ids: Vec<String>,
        new_status: String,
        start_date: Option<String>,
    ) -> McpResult<String> {
        // Validate we have at least one ID
        if ids.is_empty() {
            bail_public!(_, "No IDs provided. Please specify at least one item ID.");
        }

        let mut data = self.data.lock().unwrap();

        // Parse new status once
        let nota_status: NotaStatus = match new_status.parse() {
            Ok(s) => s,
            Err(_) => {
                drop(data);
                bail_public!(
                    _,
                    "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, reference, trash, project, context",
                    new_status
                );
            }
        };

        let is_trash = nota_status == NotaStatus::trash;

        // Parse start_date once if provided
        let parsed_start_date = if let Some(date_str) = &start_date {
            match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => {
                    drop(data);
                    bail_public!(
                        _,
                        "Invalid date format '{}'. Use YYYY-MM-DD (e.g., '2025-03-15')",
                        date_str
                    );
                }
            }
        } else {
            None
        };

        // Track successes and failures
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        // Normalize all IDs upfront for efficiency
        let normalized_ids: Vec<String> = ids
            .iter()
            .map(|id| validation::normalize_task_id(id))
            .collect();

        // Process each ID
        for normalized_id in normalized_ids {
            // Find existing nota
            let mut nota = match data.find_by_id(&normalized_id) {
                Some(n) => n,
                None => {
                    failures.push(format!("{}: not found", normalized_id));
                    continue;
                }
            };

            // Store old status for reporting
            let old_status = nota.status.clone();

            // Validate calendar status has start_date
            if nota_status == NotaStatus::calendar
                && parsed_start_date.is_none()
                && nota.start_date.is_none()
            {
                failures.push(format!(
                    "{}: calendar status requires a start_date",
                    normalized_id
                ));
                continue;
            }

            // Check if moving to trash and if nota is still referenced
            if is_trash && data.is_referenced(&normalized_id) {
                failures.push(format!(
                    "{}: still referenced by other items",
                    normalized_id
                ));
                continue;
            }

            // Update status
            nota.status = nota_status.clone();

            // Update start_date if provided
            if let Some(date) = parsed_start_date {
                nota.start_date = Some(date);
            }

            nota.updated_at = gtd::local_date_today();

            // Handle recurrence if moving to done status
            let mut next_occurrence_info: Option<String> = None;
            if nota_status == NotaStatus::done && nota.is_recurring() {
                // Calculate next occurrence date
                let from_date = nota.start_date.unwrap_or_else(gtd::local_date_today);
                if let Some(next_date) = nota.calculate_next_occurrence(from_date) {
                    // Create a new task for the next occurrence
                    let mut next_nota = nota.clone();
                    next_nota.id = format!("{}-{}", normalized_id, next_date.format("%Y%m%d"));
                    next_nota.start_date = Some(next_date);
                    next_nota.status = old_status.clone(); // Use the original status, not done
                    next_nota.created_at = gtd::local_date_today();
                    next_nota.updated_at = gtd::local_date_today();

                    // Check if next occurrence ID already exists
                    if !data.nota_map.contains_key(&next_nota.id) {
                        data.add(next_nota.clone());
                        next_occurrence_info = Some(format!(
                            "Next occurrence created: {} on {}",
                            next_nota.id, next_date
                        ));
                    }
                }
            }

            // Update the nota
            if data.update(&normalized_id, nota).is_none() {
                failures.push(format!("{}: failed to update", normalized_id));
                continue;
            }

            successes.push((normalized_id, old_status, next_occurrence_info));
        }

        drop(data);

        // Save data if any changes were made
        if !successes.is_empty() {
            let ids_str = if successes.len() == 1 {
                successes[0].0.clone()
            } else {
                format!("{} items", successes.len())
            };

            if let Err(e) =
                self.save_data_with_message(&format!("Change {} status to {}", ids_str, new_status))
            {
                bail_public!(_, "Failed to save: {}", e);
            }
        }

        // Build response message
        let mut response = String::new();

        if !successes.is_empty() {
            let action = if is_trash {
                "deleted"
            } else {
                "changed status"
            };
            response.push_str(&format!(
                "Successfully {} for {} item{}:\n",
                action,
                successes.len(),
                if successes.len() == 1 { "" } else { "s" }
            ));
            for (id, old_status, next_info) in &successes {
                if is_trash {
                    response.push_str(&format!("- {} (moved to trash)\n", id));
                } else {
                    response.push_str(&format!(
                        "- {}: {} → {}\n",
                        id,
                        format!("{:?}", old_status).to_lowercase(),
                        new_status
                    ));
                    if let Some(info) = next_info {
                        response.push_str(&format!("  {}\n", info));
                    }
                }
            }
        }

        if !failures.is_empty() {
            if !response.is_empty() {
                response.push('\n');
            }
            response.push_str(&format!(
                "Failed to change status for {} item{}:\n",
                failures.len(),
                if failures.len() == 1 { "" } else { "s" }
            ));
            for failure in &failures {
                response.push_str(&format!("- {}\n", failure));
            }
        }

        // If all failed, return error
        if successes.is_empty() {
            bail_public!(_, "{}", response.trim());
        }

        Ok(response.trim().to_string())
    }
}
