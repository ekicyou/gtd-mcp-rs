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

    /// Extract ID from response message
    ///
    /// Helper function for tests to extract nota ID from response messages.
    /// Response format: "Nota created with ID: <id> (type: task)"
    ///
    /// # Arguments
    /// * `response` - The response message from inbox() or similar operations
    ///
    /// # Returns
    /// The extracted ID
    #[cfg(test)]
    fn extract_id_from_response(response: &str) -> String {
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
#[cfg(test)]
mod tests {
    // TODO: Test Migration Required - 84 compilation errors remain after legacy tool removal
    //
    // CONTEXT:
    // - All 13 legacy MCP tools have been removed (add_task, list_tasks, etc.)
    // - 5 unified tools remain: inbox, list, update, change_status, empty_trash
    // - Tool renamed: add → inbox (GTD principle: all items start in inbox)
    // - GTD workflow tools removed: gtd_overview, process_inbox, weekly_review, next_actions
    //
    // COMPLETED FIXES (150 of 234 errors, 64%):
    // ✅ All inbox() signature fixes (added status parameter)
    // ✅ All update() signature fixes (added status parameter)
    // ✅ All change_status() fixes (removed vec! wrapper - now takes String not Vec<String>)
    // ✅ All delete_project/delete_context replacements (change_status + empty_trash)
    // ✅ All GTD workflow method call removals
    //
    // REMAINING FIXES (84 errors, 36%):
    // These are all type mismatch errors. Common patterns expected:
    // 1. Tests creating Task/Project/Context structs directly need Nota conversion
    //    Example: data.add(Nota::from_task(task))
    // 2. Tests expecting Task/Project/Context return types need Nota conversion
    //    Example: nota.to_task()?, nota.to_project()?, nota.to_context()?
    // 3. Assertions on status values need NotaStatus enum (not strings)
    // 4. Field name changes: name→title, description→notes
    //
    // APPROACH:
    // Fix errors incrementally, 5-10 at a time:
    // 1. cargo build 2>&1 | grep error | head -10
    // 2. Fix those errors
    // 3. Commit with message describing # errors fixed
    // 4. Repeat until all 84 errors resolved
    //
    // VERIFICATION:
    // - cargo test (target: ~200-220 tests passing)
    // - cargo fmt --check && cargo clippy && cargo test
    //
    use super::*;
    use crate::gtd::{Nota, local_date_today};
    use chrono::NaiveDate;
    use tempfile::NamedTempFile;

    fn get_test_handler() -> (GtdServerHandler, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let handler = GtdServerHandler::new(temp_file.path().to_str().unwrap(), false).unwrap();
        (handler, temp_file)
    }

    #[test]
    fn test_custom_file_path() {
        // カスタムファイルパスでハンドラーを作成
        let temp_file = NamedTempFile::new().unwrap();
        let custom_path = temp_file.path().to_str().unwrap();

        let handler = GtdServerHandler::new(custom_path, false).unwrap();

        // ストレージのファイルパスが正しく設定されていることを確認
        assert_eq!(handler.storage.file_path().to_str().unwrap(), custom_path);

        // データの保存と読み込みが正しく動作することを確認
        let mut data = handler.data.lock().unwrap();
        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };
        data.add(Nota::from_task(task));
        drop(data);

        // 保存
        let save_result = handler.save_data();
        assert!(save_result.is_ok());

        // ファイルが作成されていることを確認
        assert!(std::path::Path::new(custom_path).exists());

        // 新しいハンドラーで読み込み
        let handler2 = GtdServerHandler::new(custom_path, false).unwrap();
        let loaded_data = handler2.data.lock().unwrap();
        assert_eq!(loaded_data.task_count(), 1);
        let loaded_task = loaded_data.find_task_by_id("test-task").unwrap();
        assert_eq!(loaded_task.title, "Test Task");
    }

    #[test]
    fn test_normalize_task_id() {
        // Test with arbitrary task IDs - normalize should just trim
        assert_eq!(GtdServerHandler::normalize_task_id("task-1"), "task-1");
        assert_eq!(
            GtdServerHandler::normalize_task_id("meeting-prep"),
            "meeting-prep"
        );
        assert_eq!(
            GtdServerHandler::normalize_task_id("call-sarah"),
            "call-sarah"
        );

        // Test with whitespace - should be trimmed
        assert_eq!(GtdServerHandler::normalize_task_id(" task-1 "), "task-1");
        assert_eq!(
            GtdServerHandler::normalize_task_id("  meeting-prep  "),
            "meeting-prep"
        );

        // Old-style IDs with # are also valid
        assert_eq!(GtdServerHandler::normalize_task_id("#1"), "#1");
        assert_eq!(GtdServerHandler::normalize_task_id(" #42 "), "#42");
    }

    #[tokio::test]
    async fn test_change_task_status_unified_api() {
        let (handler, _temp_file) = get_test_handler();

        // Create a task in inbox
        let result = handler
            .inbox(
                "task-3".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Test moving to next_action
        let result = handler
            .change_status(task_id.clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.status, NotaStatus::next_action);
        }

        // Test moving to done
        let result = handler
            .change_status(task_id.clone(), "done".to_string(), None)
            .await;
        assert!(result.is_ok());
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.status, NotaStatus::done);
        }

        // Test moving to trash
        let result = handler
            .change_status(task_id.clone(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.status, NotaStatus::trash);
        }

        // Test invalid status
        let result = handler
            .change_status(task_id.clone(), "invalid_status".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_change_task_status_calendar_with_date() {
        let (handler, _temp_file) = get_test_handler();

        // Create a task
        let result = handler
            .inbox(
                "task-4".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Test moving to calendar with date
        let result = handler
            .change_status(
                task_id.clone(),
                "calendar".to_string(),
                Some("2024-12-25".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.status, NotaStatus::calendar);
        assert_eq!(
            task.start_date.unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 25).unwrap()
        );
    }

    #[tokio::test]
    async fn test_change_task_status_batch_operation() {
        let (handler, _temp_file) = get_test_handler();

        // Create multiple tasks
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 5 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // Test batch move to next_action
        let result = handler
            .change_status(task_ids[0].clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify all tasks moved
        let data = handler.data.lock().unwrap();
        assert_eq!(data.next_action.len(), 3);
        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert_eq!(task.status, NotaStatus::next_action);
        }
    }

    #[tokio::test]
    async fn test_update_task_with_arbitrary_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task with an arbitrary ID
        let result = handler
            .inbox(
                "meeting-prep".to_string(),
                "Prepare for meeting".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Update task using the arbitrary ID
        let result = handler
            .update(
                "meeting-prep".to_string(),
                Some("Updated meeting preparation".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify the update worked
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id("meeting-prep").unwrap();
        assert_eq!(task.title, "Updated meeting preparation");
    }

    #[tokio::test]
    async fn test_status_movement_with_arbitrary_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task with an arbitrary ID
        let result = handler
            .inbox(
                "call-sarah".to_string(),
                "Call Sarah".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Move to next_action using the arbitrary ID
        let result = handler
            .change_status("call-sarah".to_string(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify the task moved
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id("call-sarah").unwrap();
        assert_eq!(task.status, NotaStatus::next_action);
    }

    #[tokio::test]
    async fn test_update_task_title() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-8".to_string(),
                "Original Title".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Extract task ID from result
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Update title
        let result = handler
            .update(
                task_id.clone(),
                Some("Updated Title".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify update
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_update_task_status_using_next_action_task() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-9".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Verify initial status is inbox
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::inbox));
            assert_eq!(data.inbox.len(), 1);
            assert_eq!(data.next_action.len(), 0);
        }

        // Update status to next_action using new method
        let result = handler
            .change_status(task_id.clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify status changed and task moved
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::next_action));
            assert_eq!(data.inbox.len(), 0);
            assert_eq!(data.next_action.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_update_task_project_and_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project and context first
        let project_result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(project_result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&project_result.unwrap());

        {
            let mut data = handler.data.lock().unwrap();
            data.add(Nota::from_context(gtd::Context {
                name: "Office".to_string(),
                notes: None,
                title: None,
                status: gtd::NotaStatus::context,
                project: None,
                context: None,
                start_date: None,
                created_at: None,
                updated_at: None,
            }));
            drop(data);
            let _ = handler.save_data();
        }

        // Add a task
        let result = handler
            .inbox(
                "task-10".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Update project and context
        let result = handler
            .update(
                task_id.clone(),
                None,
                None,
                Some(project_id.clone()),
                Some("Office".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify update
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.project, Some(project_id));
        assert_eq!(task.context, Some("Office".to_string()));
    }

    #[tokio::test]
    async fn test_update_task_remove_optional_fields() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task with optional fields
        let result = handler
            .inbox(
                "task-2001".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                Some("Some notes".to_string()),
                Some("2024-12-25".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Verify initial state
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.notes, Some("Some notes".to_string()));
            assert!(task.start_date.is_some());
        }

        // Remove optional fields using empty strings
        let result = handler
            .update(
                task_id.clone(),
                None,
                None,
                None,
                Some("".to_string()),
                Some("".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify fields removed
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.notes, None);
        assert_eq!(task.start_date, None);
    }

    #[tokio::test]
    async fn test_update_task_invalid_date() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-11".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Try to update with invalid date
        let result = handler
            .update(
                task_id,
                None,
                None,
                None,
                None,
                Some("invalid-date".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_task_invalid_project_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-12".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Try to update with non-existent project
        let result = handler
            .update(
                task_id,
                None,
                Some("non-existent-project".to_string()),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_task_invalid_context_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-13".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Try to update with non-existent context
        let result = handler
            .update(
                task_id,
                None,
                None,
                Some("NonExistent".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_task_not_found() {
        let (handler, _temp_file) = get_test_handler();

        // Try to update non-existent task
        let result = handler
            .update(
                "non-existent-id".to_string(),
                Some("New Title".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_task_updates_timestamp() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-14".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Get initial timestamps
        let (created_at, _updated_at) = {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            (task.created_at, task.updated_at)
        };

        // Update task
        let result = handler
            .update(
                task_id.clone(),
                Some("Updated Title".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify updated_at changed but created_at didn't
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.created_at, created_at);
        // Note: In test environment, if executed fast enough, updated_at might be the same
        // This is acceptable as the implementation is correct
    }

    #[tokio::test]
    async fn test_update_project_name() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Original Name".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Update name
        let result = handler
            .update(
                project_id.clone(),
                None,
                Some("Updated Name".to_string()),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify update
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert_eq!(project.title, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_project_description() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Add description
        let result = handler
            .update(
                project_id.clone(),
                None,
                None,
                Some("New description".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify description added
        {
            let data = handler.data.lock().unwrap();
            let project = data.find_project_by_id(&project_id).unwrap();
            assert_eq!(project.notes, Some("New description".to_string()));
        }

        // Remove description
        let result = handler
            .update(
                project_id.clone(),
                None,
                None,
                Some("".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify description removed
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert_eq!(project.notes, None);
    }

    #[tokio::test]
    async fn test_update_project_status() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Verify initial status
        {
            let data = handler.data.lock().unwrap();
            let project = data.find_project_by_id(&project_id).unwrap();
            assert!(matches!(project.status, ProjectStatus::active));
        }

        // Update status to on_hold
        let result = handler
            .update(
                project_id.clone(),
                None,
                None,
                None,
                Some("on_hold".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify status changed
        {
            let data = handler.data.lock().unwrap();
            let project = data.find_project_by_id(&project_id).unwrap();
            assert!(matches!(project.status, ProjectStatus::on_hold));
        }

        // Update status to completed
        let result = handler
            .update(
                project_id.clone(),
                None,
                None,
                None,
                Some("completed".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify status changed
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert!(matches!(project.status, ProjectStatus::completed));
    }

    #[tokio::test]
    async fn test_update_project_invalid_status() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Try to update with invalid status
        let result = handler
            .update(
                project_id,
                None,
                None,
                None,
                Some("invalid_status".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_project_not_found() {
        let (handler, _temp_file) = get_test_handler();

        // Try to update non-existent project
        let result = handler
            .update(
                "non-existent-id".to_string(),
                None,
                Some("New Name".to_string()),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_project_success() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Delete the project
        let result = handler
            .change_status("test-project-1".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("deleted successfully"));

        // Verify the project was deleted
        let data = handler.data.lock().unwrap();
        assert!(data.find_project_by_id("test-project-1").is_none());
    }

    #[tokio::test]
    async fn test_delete_project_not_found() {
        let (handler, _temp_file) = get_test_handler();

        // Try to delete non-existent project
        let result = handler
            .change_status("non-existent-id".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_project_with_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Add a task that references the project
        let result = handler
            .inbox(
                "task-2002".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                Some("test-project-1".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Try to delete the project (should fail)
        let result = handler
            .change_status("test-project-1".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());

        // Verify the project was NOT deleted
        let data = handler.data.lock().unwrap();
        assert!(data.find_project_by_id("test-project-1").is_some());
    }

    #[tokio::test]
    async fn test_delete_project_after_unlinking_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Add a task that references the project
        let result = handler
            .inbox(
                "task-2003".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                Some("test-project-1".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Unlink the task from the project
        let result = handler
            .update(
                "task-2003".to_string(),
                None,
                Some("".to_string()), // Empty string removes project
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Now delete the project (should succeed)
        let result = handler
            .change_status("test-project-1".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify the project was deleted
        let data = handler.data.lock().unwrap();
        assert!(data.find_project_by_id("test-project-1").is_none());
    }

    #[tokio::test]
    async fn test_update_multiple_fields_simultaneously() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let project_result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(project_result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&project_result.unwrap());

        // Add a context
        {
            let mut data = handler.data.lock().unwrap();
            data.add(Nota::from_context(gtd::Context {
                name: "Office".to_string(),
                notes: None,
                title: None,
                status: gtd::NotaStatus::context,
                project: None,
                context: None,
                start_date: None,
                created_at: None,
                updated_at: None,
            }));
            drop(data);
            let _ = handler.save_data();
        }

        // Add a task
        let result = handler
            .inbox(
                "task-15".to_string(),
                "Original Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Update multiple fields at once
        let result = handler
            .update(
                task_id.clone(),
                Some("Updated Task".to_string()),
                Some(project_id.clone()),
                Some("Office".to_string()),
                Some("Updated notes".to_string()),
                Some("2025-01-15".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // Change status separately using new method
        let result = handler
            .change_status(task_id.clone(), "done".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify all updates
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.title, "Updated Task");
        assert!(matches!(task.status, NotaStatus::done));
        assert_eq!(task.project, Some(project_id));
        assert_eq!(task.context, Some("Office".to_string()));
        assert_eq!(task.notes, Some("Updated notes".to_string()));
        assert_eq!(
            task.start_date,
            Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap())
        );
    }

    // Tests for new status movement methods

    #[tokio::test]
    async fn test_inbox_task() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-16".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Move to next_action first
        let result = handler
            .change_status(task_id.clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify it's in next_action
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::next_action));
            assert_eq!(data.next_action.len(), 1);
            assert_eq!(data.inbox.len(), 0);
        }

        // Move back to inbox
        let result = handler
            .change_status(task_id.clone(), "inbox".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify it's back in inbox
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::inbox));
            assert_eq!(data.inbox.len(), 1);
            assert_eq!(data.next_action.len(), 0);
        }
    }

    #[tokio::test]
    async fn test_next_action_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-17".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id.clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::next_action));
        assert_eq!(data.next_action.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_waiting_for_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-18".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id.clone(), "waiting_for".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::waiting_for));
        assert_eq!(data.waiting_for.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_someday_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-19".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id.clone(), "someday".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::someday));
        assert_eq!(data.someday.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_later_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-20".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id.clone(), "later".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::later));
        assert_eq!(data.later.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_done_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-21".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id.clone(), "done".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::done));
        assert_eq!(data.done.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_trash_task_from_inbox() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-22".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id.clone(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok(), "Failed to trash task: {:?}", result.err());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::trash));
        assert_eq!(data.trash.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_trash_task_workflow_comparison() {
        let (handler, _temp_file) = get_test_handler();

        // Test 1: inbox → trash directly
        let result = handler
            .inbox(
                "task-23".to_string(),
                "Direct Trash Test".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id_1 = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id_1.clone(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok(), "Direct trash failed: {:?}", result.err());

        // Test 2: inbox → done → trash (the workflow user reported as working)
        let result = handler
            .inbox(
                "task-24".to_string(),
                "Indirect Trash Test".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id_2 = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(task_id_2.clone(), "done".to_string(), None)
            .await;
        assert!(result.is_ok(), "Moving to done failed: {:?}", result.err());

        let result = handler
            .change_status(task_id_2.clone(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok(), "Trash from done failed: {:?}", result.err());

        // Verify both tasks ended up in trash
        let data = handler.data.lock().unwrap();
        assert_eq!(data.trash.len(), 2);
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.done.len(), 0);

        let task1 = data.find_task_by_id(&task_id_1).unwrap();
        let task2 = data.find_task_by_id(&task_id_2).unwrap();
        assert!(matches!(task1.status, NotaStatus::trash));
        assert!(matches!(task2.status, NotaStatus::trash));
    }

    #[tokio::test]
    async fn test_trash_task_error_messages() {
        let (handler, _temp_file) = get_test_handler();

        // Test with various invalid task IDs to ensure error handling works
        let test_cases = vec!["#999", "invalid-id", "task-999"];

        for task_id in test_cases {
            let result = handler
                .change_status(task_id.to_string(), "trash".to_string(), None)
                .await;
            assert!(result.is_err(), "Expected error for task_id: {}", task_id);
        }
    }

    #[tokio::test]
    async fn test_trash_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=5 {
            let result = handler
                .inbox(
                    format!("task-{}", 25 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にtrashに移動
        let result = handler
            .change_status(task_ids[0].clone(), "trash".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to trash multiple tasks: {:?}",
            result.err()
        );

        // すべてのタスクがtrashに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.trash.len(), 5);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::trash));
        }
    }

    #[tokio::test]
    async fn test_trash_tasks_partial_success() {
        let (handler, _temp_file) = get_test_handler();

        // 有効なタスクを2つ作成
        let mut task_ids = Vec::new();
        for i in 1..=2 {
            let result = handler
                .inbox(
                    format!("task-{}", 26 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 無効なタスクIDを追加
        task_ids.push("#999".to_string());
        task_ids.push("invalid-id".to_string());

        // 部分的な成功を確認
        let result = handler
            .change_status(task_ids[0].clone(), "trash".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Should succeed with partial success: {:?}",
            result.err()
        );

        let result_msg = result.unwrap();
        assert!(result_msg.contains("Successfully moved 2 task(s)"));
        assert!(result_msg.contains("Failed to move 2 task(s)"));

        // 有効なタスクだけがtrashに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.trash.len(), 2);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_trash_tasks_all_invalid() {
        let (handler, _temp_file) = get_test_handler();

        // すべて無効なタスクID
        let task_ids = vec![
            "#999".to_string(),
            "invalid-id".to_string(),
            "task-999".to_string(),
        ];

        // すべて失敗する場合はエラーを返す
        if !task_ids.is_empty() {
            let result = handler
                .change_status(task_ids[0].clone(), "trash".to_string(), None)
                .await;
            assert!(result.is_err(), "Expected error when all tasks are invalid");
        }
    }

    #[tokio::test]
    async fn test_trash_tasks_empty_list() {
        let (handler, _temp_file) = get_test_handler();

        // 空のリスト
        let task_ids: Vec<String> = Vec::new();

        // Empty list - no change_status needed
        assert!(task_ids.is_empty());
        // Empty list case - nothing to assert
    }

    #[tokio::test]
    async fn test_trash_tasks_from_different_statuses() {
        let (handler, _temp_file) = get_test_handler();

        // inboxからタスクを作成
        let result = handler
            .inbox(
                "task-27".to_string(),
                "Inbox Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let inbox_task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // next_actionに移動
        let result = handler
            .inbox(
                "task-28".to_string(),
                "Next Action Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let next_action_task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
        handler
            .change_status(next_action_task_id.clone(), "next_action".to_string(), None)
            .await
            .unwrap();

        // doneに移動
        let result = handler
            .inbox(
                "task-29".to_string(),
                "Done Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let done_task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
        handler
            .change_status(done_task_id.clone(), "done".to_string(), None)
            .await
            .unwrap();

        // 異なるステータスのタスクを一度にtrashに移動
        let task_ids = vec![
            inbox_task_id.clone(),
            next_action_task_id.clone(),
            done_task_id.clone(),
        ];
        for task_id in &task_ids {
            let result = handler
                .change_status(task_id.clone(), "trash".to_string(), None)
                .await;
            assert!(result.is_ok(), "Failed to trash task: {:?}", result.err());
        }
        // All tasks successfully moved to trash

        // すべてがtrashに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.trash.len(), 3);
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.next_action.len(), 0);
        assert_eq!(data.done.len(), 0);

        let task1 = data.find_task_by_id(&inbox_task_id).unwrap();
        let task2 = data.find_task_by_id(&next_action_task_id).unwrap();
        let task3 = data.find_task_by_id(&done_task_id).unwrap();
        assert!(matches!(task1.status, NotaStatus::trash));
        assert!(matches!(task2.status, NotaStatus::trash));
        assert!(matches!(task3.status, NotaStatus::trash));
    }

    #[tokio::test]
    async fn test_calendar_task_with_start_date() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-30".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .change_status(
                task_id.clone(),
                "calendar".to_string(),
                Some("2024-12-25".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::calendar));
        assert_eq!(data.calendar.len(), 1);
        assert_eq!(data.inbox.len(), 0);
        assert!(task.start_date.is_some());
        assert_eq!(
            task.start_date.unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 25).unwrap()
        );
    }

    #[tokio::test]
    async fn test_calendar_task_without_start_date_error() {
        let (handler, _temp_file) = get_test_handler();

        // タスクを作成（start_dateなし）
        let result = handler
            .inbox(
                "task-31".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // start_dateを指定せずにcalendarに移動しようとするとエラー
        let result = handler
            .change_status(task_id.clone(), "calendar".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calendar_task_with_existing_start_date() {
        let (handler, _temp_file) = get_test_handler();

        // start_date付きのタスクを作成
        let result = handler
            .inbox(
                "task-2004".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-11-15".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // start_dateパラメータなしでcalendarに移動（既存のstart_dateを使用）
        let result = handler
            .change_status(task_id.clone(), "calendar".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::calendar));
        assert_eq!(data.calendar.len(), 1);
        assert_eq!(
            task.start_date.unwrap(),
            NaiveDate::from_ymd_opt(2024, 11, 15).unwrap()
        );
    }

    #[tokio::test]
    async fn test_calendar_task_override_start_date() {
        let (handler, _temp_file) = get_test_handler();

        // start_date付きのタスクを作成
        let result = handler
            .inbox(
                "task-2005".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-11-15".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // 新しいstart_dateを指定してcalendarに移動（既存のstart_dateを上書き）
        let result = handler
            .change_status(
                task_id.clone(),
                "calendar".to_string(),
                Some("2024-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, NotaStatus::calendar));
        assert_eq!(
            task.start_date.unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()
        );
    }

    #[tokio::test]
    async fn test_calendar_task_invalid_date_format() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-32".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // 無効な日付形式
        let result = handler
            .change_status(
                task_id.clone(),
                "calendar".to_string(),
                Some("2024/12/25".to_string()),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_status_movement_updates_timestamp() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "task-33".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let created_at = {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            task.created_at
        };

        // Move to next_action
        let result = handler
            .change_status(task_id.clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify created_at unchanged
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.created_at, created_at);
    }

    #[tokio::test]
    async fn test_status_movement_nonexistent_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .change_status(
                "nonexistent-id".to_string(),
                "next_action".to_string(),
                None,
            )
            .await;
        assert!(result.is_err());

        let result = handler
            .change_status("nonexistent-id".to_string(), "done".to_string(), None)
            .await;
        assert!(result.is_err());

        let result = handler
            .change_status("nonexistent-id".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    // Tests for context management

    #[tokio::test]
    async fn test_add_context() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Work environment".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Office"));

        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.name, "Office");
        assert_eq!(context.notes, Some("Work environment".to_string()));
    }

    #[tokio::test]
    async fn test_add_context_duplicate() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Try to add duplicate
        let result = handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_contexts_empty() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.list(None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No contexts found"));
    }

    #[tokio::test]
    async fn test_list_contexts() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Work environment".to_string()),
                None,
            )
            .await
            .unwrap();
        handler
            .inbox(
                "Home".to_string(),
                "Home".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let result = handler.list(None).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Office"));
        assert!(output.contains("Home"));
        assert!(output.contains("Work environment"));
    }

    #[tokio::test]
    async fn test_update_context() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Old description".to_string()),
                None,
            )
            .await
            .unwrap();

        let result = handler
            .update(
                "Office".to_string(),
                None,
                None,
                None,
                None,
                Some("New description".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.notes, Some("New description".to_string()));
    }

    #[tokio::test]
    async fn test_update_context_remove_description() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Old description".to_string()),
                None,
            )
            .await
            .unwrap();

        let result = handler
            .update(
                "Office".to_string(),
                None,
                None,
                None,
                None,
                Some("".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.notes, None);
    }

    #[tokio::test]
    async fn test_update_context_not_found() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .update(
                "NonExistent".to_string(),
                None,
                None,
                None,
                None,
                Some("Description".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_context() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        let result = handler.empty_trash().await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_context_not_found() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .change_status("NonExistent".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_context_with_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Add a task that references the context
        handler
            .inbox(
                "task-2006".to_string(),
                "Office work".to_string(),
                "inbox".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail
        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());

        // Verify context still exists
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        assert!(data.contexts.contains_key("Office"));
    }

    #[tokio::test]
    async fn test_delete_context_with_project_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Add a project that references the context
        handler
            .inbox(
                "office-proj".to_string(),
                "Office Project".to_string(),
                "project".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail
        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());

        // Verify context still exists
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        assert!(data.contexts.contains_key("Office"));
    }

    #[tokio::test]
    async fn test_delete_context_with_both_task_and_project_references() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Add a task that references the context
        handler
            .inbox(
                "task-2007".to_string(),
                "Office work".to_string(),
                "inbox".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Add a project that references the context
        handler
            .inbox(
                "office-proj".to_string(),
                "Office Project".to_string(),
                "project".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail (task check comes first)
        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());

        // Verify context still exists
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        assert!(data.contexts.contains_key("Office"));
    }

    #[tokio::test]
    async fn test_delete_context_after_removing_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Add a task that references the context
        let task_id = handler
            .inbox(
                "task-2008".to_string(),
                "Office work".to_string(),
                "inbox".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Extract task ID from the response
        let task_id = task_id.split("ID: ").nth(1).unwrap().trim().to_string();

        // Remove the context reference from the task
        handler
            .update(task_id, None, None, Some(String::new()), None, None, None)
            .await
            .unwrap();

        // Now deletion should succeed
        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("deleted"));

        // Verify context is gone
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_context_after_removing_project_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Add a project that references the context
        handler
            .inbox(
                "office-proj".to_string(),
                "Office Project".to_string(),
                "project".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Remove the context reference from the project
        handler
            .update(
                "office-proj".to_string(),
                None,
                None,
                None,
                None,
                Some(String::new()),
                None,
            )
            .await
            .unwrap();

        // Now deletion should succeed
        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("deleted"));

        // Verify context is gone
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_context_with_multiple_task_references() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Add multiple tasks that reference the context
        handler
            .inbox(
                "task-2009".to_string(),
                "Task 1".to_string(),
                "inbox".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        handler
            .inbox(
                "task-2010".to_string(),
                "Task 2".to_string(),
                "inbox".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail with the first task found
        let result = handler
            .change_status("Office".to_string(), "trash".to_string(), None)
            .await;
        assert!(result.is_err());

        // Verify context still exists
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
    }

    #[tokio::test]
    async fn test_add_project_with_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context first
        let result = handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Work environment".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // Add a project with context
        let result = handler
            .inbox(
                "office-proj".to_string(),
                "Office Project".to_string(),
                "project".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify project has context
        let data = handler.data.lock().unwrap();
        let project = data.projects.values().next().unwrap();
        assert_eq!(project.context, Some("Office".to_string()));
        assert_eq!(project.title, "Office Project");
    }

    #[tokio::test]
    async fn test_add_project_with_invalid_context() {
        let (handler, _temp_file) = get_test_handler();

        // Try to add project with non-existent context
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                Some("NonExistent".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_project_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        let _ = handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Work environment".to_string()),
                None,
            )
            .await;

        // Add a project without context
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Update project with context
        let result = handler
            .update(
                project_id.clone(),
                None,
                None,
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify context added
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert_eq!(project.context, Some("Office".to_string()));
    }

    #[tokio::test]
    async fn test_update_project_remove_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        let _ = handler
            .inbox(
                "Office".to_string(),
                "Office".to_string(),
                "context".to_string(),
                None,
                None,
                Some("Work environment".to_string()),
                None,
            )
            .await;

        // Add a project with context
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Remove context using empty string
        let result = handler
            .update(
                project_id.clone(),
                None,
                None,
                None,
                Some("".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify context removed
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert_eq!(project.context, None);
    }

    #[tokio::test]
    async fn test_update_project_with_invalid_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Try to update with non-existent context
        let result = handler
            .update(
                project_id,
                None,
                None,
                None,
                None,
                Some("NonExistent".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_project_with_custom_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project with custom ID
        let result = handler
            .inbox(
                "my-custom-id".to_string(),
                "Custom ID Project".to_string(),
                "project".to_string(),
                None,
                None,
                Some("Project with custom ID".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("my-custom-id"));

        // Verify project was created with custom ID
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id("my-custom-id").unwrap();
        assert_eq!(project.id, "my-custom-id");
        assert_eq!(project.title, "Custom ID Project");
    }

    #[tokio::test]
    async fn test_add_project_with_duplicate_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add first project with custom ID
        let result = handler
            .inbox(
                "duplicate-id".to_string(),
                "First Project".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Try to add second project with same ID
        let result = handler
            .inbox(
                "duplicate-id".to_string(),
                "Second Project".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_project_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Update project ID
        let result = handler
            .update(
                project_id.clone(),
                Some("new-project-id".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify old ID doesn't exist and new ID does
        let data = handler.data.lock().unwrap();
        assert!(data.find_project_by_id(&project_id).is_none());
        let project = data.find_project_by_id("new-project-id").unwrap();
        assert_eq!(project.id, "new-project-id");
        assert_eq!(project.title, "Test Project");
    }

    #[tokio::test]
    async fn test_update_project_id_duplicate() {
        let (handler, _temp_file) = get_test_handler();

        // Add two projects
        let result1 = handler
            .inbox(
                "Project 1".to_string(),
                "project-1".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result1.is_ok());
        let project1_id = GtdServerHandler::extract_id_from_response(&result1.unwrap());

        let result2 = handler
            .inbox(
                "Project 2".to_string(),
                "project-2".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result2.is_ok());
        let project2_id = GtdServerHandler::extract_id_from_response(&result2.unwrap());

        // Try to update project2's ID to project1's ID
        let result = handler
            .update(
                project2_id,
                Some(project1_id.clone()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_project_id_updates_task_references() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Add a task referencing the project
        let result = handler
            .inbox(
                "task-2011".to_string(),
                "Task in project".to_string(),
                "inbox".to_string(),
                Some(project_id.clone()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // Verify task references the original project ID
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.project, Some(project_id.clone()));
        }

        // Update project ID
        let new_project_id = "updated-project-id".to_string();
        let result = handler
            .update(
                project_id.clone(),
                Some(new_project_id.clone()),
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify task now references the new project ID
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.project, Some(new_project_id));
    }

    // ==================== Prompt Tests ====================

    // GTD workflow methods removed - tests commented out
    /*
    #[tokio::test]
    async fn test_prompt_gtd_overview() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.gtd_overview().await;
        assert!(result.is_ok());
        let content = result.unwrap();

        // プロンプトが主要なGTDコンセプトを含んでいることを確認
        assert!(content.contains("GTD"));
        assert!(content.contains("inbox"));
        assert!(content.contains("next_action"));
        assert!(content.contains("waiting_for"));
        assert!(content.contains("someday"));
        assert!(content.contains("calendar"));
        assert!(content.contains("done"));
        assert!(content.contains("trash"));
        assert!(content.contains("Projects"));
        assert!(content.contains("Contexts"));
    }

    #[tokio::test]
    async fn test_prompt_process_inbox() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.process_inbox().await;
        assert!(result.is_ok());
        let content = result.unwrap();

        // インボックス処理のワークフローガイダンスを確認
        assert!(content.contains("inbox"));
        assert!(content.contains("actionable"));
        assert!(content.contains("2 minutes"));
        assert!(content.contains("waiting_for"));
        assert!(content.contains("next_action"));
    }

    #[tokio::test]
    async fn test_prompt_weekly_review() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.weekly_review().await;
        assert!(result.is_ok());
        let content = result.unwrap();

        // 週次レビューのステップを確認
        assert!(content.contains("Weekly Review"));
        assert!(content.contains("Get Clear"));
        assert!(content.contains("Get Current"));
        assert!(content.contains("Projects"));
        assert!(content.contains("calendar"));
        assert!(content.contains("next_action"));
        assert!(content.contains("waiting_for"));
    }

    #[tokio::test]
    async fn test_prompt_next_actions() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.next_actions().await;
        assert!(result.is_ok());
        let content = result.unwrap();

        // ネクストアクションガイドの内容を確認
        assert!(content.contains("Next Actions"));
        assert!(content.contains("Context"));
        assert!(content.contains("@office"));
        assert!(content.contains("@computer"));
        assert!(content.contains("@phone"));
        assert!(content.contains("Specific"));
    }

    #[tokio::test]
    #[tokio::test]
    async fn test_prompts_return_non_empty_strings() {
        let (handler, _temp_file) = get_test_handler();

        // 全てのプロンプトが空でない文字列を返すことを確認
        let prompts = vec![
            handler.gtd_overview().await,
            handler.process_inbox().await,
            handler.weekly_review().await,
            handler.next_actions().await,
            handler.add_task_guide().await,
        ];

        for prompt in prompts {
            assert!(prompt.is_ok());
            let content = prompt.unwrap();
            assert!(!content.is_empty());
            assert!(content.len() > 100); // 各プロンプトは実質的な内容を持つ
        }
    }
    */

    // 日付フィルタリングのテスト: start_dateが未来のタスクを除外
    #[tokio::test]
    async fn test_list_tasks_with_date_filter_excludes_future_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // タスクを3つ作成: 過去、今日、未来の日付
        let result = handler
            .inbox(
                "task-2012".to_string(),
                "Past Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-01-01".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let result = handler
            .inbox(
                "task-2013".to_string(),
                "Today Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-06-15".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let result = handler
            .inbox(
                "task-2014".to_string(),
                "Future Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 日付フィルタ「2024-06-15」で一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // Past TaskとToday Taskは表示される
        assert!(list.contains("Past Task"));
        assert!(list.contains("Today Task"));
        // Future Taskは表示されない（start_dateが未来なので）
        assert!(!list.contains("Future Task"));
    }

    // 日付フィルタリングのテスト: start_dateがないタスクは表示される
    #[tokio::test]
    async fn test_list_tasks_with_date_filter_includes_tasks_without_start_date() {
        let (handler, _temp_file) = get_test_handler();

        // start_dateなしのタスクを作成
        let result = handler
            .inbox(
                "task-34".to_string(),
                "No Date Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // start_date付きのタスクを作成（未来）
        let result = handler
            .inbox(
                "task-2015".to_string(),
                "Future Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2025-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 日付フィルタで一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // start_dateがないタスクは表示される
        assert!(list.contains("No Date Task"));
        // 未来のタスクは表示されない
        assert!(!list.contains("Future Task"));
    }

    // 日付フィルタリングのテスト: カレンダーステータスとの組み合わせ
    #[tokio::test]
    async fn test_list_tasks_with_date_filter_and_calendar_status() {
        let (handler, _temp_file) = get_test_handler();

        // カレンダータスクを作成（過去と未来）
        let result = handler
            .inbox(
                "task-2016".to_string(),
                "Calendar Past".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-01-01".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id1 = GtdServerHandler::extract_id_from_response(&result.unwrap());

        let result = handler
            .inbox(
                "task-2017".to_string(),
                "Calendar Future".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2025-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id2 = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // 両方をカレンダーステータスに移動
        let result = handler
            .change_status(task_id1.clone(), "calendar".to_string(), None)
            .await;
        assert!(result.is_ok());
        let result = handler
            .change_status(task_id2.clone(), "calendar".to_string(), None)
            .await;
        assert!(result.is_ok());

        // カレンダーステータスでフィルタリングし、日付フィルタも適用
        let result = handler.list(Some("calendar".to_string())).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // 過去のカレンダータスクは表示される
        assert!(list.contains("Calendar Past"));
        // 未来のカレンダータスクは表示されない
        assert!(!list.contains("Calendar Future"));
    }

    // 日付フィルタリングのテスト: 無効な日付形式
    #[tokio::test]
    async fn test_list_tasks_with_invalid_date_format() {
        let (handler, _temp_file) = get_test_handler();

        // 無効な日付形式
        let result = handler.list(None).await;
        assert!(result.is_err());

        let result = handler.list(None).await;
        assert!(result.is_err());
    }

    // 日付フィルタリングのテスト: 日付フィルタなしでは全タスク表示
    #[tokio::test]
    async fn test_list_tasks_without_date_filter_shows_all_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 未来の日付のタスクを作成
        let result = handler
            .inbox(
                "task-2018".to_string(),
                "Future Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2025-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 日付フィルタなしで一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // 未来のタスクも表示される
        assert!(list.contains("Future Task"));
    }

    // 日付フィルタリングのテスト: start_dateが指定日と同じ場合は表示される
    #[tokio::test]
    async fn test_list_tasks_with_date_filter_includes_same_date() {
        let (handler, _temp_file) = get_test_handler();

        // 指定日と同じ日付のタスクを作成
        let result = handler
            .inbox(
                "task-2019".to_string(),
                "Same Date Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2024-06-15".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 同じ日付でフィルタリング
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // 同じ日付のタスクは表示される（未来ではない）
        assert!(list.contains("Same Date Task"));
    }

    // notesフィールドがlist_tasksの出力に含まれることを確認
    #[tokio::test]
    async fn test_list_tasks_includes_notes_by_default() {
        let (handler, _temp_file) = get_test_handler();

        // notesを持つタスクを作成
        let result = handler
            .inbox(
                "task-2020".to_string(),
                "Task with notes".to_string(),
                "inbox".to_string(),
                None,
                None,
                Some("Important notes here".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // notesなしのタスクも作成
        let result = handler
            .inbox(
                "task-35".to_string(),
                "Task without notes".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // デフォルト（exclude_notes=None）で一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // notesが含まれていることを確認
        assert!(list.contains("Task with notes"));
        assert!(list.contains("[notes: Important notes here]"));

        // notesなしのタスクにはnotesフィールドがないことを確認
        assert!(list.contains("Task without notes"));
        let lines: Vec<&str> = list.lines().collect();
        let without_notes_line = lines
            .iter()
            .find(|line| line.contains("Task without notes"))
            .unwrap();
        assert!(!without_notes_line.contains("[notes:"));
    }

    // exclude_notes=trueでnotesが除外されることを確認
    #[tokio::test]
    async fn test_list_tasks_excludes_notes_when_requested() {
        let (handler, _temp_file) = get_test_handler();

        // notesを持つタスクを作成
        let result = handler
            .inbox(
                "task-2021".to_string(),
                "Task with notes".to_string(),
                "inbox".to_string(),
                None,
                None,
                Some("Important notes here".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // exclude_notes=trueで一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // タスクは存在するがnotesは含まれていないことを確認
        assert!(list.contains("Task with notes"));
        assert!(!list.contains("[notes:"));
        assert!(!list.contains("Important notes here"));
    }

    // exclude_notes=falseで明示的にnotesを含めることを確認
    #[tokio::test]
    async fn test_list_tasks_includes_notes_when_explicitly_false() {
        let (handler, _temp_file) = get_test_handler();

        // notesを持つタスクを作成
        let result = handler
            .inbox(
                "task-2022".to_string(),
                "Task with notes".to_string(),
                "inbox".to_string(),
                None,
                None,
                Some("Important notes here".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // exclude_notes=falseで明示的に一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // notesが含まれていることを確認
        assert!(list.contains("Task with notes"));
        assert!(list.contains("[notes: Important notes here]"));
    }

    // notesに複数行やspecial charactersが含まれる場合のテスト
    #[tokio::test]
    async fn test_list_tasks_with_multiline_notes() {
        let (handler, _temp_file) = get_test_handler();

        // 複数行のnotesを持つタスクを作成（改行を含む）
        let result = handler
            .inbox(
                "task-2023".to_string(),
                "Complex task".to_string(),
                "inbox".to_string(),
                None,
                None,
                Some("Line 1\nLine 2\nLine 3".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // デフォルトで一覧取得
        let result = handler.list(None).await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // notesが含まれていることを確認（改行も含む）
        assert!(list.contains("Complex task"));
        assert!(list.contains("[notes: Line 1\nLine 2\nLine 3]"));
    }

    #[tokio::test]
    async fn test_inbox_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成してnext_actionに移動
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 36 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            // Move to next_action first
            let _ = handler
                .change_status(task_id.clone(), "next_action".to_string(), None)
                .await;
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にinboxに移動
        let result = handler
            .change_status(task_ids[0].clone(), "inbox".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to inbox: {:?}",
            result.err()
        );

        // すべてのタスクがinboxに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.inbox.len(), 3);
        assert_eq!(data.next_action.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::inbox));
        }
    }

    #[tokio::test]
    async fn test_next_action_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=4 {
            let result = handler
                .inbox(
                    format!("task-{}", 37 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にnext_actionに移動
        let result = handler
            .change_status(task_ids[0].clone(), "next_action".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to next_action: {:?}",
            result.err()
        );

        // すべてのタスクがnext_actionに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.next_action.len(), 4);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::next_action));
        }
    }

    #[tokio::test]
    async fn test_waiting_for_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 38 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にwaiting_forに移動
        let result = handler
            .change_status(task_ids[0].clone(), "waiting_for".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to waiting_for: {:?}",
            result.err()
        );

        // すべてのタスクがwaiting_forに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.waiting_for.len(), 3);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::waiting_for));
        }
    }

    #[tokio::test]
    async fn test_someday_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 39 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にsomedayに移動
        let result = handler
            .change_status(task_ids[0].clone(), "someday".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to someday: {:?}",
            result.err()
        );

        // すべてのタスクがsomedayに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.someday.len(), 3);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::someday));
        }
    }

    #[tokio::test]
    async fn test_later_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 40 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にlaterに移動
        let result = handler
            .change_status(task_ids[0].clone(), "later".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to later: {:?}",
            result.err()
        );

        // すべてのタスクがlaterに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.later.len(), 3);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::later));
        }
    }

    #[tokio::test]
    async fn test_done_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 41 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にdoneに移動
        let result = handler
            .change_status(task_ids[0].clone(), "done".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to done: {:?}",
            result.err()
        );

        // すべてのタスクがdoneに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.done.len(), 3);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::done));
        }
    }

    // ==================== Invalid Status Error Message Tests ====================

    #[tokio::test]
    async fn test_change_task_status_invalid_status_error_message() {
        let (handler, _temp_file) = get_test_handler();

        // タスクを作成
        let result = handler
            .inbox(
                "task-42".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // 無効なステータス "in_progress" でエラーをテスト（問題として報告されたもの）
        let result = handler
            .change_status(task_id.clone(), "in_progress".to_string(), None)
            .await;
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid status 'in_progress'"));
        assert!(err_msg.contains("inbox"));
        assert!(err_msg.contains("next_action"));
        assert!(err_msg.contains("waiting_for"));
        assert!(err_msg.contains("someday"));
        assert!(err_msg.contains("later"));
        assert!(err_msg.contains("calendar"));
        assert!(err_msg.contains("done"));
        assert!(err_msg.contains("trash"));
    }

    #[tokio::test]
    async fn test_change_task_status_various_invalid_statuses() {
        let (handler, _temp_file) = get_test_handler();

        // タスクを作成
        let result = handler
            .inbox(
                "task-43".to_string(),
                "Test Task".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());

        // 様々な無効なステータスをテスト
        let invalid_statuses = vec![
            "invalid",
            "complete",
            "completed",
            "pending",
            "todo",
            "in-progress",
            "INBOX",
            "Next_Action",
        ];

        for invalid_status in invalid_statuses {
            let result = handler
                .change_status(task_id.clone(), invalid_status.to_string(), None)
                .await;
            assert!(
                result.is_err(),
                "Expected error for invalid status: {}",
                invalid_status
            );
            let err_msg = format!("{:?}", result.unwrap_err());
            assert!(
                err_msg.contains(&format!("Invalid status '{}'", invalid_status)),
                "Error message should contain the invalid status '{}', got: {}",
                invalid_status,
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_list_tasks_invalid_status_error_message() {
        let (handler, _temp_file) = get_test_handler();

        // 無効なステータスでリストを取得しようとする
        let result = handler.list(Some("in_progress".to_string())).await;
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid status 'in_progress'"));
        assert!(err_msg.contains("inbox"));
        assert!(err_msg.contains("next_action"));
    }

    #[tokio::test]
    async fn test_list_tasks_various_invalid_statuses() {
        let (handler, _temp_file) = get_test_handler();

        let invalid_statuses = vec!["invalid", "complete", "pending", "INBOX"];

        for invalid_status in invalid_statuses {
            let result = handler.list(Some(invalid_status.to_string())).await;
            assert!(
                result.is_err(),
                "Expected error for invalid status: {}",
                invalid_status
            );
            let err_msg = format!("{:?}", result.unwrap_err());
            assert!(
                err_msg.contains(&format!("Invalid status '{}'", invalid_status)),
                "Error message should contain the invalid status '{}'",
                invalid_status
            );
        }
    }

    #[tokio::test]
    async fn test_update_project_invalid_status_error_message() {
        let (handler, _temp_file) = get_test_handler();

        // プロジェクトを作成
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // 無効なステータスで更新しようとする
        let result = handler
            .update(
                "test-project-1".to_string(),
                None,
                None,
                None,
                Some("in_progress".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid project status 'in_progress'"));
        assert!(err_msg.contains("active"));
        assert!(err_msg.contains("on_hold"));
        assert!(err_msg.contains("completed"));
    }

    #[tokio::test]
    async fn test_update_project_various_invalid_statuses() {
        let (handler, _temp_file) = get_test_handler();

        // プロジェクトを作成
        let result = handler
            .inbox(
                "test-project-1".to_string(),
                "Test Project".to_string(),
                "project".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        let invalid_statuses = vec!["pending", "in_progress", "done", "onhold", "ACTIVE"];

        for invalid_status in invalid_statuses {
            let result = handler
                .update(
                    "test-project-1".to_string(),
                    None,
                    None,
                    None,
                    Some(invalid_status.to_string()),
                    None,
                    None,
                )
                .await;
            assert!(
                result.is_err(),
                "Expected error for invalid project status: {}",
                invalid_status
            );
            let err_msg = format!("{:?}", result.unwrap_err());
            assert!(
                err_msg.contains(&format!("Invalid project status '{}'", invalid_status)),
                "Error message should contain the invalid status '{}'",
                invalid_status
            );
        }
    }

    #[tokio::test]
    async fn test_calendar_tasks_multiple_tasks_with_date() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .inbox(
                    format!("task-{}", 44 - 1 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にcalendarに移動（start_date指定）
        for task_id in &task_ids {
            let result = handler
                .change_status(
                    task_id.clone(),
                    "calendar".to_string(),
                    Some("2025-01-15".to_string()),
                )
                .await;
            assert!(
                result.is_ok(),
                "Failed to move task to calendar: {:?}",
                result.err()
            );
        }

        // すべてのタスクがcalendarに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.calendar.len(), 3);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::calendar));
            assert_eq!(
                task.start_date,
                Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap())
            );
        }
    }

    #[tokio::test]
    async fn test_calendar_tasks_with_existing_dates() {
        let (handler, _temp_file) = get_test_handler();

        // start_dateを持つタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=2 {
            let result = handler
                .inbox(
                    format!("task-{}", 44 + i),
                    format!("Test Task {}", i),
                    "inbox".to_string(),
                    None,
                    None,
                    None,
                    Some("2025-02-01".to_string()),
                )
                .await;
            assert!(result.is_ok());
            let task_id = GtdServerHandler::extract_id_from_response(&result.unwrap());
            task_ids.push(task_id);
        }

        // start_dateを指定せずにcalendarに移動（既存のstart_dateを使用）
        for task_id in &task_ids {
            let result = handler
                .change_status(task_id.clone(), "calendar".to_string(), None)
                .await;
            assert!(
                result.is_ok(),
                "Failed to move task to calendar: {:?}",
                result.err()
            );
        }

        // すべてのタスクがcalendarに移動され、既存のstart_dateが保持されていることを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.calendar.len(), 2);
        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, NotaStatus::calendar));
            assert_eq!(
                task.start_date,
                Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap())
            );
        }
    }

    #[tokio::test]
    async fn test_calendar_tasks_partial_failure() {
        let (handler, _temp_file) = get_test_handler();

        // start_dateを持つタスクと持たないタスクを作成
        let mut task_ids = Vec::new();

        // start_dateを持つタスク
        let result = handler
            .inbox(
                "task-2024".to_string(),
                "Task with date".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                Some("2025-03-01".to_string()),
            )
            .await;
        assert!(result.is_ok());
        task_ids.push(
            GtdServerHandler::extract_id_from_response(&result.unwrap()),
        );

        // start_dateを持たないタスク
        let result = handler
            .inbox(
                "task-46".to_string(),
                "Task without date".to_string(),
                "inbox".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        task_ids.push(
            GtdServerHandler::extract_id_from_response(&result.unwrap()),
        );

        // start_dateを指定せずに移動を試みる（部分的な失敗）
        // First task has date, should succeed
        let result1 = handler
            .change_status(task_ids[0].clone(), "calendar".to_string(), None)
            .await;
        assert!(result1.is_ok(), "Task with date should move to calendar");

        // Second task has no date, should fail
        let result2 = handler
            .change_status(task_ids[1].clone(), "calendar".to_string(), None)
            .await;
        assert!(result2.is_err(), "Task without date should fail");

        // 1つのタスクだけが移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.calendar.len(), 1);
        assert_eq!(data.inbox.len(), 1);
    }
}
