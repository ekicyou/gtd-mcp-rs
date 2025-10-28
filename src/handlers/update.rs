//! Update handler for GTD MCP server

use crate::GtdServerHandler;
use crate::gtd::{self, NotaStatus};
use crate::validation;
use chrono::NaiveDate;
use mcp_attr::{Result as McpResult, bail_public};

impl GtdServerHandler {
    /// **Clarify**: Update item details. Add context, notes, project links after capturing.
    /// **When**: After inbox capture, clarify what it is, why it matters, what's needed.
    /// **Tip**: Use ""(empty string) to clear optional fields.
    /// **Note**: Item ID cannot be changed - IDs are immutable. To "rename", create new item and delete old one.
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_update(
        &self,
        id: String,
        title: Option<String>,
        status: Option<String>,
        project: Option<String>,
        context: Option<String>,
        notes: Option<String>,
        start_date: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Find existing nota
        let mut nota = match data.find_by_id(&id) {
            Some(n) => n,
            None => {
                drop(data);
                bail_public!(
                    _,
                    "Item not found: Item '{}' does not exist. Use list() to see available items.",
                    id
                );
            }
        };

        // Update fields if provided
        if let Some(new_title) = title {
            nota.title = new_title;
        }

        if let Some(new_status_str) = status {
            let new_status: NotaStatus = match new_status_str.parse() {
                Ok(s) => s,
                Err(_) => {
                    drop(data);
                    bail_public!(
                        _,
                        "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, reference, trash, project, context",
                        new_status_str
                    );
                }
            };
            nota.status = new_status;
        }

        // Handle optional reference fields (empty string means clear)
        if let Some(proj) = project {
            nota.project = if proj.is_empty() {
                None
            } else {
                // Validate project exists
                if data.find_project_by_id(&proj).is_none() {
                    let error_msg = validation::format_invalid_project_error(&proj, &data);
                    drop(data);
                    bail_public!(_, "{}", error_msg);
                }
                Some(proj)
            };
        }

        if let Some(ctx) = context {
            nota.context = if ctx.is_empty() {
                None
            } else {
                // Validate context exists
                if data.find_context_by_name(&ctx).is_none() {
                    let error_msg = validation::format_invalid_context_error(&ctx, &data);
                    drop(data);
                    bail_public!(_, "{}", error_msg);
                }
                Some(ctx)
            };
        }

        if let Some(n) = notes {
            nota.notes = if n.is_empty() { None } else { Some(n) };
        }

        if let Some(date_str) = start_date {
            nota.start_date = if date_str.is_empty() {
                None
            } else {
                match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
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
            };
        }

        // Validate calendar status has start_date
        if nota.status == NotaStatus::calendar && nota.start_date.is_none() {
            drop(data);
            bail_public!(
                _,
                "Calendar status validation failed: status=calendar requires start_date. Please provide a start_date or change to a different status."
            );
        }

        nota.updated_at = gtd::local_date_today();

        // Update the nota
        if data.update(&id, nota).is_none() {
            drop(data);
            bail_public!(_, "Failed to update item '{}'", id);
        }
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Update item {}", id)) {
            bail_public!(_, "Failed to save: {}", e);
        }

        Ok(format!("Item {} updated successfully", id))
    }
}
