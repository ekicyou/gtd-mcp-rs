//! GTD MCP Server Library
//!
//! This library provides a Model Context Protocol (MCP) server for GTD (Getting Things Done)
//! task management. It implements the GTD methodology with support for tasks, projects,
//! and contexts, with automatic Git-based version control.
//!
//! # Architecture
//!
//! The library follows a 3-layer architecture:
//! - **MCP Layer**: `GtdServerHandler` - Handles MCP protocol communication
//! - **Domain Layer**: `gtd` module - Core GTD data models and business logic
//! - **Persistence Layer**: `storage` module - File-based TOML storage with Git sync
//!
//! # Example
//!
//! ```no_run
//! use gtd_mcp::GtdServerHandler;
//! use anyhow::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let handler = GtdServerHandler::new("gtd.toml", false)?;
//!     // Use handler with MCP server...
//!     Ok(())
//! }
//! ```

mod git_ops;
mod gtd;
mod migration;
mod storage;

use anyhow::Result;
use chrono::NaiveDate;

use mcp_attr::server::{McpServer, mcp_server};
use mcp_attr::{Result as McpResult, bail};
use std::sync::Mutex;

// Re-export commonly used types
pub use gtd::{Context, GtdData, NotaStatus, Project, ProjectStatus, Task};
pub use storage::Storage;

/// MCP Server handler for GTD task management
///
/// Provides an MCP interface to GTD functionality including task management,
/// project tracking, and context organization. All changes are automatically
/// persisted to a TOML file and optionally synchronized with Git.
pub struct GtdServerHandler {
    pub(crate) data: Mutex<GtdData>,
    pub(crate) storage: Storage,
}

impl GtdServerHandler {
    /// Create a new GTD server handler
    ///
    /// # Arguments
    /// * `storage_path` - Path to the GTD data file (TOML format)
    /// * `sync_git` - Enable automatic Git synchronization
    ///
    /// # Returns
    /// Result containing the handler or an error
    ///
    /// # Example
    /// ```no_run
    /// # use gtd_mcp::GtdServerHandler;
    /// # use anyhow::Result;
    /// # fn main() -> Result<()> {
    /// let handler = GtdServerHandler::new("gtd.toml", false)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(storage_path: &str, sync_git: bool) -> Result<Self> {
        let storage = Storage::new(storage_path, sync_git);
        let data = Mutex::new(storage.load()?);
        Ok(Self { data, storage })
    }

    /// Save GTD data with a default message
    #[allow(dead_code)]
    fn save_data(&self) -> Result<()> {
        let data = self.data.lock().unwrap();
        self.storage.save(&data)?;
        Ok(())
    }

    /// Save GTD data with a custom commit message
    fn save_data_with_message(&self, message: &str) -> Result<()> {
        let data = self.data.lock().unwrap();
        self.storage.save_with_message(&data, message)?;
        Ok(())
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
    /// # use gtd_mcp::GtdServerHandler;
    /// // normalize_task_id("task-1") -> "task-1"
    /// // normalize_task_id("meeting-prep") -> "meeting-prep"
    /// ```
    fn normalize_task_id(task_id: &str) -> String {
        task_id.trim().to_string()
    }
}

impl Drop for GtdServerHandler {
    fn drop(&mut self) {
        // Push to git on shutdown if sync is enabled
        if let Err(e) = self.storage.shutdown() {
            eprintln!("Warning: Shutdown git sync failed: {}", e);
        }
    }
}

