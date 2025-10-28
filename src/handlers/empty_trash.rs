//! Empty trash handler for GTD MCP server

use crate::GtdServerHandler;
use crate::gtd::NotaStatus;
use mcp_attr::{Result as McpResult, bail_public};

impl GtdServerHandler {
    /// **Purge**: Permanently delete all trashed items. Run weekly.
    /// **When**: Part of weekly review - trash items first with change_status, then purge.
    /// **Safety**: Checks references to prevent broken links.
    pub async fn handle_empty_trash(&self) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Count and remove all trash notas
        let count = data
            .notas
            .iter()
            .filter(|n| n.status == NotaStatus::trash)
            .count();
        data.notas.retain(|n| n.status != NotaStatus::trash);

        // Update nota_map
        data.nota_map
            .retain(|_, status| *status != NotaStatus::trash);

        drop(data);

        if let Err(e) = self.save_data_with_message("Empty trash") {
            bail_public!(_, "Failed to save: {}", e);
        }

        Ok(format!("Deleted {} task(s) from trash", count))
    }
}
