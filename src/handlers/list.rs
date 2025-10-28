//! List handler for GTD MCP server

use crate::GtdServerHandler;
use crate::formatting;
use crate::validation;
use mcp_attr::Result as McpResult;

impl GtdServerHandler {
    /// Handles list/filter operations - applies filters and formats results for display.
    pub async fn handle_list(
        &self,
        status: Option<String>,
        date: Option<String>,
        exclude_notes: Option<bool>,
        keyword: Option<String>,
        project: Option<String>,
        context: Option<String>,
    ) -> McpResult<String> {
        // Parse and validate status filter
        let status_filter = if let Some(ref status_str) = status {
            Some(validation::parse_status_filter(status_str)?)
        } else {
            None
        };

        // Parse and validate date filter
        let date_filter = if let Some(ref date_str) = date {
            Some(validation::parse_date_filter(date_str)?)
        } else {
            None
        };

        // Get initial list of notas filtered by status
        let data = self.data.lock().unwrap();
        let mut notas = data.list_all(status_filter);
        drop(data);

        // Apply additional filters in sequence
        if let Some(filter_date) = date_filter {
            formatting::apply_date_filter(&mut notas, filter_date);
        }

        if let Some(ref keyword_filter) = keyword {
            formatting::apply_keyword_filter(&mut notas, keyword_filter);
        }

        if let Some(ref project_filter) = project {
            formatting::apply_project_filter(&mut notas, project_filter);
        }

        if let Some(ref context_filter) = context {
            formatting::apply_context_filter(&mut notas, context_filter);
        }

        // Format and return results
        let exclude_notes_flag = exclude_notes.unwrap_or(false);
        Ok(formatting::format_notas(notas, exclude_notes_flag))
    }
}