/// GTD (Getting Things Done) task management server implementing David Allen's productivity methodology.
///
/// This server helps you capture, organize, and track tasks through a proven workflow system.
/// GTD organizes tasks into different status categories (inbox, next_action, waiting_for, someday, later, calendar, done, trash)
/// and supports projects (multi-step endeavors) and contexts (environments/tools like @office, @home).
///
/// Key concepts:
/// - **inbox**: Unprocessed items (start here)
/// - **next_action**: Ready-to-execute tasks (focus here)
/// - **waiting_for**: Blocked tasks awaiting someone/something
/// - **someday**: Potential future actions
/// - **later**: Deferred but planned tasks
/// - **calendar**: Date-specific tasks
/// - **done**: Completed tasks
/// - **trash**: Deleted tasks
///
/// Task IDs use format: #1, #2, #3
/// Project IDs: Use meaningful abbreviations (e.g., "website-redesign", "q1-budget")
#[mcp_server]
impl McpServer for GtdServerHandler {
    /// **GTD Purge**: Permanently delete trashed items. Run weekly. Checks references to prevent broken links.
    /// **Workflow**: Trash items via change_status first, then empty_trash to delete permanently.
    #[tool]
    async fn empty_trash(&self) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        let count = data.trash.len();
        data.trash.clear();

        drop(data);

        if let Err(e) = self.save_data_with_message("Empty trash") {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Deleted {} task(s) from trash", count))
    }

