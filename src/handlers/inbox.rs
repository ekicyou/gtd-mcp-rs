//! Inbox handler for GTD MCP server

use crate::GtdServerHandler;
use crate::gtd::{self, NotaStatus};
use crate::validation;
use chrono::NaiveDate;
use mcp_attr::{Result as McpResult, bail_public};

impl GtdServerHandler {
    /// **Capture**: Quickly capture anything needing attention. First GTD step - all items start here.
    /// **When**: Something crosses your mind? Capture immediately without thinking.
    /// **Next**: Use list(status="inbox") to review, then update/change_status to organize.
    ///
    /// **ID Naming Guidelines**:
    /// - Use kebab-case (lowercase with hyphens): "fix-io-button", "review-q3-sales"
    /// - Start with verb when possible: "update-", "fix-", "create-", "review-"
    /// - Keep concise but meaningful (3-5 words max)
    /// - Use project prefix for clarity: "eci-fix-button", "fft-level-cloud"
    /// - IDs are immutable - choose carefully as they cannot be changed later
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_inbox(
        &self,
        id: String,
        title: String,
        status: String,
        project: Option<String>,
        context: Option<String>,
        notes: Option<String>,
        start_date: Option<String>,
        recurrence: Option<String>,
        recurrence_config: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check for duplicate ID across all notas
        if data.nota_map.contains_key(&id) {
            let existing_status = data.nota_map[&id].clone();
            drop(data);
            bail_public!(
                _,
                "Duplicate ID error: ID '{}' already exists (status: {:?}). Each item must have a unique ID. Please choose a different ID.",
                id,
                existing_status
            );
        }

        // Parse status
        let nota_status: NotaStatus = match status.parse() {
            Ok(s) => s,
            Err(_) => {
                drop(data);
                bail_public!(
                    _,
                    "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, reference, trash, project, context",
                    status
                );
            }
        };

        // Validate calendar status has start_date
        if nota_status == NotaStatus::calendar && start_date.is_none() {
            drop(data);
            bail_public!(
                _,
                "Calendar status validation failed: status=calendar requires start_date parameter. Please provide a date in YYYY-MM-DD format."
            );
        }

        // Parse start_date if provided
        let parsed_start_date = if let Some(ref date_str) = start_date {
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

        // Validate project reference if provided
        if let Some(ref proj_id) = project
            && data.find_project_by_id(proj_id).is_none()
        {
            let error_msg = validation::format_invalid_project_error(proj_id, &data);
            drop(data);
            bail_public!(_, "{}", error_msg);
        }

        // Validate context reference if provided
        if let Some(ref ctx_name) = context
            && data.find_context_by_name(ctx_name).is_none()
        {
            let error_msg = validation::format_invalid_context_error(ctx_name, &data);
            drop(data);
            bail_public!(_, "{}", error_msg);
        }

        // Parse recurrence pattern if provided
        let recurrence_pattern = if let Some(ref recurrence_str) = recurrence {
            match recurrence_str.as_str() {
                "daily" => Some(gtd::RecurrencePattern::daily),
                "weekly" => Some(gtd::RecurrencePattern::weekly),
                "monthly" => Some(gtd::RecurrencePattern::monthly),
                "yearly" => Some(gtd::RecurrencePattern::yearly),
                _ => {
                    drop(data);
                    bail_public!(
                        _,
                        "Invalid recurrence pattern '{}'. Valid patterns: daily, weekly, monthly, yearly",
                        recurrence_str
                    );
                }
            }
        } else {
            None
        };

        // Validate recurrence configuration if recurrence pattern is provided
        if let Some(ref pattern) = recurrence_pattern
            && recurrence_config.is_none()
        {
            // Only weekly, monthly, and yearly require config
            match pattern {
                gtd::RecurrencePattern::weekly => {
                    drop(data);
                    bail_public!(
                        _,
                        "Recurrence pattern 'weekly' requires recurrence_config with weekday names (e.g., \"Monday,Wednesday,Friday\")"
                    );
                }
                gtd::RecurrencePattern::monthly => {
                    drop(data);
                    bail_public!(
                        _,
                        "Recurrence pattern 'monthly' requires recurrence_config with day numbers (e.g., \"1,15,25\")"
                    );
                }
                gtd::RecurrencePattern::yearly => {
                    drop(data);
                    bail_public!(
                        _,
                        "Recurrence pattern 'yearly' requires recurrence_config with month-day pairs (e.g., \"1-1,12-25\")"
                    );
                }
                gtd::RecurrencePattern::daily => {} // Daily doesn't need config
            }
        }

        let today = gtd::local_date_today();
        let nota = gtd::Nota {
            id: id.clone(),
            title: title.clone(),
            status: nota_status.clone(),
            project,
            context,
            notes,
            start_date: parsed_start_date,
            created_at: today,
            updated_at: today,
            recurrence_pattern,
            recurrence_config,
        };

        data.add(nota);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Add item {}", id)) {
            bail_public!(_, "Failed to save: {}", e);
        }

        Ok(format!(
            "Item created with ID: {} (type: {})",
            id,
            if nota_status == NotaStatus::context {
                "context"
            } else if nota_status == NotaStatus::project {
                "project"
            } else {
                "task"
            }
        ))
    }
}
