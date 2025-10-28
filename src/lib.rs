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

pub mod formatting;
pub mod git_ops;
pub mod gtd;
pub mod handlers;
pub mod migration;
pub mod storage;
pub mod validation;

use anyhow::Result;

use mcp_attr::Result as McpResult;
use mcp_attr::server::{McpServer, mcp_server};
use std::sync::Mutex;

// Re-export for integration tests (McpServer trait already in scope above)

// Re-export commonly used types
pub use git_ops::GitOps;
pub use gtd::{GtdData, Nota, NotaStatus, local_date_today};
pub use storage::Storage;

/// MCP Server handler for GTD task management
///
/// Provides an MCP interface to GTD functionality including task management,
/// project tracking, and context organization. All changes are automatically
/// persisted to a TOML file and optionally synchronized with Git.
pub struct GtdServerHandler {
    pub data: Mutex<GtdData>,
    pub storage: Storage,
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

    /// Save GTD data with a default commit message.
    ///
    /// Persists the current in-memory GTD data to disk using the default commit message
    /// defined in `Storage::save()`, which is "Update GTD data".
    /// This is typically called by handler modules after modifying GTD data,
    /// following the MCP tool implementation pattern.
    pub fn save_data(&self) -> Result<()> {
        let data = self.data.lock().unwrap();
        self.storage.save(&data)?;
        Ok(())
    }

    /// Save GTD data with a custom commit message.
    ///
    /// Persists the current GTD data to disk and creates a Git commit using the provided message.
    ///
    /// # Arguments
    /// * `message` - Commit message to use for the Git version history.
    pub(crate) fn save_data_with_message(&self, message: &str) -> Result<()> {
        let data = self.data.lock().unwrap();
        self.storage.save_with_message(&data, message)?;
        Ok(())
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

/// GTD task management server implementing David Allen's methodology.
/// Workflow: Capture(inbox) → Review(list) → Clarify(update) → Organize(change_status) → Do → Purge(empty_trash)
///
/// **Statuses**: inbox(start) | next_action(ready) | waiting_for(blocked) | later(deferred) | calendar(dated) | someday(maybe) | done | reference | trash
/// **Types**: task | project(multi-step) | context(@location)
/// **IDs**: Use meaningful strings (e.g., "call-john", "website-redesign")
#[mcp_server]
impl McpServer for GtdServerHandler {
    /// **Purge**: Permanently delete all trashed items. Run weekly.
    /// **When**: Part of weekly review - trash items first with change_status, then purge.
    /// **Safety**: Checks references to prevent broken links.
    #[tool]
    pub async fn empty_trash(&self) -> McpResult<String> {
        self.handle_empty_trash().await
    }

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
    #[tool]
    pub async fn inbox(
        &self,
        /// Unique string ID - follow kebab-case guidelines above (e.g., "call-john", "web-redesign")
        id: String,
        /// Brief description
        title: String,
        /// inbox | next_action | waiting_for | later | calendar | someday | done | reference | project | context | trash
        status: String,
        /// Optional: Parent project ID
        project: Option<String>,
        /// Optional: Where applies (e.g., "@home", "@office")
        context: Option<String>,
        /// Optional: Markdown notes
        notes: Option<String>,
        /// Optional: YYYY-MM-DD, required for calendar status
        start_date: Option<String>,
        /// Optional: Recurrence pattern - daily | weekly | monthly | yearly
        recurrence: Option<String>,
        /// Optional: Recurrence configuration
        /// - weekly: weekday names (e.g., "Monday,Wednesday,Friday")
        /// - monthly: day numbers (e.g., "1,15,25")
        /// - yearly: month-day pairs (e.g., "1-1,12-25" for Jan 1 and Dec 25)
        recurrence_config: Option<String>,
    ) -> McpResult<String> {
        self.handle_inbox(
            id,
            title,
            status,
            project,
            context,
            notes,
            start_date,
            recurrence,
            recurrence_config,
        )
        .await
    }

    /// **Review**: List/filter all items. Essential for daily/weekly reviews.
    /// **When**: Daily - check next_action. Weekly - review all. Use filters to focus.
    /// **Filters**: No filter=all | status="inbox"=unprocessed | status="next_action"=ready | status="calendar"+date=today's tasks | keyword="text"=search | project="id"=by project | context="name"=by context.
    #[tool]
    pub async fn list(
        &self,
        /// Optional: Filter by status (inbox | next_action | waiting_for | later | calendar | someday | done | reference | project | context | trash)
        status: Option<String>,
        /// Optional: Date filter YYYY-MM-DD - For calendar, shows tasks with start_date <= this date
        date: Option<String>,
        /// Optional: True to exclude notes and reduce token usage
        exclude_notes: Option<bool>,
        /// Optional: Search keyword in id, title and notes (case-insensitive)
        keyword: Option<String>,
        /// Optional: Filter by project ID - use meaningful abbreviation (e.g., "website-redesign", "q1-budget")
        project: Option<String>,
        /// Optional: Filter by context name
        context: Option<String>,
    ) -> McpResult<String> {
        self.handle_list(status, date, exclude_notes, keyword, project, context)
            .await
    }

    /// **Clarify**: Update item details. Add context, notes, project links after capturing.
    /// **When**: After inbox capture, clarify what it is, why it matters, what's needed.
    /// **Tip**: Use ""(empty string) to clear optional fields.
    /// **Note**: Item ID cannot be changed - IDs are immutable. To "rename", create new item and delete old one.
    #[allow(clippy::too_many_arguments)]
    #[tool]
    pub async fn update(
        &self,
        /// Item ID to update (immutable - cannot be changed)
        id: String,
        /// Optional: New title
        title: Option<String>,
        /// Optional: New status (changes type if project/context)
        status: Option<String>,
        /// Optional: Project link, ""=clear
        project: Option<String>,
        /// Optional: Context tag, ""=clear
        context: Option<String>,
        /// Optional: Markdown notes, ""=clear
        notes: Option<String>,
        /// Optional: Start date YYYY-MM-DD, ""=clear
        start_date: Option<String>,
    ) -> McpResult<String> {
        self.handle_update(id, title, status, project, context, notes, start_date)
            .await
    }

    /// **Organize/Do**: Move items through workflow stages as you process them.
    /// **When**: inbox→next_action(ready) | →waiting_for(blocked) | →done(complete) | →trash(discard).
    /// **Tip**: Use change_status to trash before empty_trash to permanently delete.
    /// **Batch**: Supports multiple IDs for efficient batch operations (e.g., weekly review).
    #[tool]
    pub async fn change_status(
        &self,
        /// Item IDs to change - format: ["#1", "#2", "#3"] for batch operations, or single ID for single item
        ids: Vec<String>,
        /// New status: inbox | next_action | waiting_for | later | calendar | someday | done | reference | project | context | trash
        new_status: String,
        /// Optional: Start date YYYY-MM-DD (required for calendar)
        start_date: Option<String>,
    ) -> McpResult<String> {
        self.handle_change_status(ids, new_status, start_date).await
    }
}