    /// **GTD Capture (Inbox)**: Quickly capture anything needing attention. First step - all items start here.
    /// **Status**: Use "inbox" for tasks, "project" for projects, "context" for contexts.
    /// **Workflow**: 1) inbox everything → 2) list to review → 3) update/change_status to organize.
    #[tool]
    async fn inbox(
        &self,
        /// ID: any string (e.g., "call-john", "web-redesign")
        id: String,
        /// Title: brief description
        title: String,
        /// Status: inbox/next_action/waiting_for/later/calendar/someday/done/project/context/trash
        status: String,
        /// Project: parent project ID (optional)
        project: Option<String>,
        /// Context: where applies (e.g., "@home", "@office") (optional)
        context: Option<String>,
        /// Notes: Markdown details (optional)
        notes: Option<String>,
        /// Start date: YYYY-MM-DD, required for calendar status (optional)
        start_date: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check for duplicate ID across all notas
        if data.task_map.contains_key(&id)
            || data.projects.contains_key(&id)
            || data.contexts.contains_key(&id)
        {
            drop(data);
            bail!("Nota ID '{}' already exists. Please use a unique ID.", id);
        }

        // Parse status
        let nota_status: NotaStatus = match status.parse() {
            Ok(s) => s,
            Err(_) => {
                drop(data);
                bail!(
                    "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context",
                    status
                );
            }
        };

        // Validate calendar status has start_date
        if nota_status == NotaStatus::calendar && start_date.is_none() {
            drop(data);
            bail!("Calendar status requires start_date parameter");
        }

        // Parse start_date if provided
        let parsed_start_date = if let Some(ref date_str) = start_date {
            match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => {
                    drop(data);
                    bail!(
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
            drop(data);
            bail!("Project '{}' does not exist", proj_id);
        }

        // Validate context reference if provided
        if let Some(ref ctx_name) = context
            && data.find_context_by_name(ctx_name).is_none()
        {
            drop(data);
            bail!("Context '{}' does not exist", ctx_name);
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
        };

        data.add(nota);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Add nota {}", id)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!(
            "Nota created with ID: {} (type: {})",
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

    /// **GTD Review**: List/view all notas, filter by status.
    /// **Workflow**: Start here - review regularly (daily/weekly). Filter status="inbox" for items needing processing.
    /// **Use**: No filter=all; "inbox"=uncaptured; "next_action"=ready to do; "project"=all projects.
    #[tool]
    async fn list(
        &self,
        /// Status filter: inbox/next_action/waiting_for/later/calendar/someday/done/project/context/trash. Empty=all.
        status: Option<String>,
    ) -> McpResult<String> {
        let data = self.data.lock().unwrap();

        // Parse status filter if provided
        let status_filter = if let Some(ref status_str) = status {
            match status_str.parse::<NotaStatus>() {
                Ok(s) => Some(s),
                Err(_) => {
                    drop(data);
                    bail!(
                        "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context",
                        status_str
                    );
                }
            }
        } else {
            None
        };

        let notas = data.list_all(status_filter);
        drop(data);

        if notas.is_empty() {
            return Ok("No notas found".to_string());
        }

        let mut result = format!("Found {} nota(s):\n\n", notas.len());
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
            if let Some(ref n) = nota.notes {
                result.push_str(&format!("  Notes: {}\n", n));
            }
            if let Some(ref date) = nota.start_date {
                result.push_str(&format!("  Start date: {}\n", date));
            }
        }

        Ok(result)
    }

    /// **GTD Clarify/Organize**: Update nota details. Add context, clarify next steps, link to project.
    /// **Workflow**: After inbox, clarify what it is, add notes/context/project links.
    /// **Tip**: Use empty string "" to clear optional fields.
    #[allow(clippy::too_many_arguments)]
    #[tool]
    async fn update(
        &self,
        /// ID of nota to update
        id: String,
        /// New title (optional)
        title: Option<String>,
        /// New status - changes type if project/context (optional)
        status: Option<String>,
        /// Project link, ""=clear (optional)
        project: Option<String>,
        /// Context tag, ""=clear (optional)
        context: Option<String>,
        /// Notes in Markdown, ""=clear (optional)
        notes: Option<String>,
        /// Start date YYYY-MM-DD, ""=clear (optional)
        start_date: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Find existing nota
        let mut nota = match data.find_by_id(&id) {
            Some(n) => n,
            None => {
                drop(data);
                bail!("Nota '{}' not found", id);
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
                    bail!(
                        "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context",
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
                    drop(data);
                    bail!("Project '{}' does not exist", proj);
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
                    drop(data);
                    bail!("Context '{}' does not exist", ctx);
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
                        bail!(
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
            bail!("Calendar status requires start_date");
        }

        nota.updated_at = gtd::local_date_today();

        // Update the nota
        if data.update(&id, nota).is_none() {
            drop(data);
            bail!("Failed to update nota '{}'", id);
        }
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Update nota {}", id)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Nota {} updated successfully", id))
    }

    /// **GTD Do/Organize**: Move nota through workflow stages.
    /// **Workflow**: inbox→next_action (ready to do), →waiting_for (blocked), →done (complete), →trash (discard).
    /// **Tip**: status="project"/"context" transforms type. Use change_status before empty_trash to delete.
    #[tool]
    async fn change_status(
        &self,
        /// Nota ID
        id: String,
        /// New status: inbox/next_action/waiting_for/later/calendar/someday/done/project/context/trash
        new_status: String,
        /// Start date YYYY-MM-DD (required for calendar)
        start_date: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Parse new status
        let nota_status: NotaStatus = match new_status.parse() {
            Ok(s) => s,
            Err(_) => {
                drop(data);
                bail!(
                    "Invalid status '{}'. Valid statuses: inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context",
                    new_status
                );
            }
        };

        // Validate calendar status has start_date
        if nota_status == NotaStatus::calendar && start_date.is_none() {
            drop(data);
            bail!("Calendar status requires start_date parameter");
        }

        // Find existing nota
        let mut nota = match data.find_by_id(&id) {
            Some(n) => n,
            None => {
                drop(data);
                bail!("Nota '{}' not found", id);
            }
        };

        // Update status
        nota.status = nota_status;

        // Update start_date if provided for calendar
        if let Some(date_str) = start_date {
            nota.start_date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => {
                    drop(data);
                    bail!(
                        "Invalid date format '{}'. Use YYYY-MM-DD (e.g., '2025-03-15')",
                        date_str
                    );
                }
            };
        }

        nota.updated_at = gtd::local_date_today();

        // Update the nota (this will automatically move it to the correct container)
        if data.update(&id, nota).is_none() {
            drop(data);
            bail!("Failed to update nota '{}'", id);
        }
        drop(data);

        if let Err(e) =
            self.save_data_with_message(&format!("Change nota {} status to {}", id, new_status))
        {
            bail!("Failed to save: {}", e);
        }

        Ok(format!(
            "Nota {} status changed to {} successfully",
            id, new_status
        ))
    }
}
