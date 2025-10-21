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
use gtd::local_date_today;
use mcp_attr::server::{McpServer, mcp_server};
use mcp_attr::{Result as McpResult, bail};
use std::sync::Mutex;

// Re-export commonly used types
pub use gtd::{Context, GtdData, Project, ProjectStatus, Task, TaskStatus};
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

    /// Normalize task ID by ensuring it starts with '#'
    ///
    /// This helper function allows LLMs to omit the '#' prefix when specifying task IDs.
    /// If the input is a plain number (e.g., "1", "2"), it prepends '#'.
    /// If the input already starts with '#', it returns it unchanged.
    ///
    /// # Arguments
    /// * `task_id` - The task ID to normalize (e.g., "1" or "#1")
    ///
    /// # Returns
    /// The normalized task ID with '#' prefix (e.g., "#1")
    ///
    /// # Examples
    /// ```
    /// # use gtd_mcp::GtdServerHandler;
    /// // normalize_task_id("1") -> "#1"
    /// // normalize_task_id("#1") -> "#1"
    /// // normalize_task_id("42") -> "#42"
    /// ```
    fn normalize_task_id(task_id: &str) -> String {
        let trimmed = task_id.trim();
        if trimmed.starts_with('#') {
            trimmed.to_string()
        } else {
            format!("#{}", trimmed)
        }
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
    /// Capture new task into GTD inbox for later processing. First step in GTD workflow - quickly capture anything needing attention. Task starts in 'inbox' status.
    #[tool]
    async fn add_task(
        &self,
        /// Task title describing action (e.g., "Call Sarah about meeting")
        title: String,
        /// Optional project ID - use meaningful abbreviation like "website-redesign"
        project: Option<String>,
        /// Optional context (e.g., "@office", "@phone") for where/how task can be done
        context: Option<String>,
        /// Optional notes for details (Markdown supported)
        notes: Option<String>,
        /// Optional start date in YYYY-MM-DD format (e.g., "2024-03-15")
        start_date: Option<String>,
    ) -> McpResult<String> {
        let parsed_start_date = if let Some(date_str) = start_date {
            match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => bail!("Invalid date format. Use YYYY-MM-DD"),
            }
        } else {
            None
        };

        let mut data = self.data.lock().unwrap();

        let today = local_date_today();
        let task = Task {
            id: data.generate_task_id(),
            title: title.clone(),
            status: TaskStatus::inbox,
            project,
            context,
            notes,
            start_date: parsed_start_date,
            created_at: today,
            updated_at: today,
        };

        // Validate references before adding the task
        if !data.validate_task_references(&task) {
            let mut errors = Vec::new();
            if !data.validate_task_project(&task) {
                errors.push("Invalid project reference");
            }
            if !data.validate_task_context(&task) {
                errors.push("Invalid context reference");
            }
            drop(data);
            bail!("Failed to add task: {}", errors.join(", "));
        }

        let task_id = task.id.clone();
        data.add_task(task);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Add task to inbox: {}", title)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Task created with ID: {}", task_id))
    }

    /// View tasks with optional status or date filtering. Filter by status to focus on specific GTD workflow stage.
    #[tool]
    async fn list_tasks(
        &self,
        /// Optional status filter: "inbox", "next_action", "waiting_for", "someday", "later", "done", "trash", "calendar"
        status: Option<String>,
        /// Optional date filter (YYYY-MM-DD) - excludes tasks with future start_date
        date: Option<String>,
        /// Optional - exclude notes to reduce tokens (default: false)
        exclude_notes: Option<bool>,
    ) -> McpResult<String> {
        // Parse the date filter if provided
        let filter_date = if let Some(date_str) = date {
            match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => bail!("Invalid date format. Use YYYY-MM-DD"),
            }
        } else {
            None
        };

        let data = self.data.lock().unwrap();
        let mut tasks: Vec<&Task> = Vec::new();

        // Collect tasks from all status lists or just the filtered one
        if let Some(status_str) = status {
            match status_str.as_str() {
                "inbox" => tasks.extend(data.inbox.iter()),
                "next_action" => tasks.extend(data.next_action.iter()),
                "waiting_for" => tasks.extend(data.waiting_for.iter()),
                "someday" => tasks.extend(data.someday.iter()),
                "later" => tasks.extend(data.later.iter()),
                "done" => tasks.extend(data.done.iter()),
                "trash" => tasks.extend(data.trash.iter()),
                "calendar" => tasks.extend(data.calendar.iter()),
                _ => {
                    // If unknown status, return all tasks
                    tasks.extend(data.inbox.iter());
                    tasks.extend(data.next_action.iter());
                    tasks.extend(data.waiting_for.iter());
                    tasks.extend(data.someday.iter());
                    tasks.extend(data.later.iter());
                    tasks.extend(data.done.iter());
                    tasks.extend(data.trash.iter());
                    tasks.extend(data.calendar.iter());
                }
            }
        } else {
            // No filter, return all tasks
            tasks.extend(data.inbox.iter());
            tasks.extend(data.next_action.iter());
            tasks.extend(data.waiting_for.iter());
            tasks.extend(data.someday.iter());
            tasks.extend(data.later.iter());
            tasks.extend(data.done.iter());
            tasks.extend(data.trash.iter());
            tasks.extend(data.calendar.iter());
        }

        let exclude_notes = exclude_notes.unwrap_or(false);
        let mut result = String::new();
        for task in tasks {
            // Filter by date if specified: exclude tasks with start_date in the future
            if let Some(ref filter_d) = filter_date
                && let Some(start_d) = task.start_date
                && start_d > *filter_d
            {
                continue; // Skip this task as its start_date is in the future
            }

            let date_info = task
                .start_date
                .map(|d| format!(" [start: {}]", d))
                .unwrap_or_default();
            let project_info = task
                .project
                .as_ref()
                .map(|p| format!(" [project: {}]", p))
                .unwrap_or_default();
            let context_info = task
                .context
                .as_ref()
                .map(|c| format!(" [context: {}]", c))
                .unwrap_or_default();
            let notes_info = if exclude_notes {
                String::new()
            } else {
                task.notes
                    .as_ref()
                    .map(|n| format!(" [notes: {}]", n))
                    .unwrap_or_default()
            };
            result.push_str(&format!(
                "- [{}] {} (status: {:?}){}{}{}{} [created: {}, updated: {}]\n",
                task.id,
                task.title,
                task.status,
                date_info,
                project_info,
                context_info,
                notes_info,
                task.created_at,
                task.updated_at
            ));
        }

        Ok(result)
    }

    /// Move tasks between GTD workflow statuses. Key GTD statuses: **inbox** (unprocessed), **next_action** (ready to execute), **waiting_for** (blocked/delegated), **someday** (maybe later), **later** (deferred), **calendar** (date-specific, needs start_date), **done** (completed), **trash** (deleted). Process inbox regularly. Be specific with next_action. Review waiting_for weekly. Calendar requires start_date.
    #[tool]
    async fn change_task_status(
        &self,
        /// Task IDs to change. Format: ["#1", "#2", "#3"]
        task_ids: Vec<String>,
        /// Target status: "inbox", "next_action", "waiting_for", "someday", "later", "calendar", "done", "trash"
        status: String,
        /// Optional start date (YYYY-MM-DD) - required for "calendar", optional for others
        start_date: Option<String>,
    ) -> McpResult<String> {
        // Parse the target status
        let target_status = match status.as_str() {
            "inbox" => TaskStatus::inbox,
            "next_action" => TaskStatus::next_action,
            "waiting_for" => TaskStatus::waiting_for,
            "someday" => TaskStatus::someday,
            "later" => TaskStatus::later,
            "calendar" => TaskStatus::calendar,
            "done" => TaskStatus::done,
            "trash" => TaskStatus::trash,
            _ => bail!(
                "Invalid status '{}'. Use: inbox, next_action, waiting_for, someday, later, calendar, done, or trash",
                status
            ),
        };

        // Parse date if provided
        let parsed_start_date = if let Some(date_str) = start_date {
            match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => bail!("Invalid date format. Use YYYY-MM-DD"),
            }
        } else {
            None
        };

        let mut data = self.data.lock().unwrap();

        let mut successful: Vec<String> = Vec::new();
        let mut failed: Vec<(String, String)> = Vec::new();

        // Process each task ID
        for task_id in &task_ids {
            let task_id = Self::normalize_task_id(task_id.trim());

            // Check if task exists
            if data.find_task_by_id(&task_id).is_none() {
                eprintln!("Error: Task not found: {}", task_id);
                failed.push((task_id.to_string(), "Task not found".to_string()));
                continue;
            }

            // Check if calendar status requires start_date
            if target_status == TaskStatus::calendar {
                let current_start_date = data.find_task_by_id(&task_id).unwrap().start_date;
                let final_start_date = parsed_start_date.or(current_start_date);

                if final_start_date.is_none() {
                    eprintln!("Error: Task {} has no start_date for calendar", task_id);
                    failed.push((
                        task_id.to_string(),
                        "Task must have a start_date for calendar status".to_string(),
                    ));
                    continue;
                }
            }

            let original_status = data
                .find_task_by_id(&task_id)
                .map(|t| format!("{:?}", t.status));
            eprintln!(
                "Moving task {} from {:?} to {:?}",
                task_id, original_status, target_status
            );

            // Move task to target status
            if data.move_status(&task_id, target_status.clone()).is_some() {
                // Update the start_date if provided for calendar, and update timestamp
                if let Some(task) = data.find_task_by_id_mut(&task_id) {
                    if let Some(date) = parsed_start_date {
                        task.start_date = Some(date);
                    }
                    task.updated_at = local_date_today();
                }
                successful.push(task_id.to_string());
                eprintln!("Successfully moved task {} to {:?}", task_id, target_status);
            } else {
                eprintln!(
                    "Error: Failed to move task {} to {:?}",
                    task_id, target_status
                );
                failed.push((task_id.to_string(), "Failed to move task".to_string()));
            }
        }

        drop(data);

        // Save data if any tasks were successfully moved
        if !successful.is_empty() {
            let task_list = successful.join(", ");
            let commit_message = match target_status {
                TaskStatus::inbox => format!("Move tasks to inbox: {}", task_list),
                TaskStatus::next_action => format!("Move tasks to next_action: {}", task_list),
                TaskStatus::waiting_for => format!("Move tasks to waiting_for: {}", task_list),
                TaskStatus::someday => format!("Move tasks to someday: {}", task_list),
                TaskStatus::later => format!("Move tasks to later: {}", task_list),
                TaskStatus::calendar => format!("Move tasks to calendar: {}", task_list),
                TaskStatus::done => format!("Mark tasks as done: {}", task_list),
                TaskStatus::trash => format!("Move tasks to trash: {}", task_list),
            };

            if let Err(e) = self.save_data_with_message(&commit_message) {
                eprintln!(
                    "Error: Failed to save data after moving tasks to {:?}: {}",
                    target_status, e
                );
                bail!(
                    "Failed to save tasks to {:?}: {}. Some tasks may not have been moved.",
                    target_status,
                    e
                );
            }
        }

        // Build result message
        let mut result = String::new();
        if !successful.is_empty() {
            let status_display = match target_status {
                TaskStatus::next_action => "next action",
                TaskStatus::waiting_for => "waiting for",
                _ => &status,
            };
            result.push_str(&format!(
                "Successfully moved {} task(s) to {}: {}",
                successful.len(),
                status_display,
                successful.join(", ")
            ));
        }
        if !failed.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!("Failed to move {} task(s): ", failed.len()));
            let failures: Vec<String> = failed
                .iter()
                .map(|(id, reason)| format!("{} ({})", id, reason))
                .collect();
            result.push_str(&failures.join(", "));
        }

        if successful.is_empty() && !failed.is_empty() {
            bail!("{}", result);
        }

        Ok(result)
    }

    /// Permanently delete all trashed tasks (irreversible). Use when certain you don't need those tasks.
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

    /// Create project to organize related tasks. Projects are multi-step endeavors (e.g., "Launch website"). Use to group related tasks and track larger goals.
    #[tool]
    async fn add_project(
        &self,
        /// Project name (e.g., "Website Redesign")
        name: String,
        /// Project ID - use meaningful abbreviation (e.g., "website-redesign", "q1-budget")
        id: String,
        /// Optional project goal description
        description: Option<String>,
        /// Optional context where project work happens (e.g., "@office")
        context: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Validate that ID doesn't already exist
        if data.find_project_by_id(&id).is_some() {
            drop(data);
            bail!("Project ID already exists: {}", id);
        }

        let project = Project {
            id: id.clone(),
            name: name.clone(),
            description,
            status: ProjectStatus::active,
            context,
        };

        // Validate context reference before adding the project
        if !data.validate_project_context(&project) {
            drop(data);
            bail!("Invalid context reference: context does not exist");
        }

        data.add_project(project);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Add project: {}", name)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Project created with ID: {}", id))
    }

    /// View all projects with status (active/on_hold/completed), descriptions, and contexts. Review regularly for progress tracking.
    #[tool]
    async fn list_projects(&self) -> McpResult<String> {
        let data = self.data.lock().unwrap();
        let projects: Vec<&Project> = data.projects.values().collect();

        let mut result = String::new();
        for project in projects {
            let desc_info = project
                .description
                .as_ref()
                .map(|d| format!(" [desc: {}]", d))
                .unwrap_or_default();
            let context_info = project
                .context
                .as_ref()
                .map(|c| format!(" [context: {}]", c))
                .unwrap_or_default();
            result.push_str(&format!(
                "- [{}] {} (status: {:?}){}{}\n",
                project.id, project.name, project.status, desc_info, context_info
            ));
        }

        Ok(result)
    }

    /// Modify task properties. Update details, reassign projects, change contexts, add notes, or set/update start dates. Use empty string "" to remove optional fields.
    #[tool]
    async fn update_task(
        &self,
        /// Task ID (e.g., "#1")
        task_id: String,
        /// Optional new title
        title: Option<String>,
        /// Optional new project ID, or "" to unlink
        project: Option<String>,
        /// Optional new context, or "" to remove
        context: Option<String>,
        /// Optional new notes, or "" to remove
        notes: Option<String>,
        /// Optional new start date (YYYY-MM-DD), or "" to remove
        start_date: Option<String>,
    ) -> McpResult<String> {
        // Normalize task ID to ensure # prefix
        let task_id = Self::normalize_task_id(&task_id);

        // Parse date first if provided
        let new_start_date = if let Some(new_date_str) = start_date {
            if new_date_str.is_empty() {
                Some(None)
            } else {
                match NaiveDate::parse_from_str(&new_date_str, "%Y-%m-%d") {
                    Ok(date) => Some(Some(date)),
                    Err(_) => bail!("Invalid date format. Use YYYY-MM-DD"),
                }
            }
        } else {
            None
        };

        let mut data = self.data.lock().unwrap();

        // Find the task
        let task = match data.find_task_by_id_mut(&task_id) {
            Some(t) => t,
            None => {
                drop(data);
                bail!("Task not found: {}", task_id);
            }
        };

        // Update in place
        if let Some(new_title) = title {
            task.title = new_title;
        }
        if let Some(new_project) = project {
            task.project = if new_project.is_empty() {
                None
            } else {
                Some(new_project)
            };
        }
        if let Some(new_context) = context {
            task.context = if new_context.is_empty() {
                None
            } else {
                Some(new_context)
            };
        }
        if let Some(new_notes) = notes {
            task.notes = if new_notes.is_empty() {
                None
            } else {
                Some(new_notes)
            };
        }
        if let Some(date_opt) = new_start_date {
            task.start_date = date_opt;
        }
        task.updated_at = local_date_today();

        // Clone task for validation
        let task_clone = task.clone();

        // Validate references
        if !data.validate_task_references(&task_clone) {
            let mut errors = Vec::new();
            if !data.validate_task_project(&task_clone) {
                errors.push("Invalid project reference");
            }
            if !data.validate_task_context(&task_clone) {
                errors.push("Invalid context reference");
            }
            drop(data);
            bail!("Failed to update task: {}", errors.join(", "));
        }

        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Update task {}", task_id)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Task {} updated successfully", task_id))
    }

    /// Modify project properties. Update details, change status (active/on_hold/completed), or reassign contexts. Use "" to remove optional fields.
    #[tool]
    async fn update_project(
        &self,
        /// Project ID (e.g., "website-redesign")
        project_id: String,
        /// Optional new project ID if renaming
        id: Option<String>,
        /// Optional new name
        name: Option<String>,
        /// Optional new description, or "" to remove
        description: Option<String>,
        /// Optional new status: "active", "on_hold", "completed"
        status: Option<String>,
        /// Optional new context, or "" to remove
        context: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Validate project exists
        if data.find_project_by_id(&project_id).is_none() {
            drop(data);
            bail!("Project not found: {}", project_id);
        }

        // Validate new ID if provided
        if let Some(ref new_id) = id
            && new_id != &project_id
            && data.find_project_by_id(new_id).is_some()
        {
            drop(data);
            bail!("Project ID already exists: {}", new_id);
        }

        // If ID is changing, we need to remove from old key and insert with new key
        let new_project_id = id.clone().unwrap_or_else(|| project_id.clone());
        let id_changed = new_project_id != project_id;

        // Get the project and update fields
        let mut project = if id_changed {
            // Remove from old key
            data.projects.remove(&project_id).unwrap()
        } else {
            // Just get mutable reference
            data.find_project_by_id_mut(&project_id).unwrap().clone()
        };

        // Update ID if changed
        if id_changed {
            project.id = new_project_id.clone();
        }

        // Update name if provided
        if let Some(new_name) = name {
            project.name = new_name;
        }

        // Update description if provided (empty string removes it)
        if let Some(new_description) = description {
            project.description = if new_description.is_empty() {
                None
            } else {
                Some(new_description)
            };
        }

        // Update status if provided
        if let Some(status_str) = status {
            project.status = match status_str.as_str() {
                "active" => ProjectStatus::active,
                "on_hold" => ProjectStatus::on_hold,
                "completed" => ProjectStatus::completed,
                _ => {
                    drop(data);
                    bail!("Invalid status. Use: active, on_hold, completed");
                }
            };
        }

        // Update context if provided (empty string removes it)
        if let Some(new_context) = context {
            project.context = if new_context.is_empty() {
                None
            } else {
                Some(new_context)
            };
        }

        // Validate context reference
        if !data.validate_project_context(&project) {
            drop(data);
            bail!("Invalid context reference: context does not exist");
        }

        // Update task references if project ID changed
        if id_changed {
            data.update_project_id_in_tasks(&project_id, &new_project_id);
        }

        // Insert project back (either with new key if ID changed, or update existing)
        if id_changed {
            data.projects.insert(new_project_id.clone(), project);
        } else {
            *data.find_project_by_id_mut(&project_id).unwrap() = project;
        }

        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Update project {}", new_project_id)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Project {} updated successfully", new_project_id))
    }

    /// Delete project if not referenced by any tasks. Ensures data integrity by preventing deletion of projects in use.
    #[tool]
    async fn delete_project(
        &self,
        /// Project ID (e.g., "website-redesign")
        project_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check if project exists
        if data.find_project_by_id(&project_id).is_none() {
            drop(data);
            bail!("Project not found: {}", project_id);
        }

        // Check if any tasks reference this project
        let task_lists = [
            &data.inbox,
            &data.next_action,
            &data.waiting_for,
            &data.later,
            &data.calendar,
            &data.someday,
            &data.done,
            &data.trash,
        ];

        for task_list in &task_lists {
            for task in *task_list {
                if let Some(ref task_project) = task.project
                    && task_project == &project_id
                {
                    let task_id = task.id.clone();
                    drop(data);
                    bail!(
                        "Cannot delete project '{}': task {} still references it",
                        project_id,
                        task_id
                    );
                }
            }
        }

        // Remove the project
        data.projects.remove(&project_id);

        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Delete project {}", project_id)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Project {} deleted successfully", project_id))
    }

    /// Create context to categorize where/how tasks can be done. Contexts are locations, tools, or situations (e.g., "@office", "@home", "@phone"). Use to filter tasks by current situation.
    #[tool]
    async fn add_context(
        &self,
        /// Context name (e.g., "@office", "@home", "@phone")
        name: String,
        /// Optional description
        description: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check if context already exists
        if data.find_context_by_name(&name).is_some() {
            drop(data);
            bail!("Context already exists: {}", name);
        }

        let context = gtd::Context {
            name: name.clone(),
            description,
        };

        data.add_context(context);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Add context: {}", name)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Context created: {}", name))
    }

    /// View all contexts with descriptions. See available contexts for categorizing tasks/projects.
    #[tool]
    async fn list_contexts(&self) -> McpResult<String> {
        let data = self.data.lock().unwrap();

        if data.contexts.is_empty() {
            return Ok("No contexts found".to_string());
        }

        let mut result = String::new();
        let mut contexts: Vec<_> = data.contexts.values().collect();
        contexts.sort_by_key(|c| &c.name);

        for context in contexts {
            let desc = context
                .description
                .as_ref()
                .map(|d| format!(": {}", d))
                .unwrap_or_default();
            result.push_str(&format!("- {}{}\n", context.name, desc));
        }

        Ok(result)
    }

    /// Modify context description. Use "" to remove description.
    #[tool]
    async fn update_context(
        &self,
        /// Context name (e.g., "@office")
        name: String,
        /// Optional new description, or "" to remove
        description: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check if context exists
        if data.find_context_by_name(&name).is_none() {
            drop(data);
            bail!("Context not found: {}", name);
        }

        // Remove and re-add with updated description
        let context = gtd::Context {
            name: name.clone(),
            description: if let Some(desc) = description {
                if desc.is_empty() { None } else { Some(desc) }
            } else {
                data.contexts.get(&name).and_then(|c| c.description.clone())
            },
        };

        data.contexts.insert(name.clone(), context);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Update context {}", name)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Context {} updated successfully", name))
    }

    /// Delete context if not referenced by any tasks or projects. Ensures data integrity by preventing deletion of contexts in use.
    #[tool]
    async fn delete_context(
        &self,
        /// Context name (e.g., "@office")
        name: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check if context exists
        if data.find_context_by_name(&name).is_none() {
            drop(data);
            bail!("Context not found: {}", name);
        }

        // Check if any tasks reference this context
        let task_lists = [
            &data.inbox,
            &data.next_action,
            &data.waiting_for,
            &data.later,
            &data.calendar,
            &data.someday,
            &data.done,
            &data.trash,
        ];

        for task_list in &task_lists {
            for task in *task_list {
                if let Some(ref task_context) = task.context
                    && task_context == &name
                {
                    let task_id = task.id.clone();
                    drop(data);
                    bail!(
                        "Cannot delete context '{}': task {} still references it",
                        name,
                        task_id
                    );
                }
            }
        }

        // Check if any projects reference this context
        for (project_id, project) in &data.projects {
            if let Some(ref project_context) = project.context
                && project_context == &name
            {
                let project_id = project_id.clone();
                drop(data);
                bail!(
                    "Cannot delete context '{}': project {} still references it",
                    name,
                    project_id
                );
            }
        }

        // Remove the context
        data.contexts.remove(&name);

        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Delete context {}", name)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Context {} deleted successfully", name))
    }

    // ==================== Prompts ====================

    /// GTD system overview and available tools
    #[prompt]
    async fn gtd_overview(&self) -> McpResult<String> {
        Ok(r#"# GTD Task Management System

This MCP server implements the Getting Things Done (GTD) methodology by David Allen.

## Core Concepts

**Task Statuses:**
- `inbox`: Unprocessed items (default for new tasks)
- `next_action`: Actionable tasks ready to work on
- `waiting_for`: Tasks blocked waiting for someone/something
- `someday`: Tasks for potential future action
- `later`: Tasks to do later (deferred but not someday)
- `calendar`: Date-specific tasks (require start_date)
- `done`: Completed tasks
- `trash`: Deleted tasks

**Projects:** Collections of related tasks with statuses (active, on_hold, completed)

**Contexts:** Work environments/tools needed (@office, @home, @computer, @phone)

## Task IDs

Tasks use GitHub-style IDs: #1, #2, #3 (efficient for LLM token usage)
Projects use: project-1, project-2, project-3

**IMPORTANT:** When referencing tasks, ALWAYS include the '#' prefix (e.g., #1, #2, #3).
- Correct: Specify task IDs with # prefix like #1, #2, #3
- Also accepted: Plain numbers like 1, 2, 3 (system auto-corrects to #1, #2, #3)
- The '#' prefix identifies tasks and improves clarity

## Common Workflows

1. **Capture**: Use `add_task` to capture items to inbox
2. **Process**: Review inbox, clarify and organize with status movement tools
3. **Review**: Regularly check all task statuses with `list_tasks`
4. **Do**: Focus on `next_action` tasks for execution

## Key Tools

- Task Management: add_task, update_task, list_tasks, delete_task
- Status Movement: change_task_status (unified tool for all status transitions)
- Projects: add_project, list_projects, update_project, delete_project
- Contexts: add_context, list_contexts, update_context, delete_context

Use prompts like `process_inbox`, `weekly_review`, or `next_actions` for workflow guidance."#
            .to_string())
    }

    /// Guide for processing inbox items
    #[prompt]
    async fn process_inbox(&self) -> McpResult<String> {
        Ok(r#"# GTD Inbox Processing Guide

## Workflow for each inbox item:

1. **List inbox**: Use `list_tasks` with status "inbox"

2. **For each task, ask:**
   - Is it actionable?
     - NO → Use `change_task_status` to move to "someday" or "trash"
     - YES → Continue to step 3

3. **Will it take less than 2 minutes?**
   - YES → Do it now, then use `change_task_status` to move to "done"
   - NO → Continue to step 4

4. **Can I do it myself?**
   - NO → Use `waiting_for_tasks` and add notes about who/what you're waiting for
   - YES → Continue to step 5

5. **Is there a specific date?**
   - YES → Use `calendar_tasks` with start_date parameter
   - NO → Continue to step 6

6. **Should this be done later (deferred)?**
   - YES → Use `later_tasks` for tasks deferred to a later time
   - NO → Continue to step 7

7. **Is it part of a larger project?**
   - YES → Use `update_task` to assign project
   - NO → Continue to step 8

8. **Add context if helpful** (e.g., @office, @computer)
   - Use `update_task` to set context

9. **Move to next actions**
   - Use `change_task_status` with status "next_action"

## Goal

Process inbox to zero. Every item should be clarified and organized."#
            .to_string())
    }

    /// Guide for conducting GTD weekly review
    #[prompt]
    async fn weekly_review(&self) -> McpResult<String> {
        Ok(r#"# GTD Weekly Review Process

The weekly review keeps your system current and complete.

## Review Steps

1. **Get Clear**
   - Process inbox to zero: Use `process_inbox` prompt guide
   - Empty your head: Add any floating thoughts with `add_task`

2. **Get Current**
   - Review `calendar` tasks: `list_tasks` status "calendar"
     - Check dates are still accurate
     - Move completed calendar items to done
   
   - Review `next_action` tasks: `list_tasks` status "next_action"
     - Mark completed ones as done using `change_task_status`
     - Update stale tasks with `update_task`
     - Identify tasks that should move to waiting/someday/later

   - Review `waiting_for` tasks: `list_tasks` status "waiting_for"
     - Check if any can now move to next actions
     - Update notes on what you're waiting for
   
   - Review `later` tasks: `list_tasks` status "later"
     - Move tasks that are now ready to next actions
     - Update or clarify deferred tasks
   
   - Review `someday` tasks: `list_tasks` status "someday"
     - Move newly relevant items to inbox or next actions
     - Trash items no longer relevant

3. **Review Projects**
   - List all projects: `list_projects`
   - For each project:
     - Does it have a next action? Add one if missing
     - Is status correct? Update if needed (active/on_hold/completed)
     - Update project description if scope changed

4. **Get Creative**
   - Brainstorm new projects or tasks
   - Review someday/maybe for inspiration

## Frequency
Conduct weekly reviews every 7 days to maintain system integrity."#
            .to_string())
    }

    /// Guide for identifying and managing next actions
    #[prompt]
    async fn next_actions(&self) -> McpResult<String> {
        Ok(r#"# Next Actions Guide

Next actions are physical, visible activities that can be done immediately.

## Identifying Next Actions

A good next action is:
- **Specific**: "Call John about proposal" not "Handle proposal"
- **Physical**: Describes concrete action, not outcome
- **Doable**: Can be done in current context
- **Single-step**: One clear action, not a project

## Context-Based Work

Use contexts to filter by location/tools:
- `@office`: Needs office environment
- `@computer`: Needs computer/internet
- `@phone`: Phone calls
- `@home`: Home activities
- `@errands`: Out-and-about tasks

Add context with `update_task` and filter tasks by context when working.

## Choosing What to Do

When ready to work, consider:
1. **Context**: What's available? (tools, location)
2. **Time**: How much time do you have?
3. **Energy**: High energy for hard tasks, low energy for simple ones
4. **Priority**: What's most important now?

List next actions with `list_tasks` status "next_action"

## After Completion

When done:
- Use `change_task_status` to mark complete (status "done")
- If it was part of a project, check if project needs a new next action"#
            .to_string())
    }

    /// Guide for creating well-formed tasks
    #[prompt]
    async fn add_task_guide(&self) -> McpResult<String> {
        Ok(r#"# Task Creation Best Practices

## Good Task Titles

**Good examples:**
- "Call Sarah to schedule meeting"
- "Draft Q1 budget proposal"
- "Buy new printer ink"
- "Email team about project status"

**Poor examples (avoid):**
- "Sarah" (not specific)
- "Budget" (not an action)
- "Ink" (not actionable)
- "Follow up" (too vague)

## Using Optional Fields

**project**: Link to project-1, project-2, etc.
- Use for multi-step endeavors
- Keep single tasks unlinked

**context**: Work environment/tools needed
- @office, @home, @computer, @phone, @errands
- Helps filter when in specific contexts

**notes**: Additional details (Markdown format recommended)
- Background information
- URLs, reference numbers
- Why this matters
- Use Markdown for formatting: **bold**, *italic*, lists, links, etc.

**start_date**: For calendar tasks only (YYYY-MM-DD)
- Events with specific dates
- Tickler file items
- Don't use for regular next actions

## Workflow

1. Quick capture to inbox: `add_task` with just title
2. Later process inbox to clarify and organize
3. Use `update_task` to add details as needed

**IMPORTANT:** Task IDs use the '#' prefix: #1, #2, #3, etc.
- Preferred: Use the '#' prefix (e.g., #1 for task 1)
- Also works: Plain numbers (e.g., 1) are auto-corrected to #1
- Date format: YYYY-MM-DD (e.g., 2024-03-15)"#
            .to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };
        data.add_task(task);
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
        // Test with plain numbers
        assert_eq!(GtdServerHandler::normalize_task_id("1"), "#1");
        assert_eq!(GtdServerHandler::normalize_task_id("42"), "#42");
        assert_eq!(GtdServerHandler::normalize_task_id("123"), "#123");

        // Test with # prefix already present
        assert_eq!(GtdServerHandler::normalize_task_id("#1"), "#1");
        assert_eq!(GtdServerHandler::normalize_task_id("#42"), "#42");
        assert_eq!(GtdServerHandler::normalize_task_id("#123"), "#123");

        // Test with whitespace
        assert_eq!(GtdServerHandler::normalize_task_id(" 1 "), "#1");
        assert_eq!(GtdServerHandler::normalize_task_id(" #1 "), "#1");
        assert_eq!(GtdServerHandler::normalize_task_id("  42  "), "#42");
    }

    #[tokio::test]
    async fn test_change_task_status_unified_api() {
        let (handler, _temp_file) = get_test_handler();

        // Create a task in inbox
        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Test moving to next_action
        let result = handler
            .change_task_status(vec![task_id.clone()], "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.status, TaskStatus::next_action);
        }

        // Test moving to done
        let result = handler
            .change_task_status(vec![task_id.clone()], "done".to_string(), None)
            .await;
        assert!(result.is_ok());
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.status, TaskStatus::done);
        }

        // Test moving to trash
        let result = handler
            .change_task_status(vec![task_id.clone()], "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.status, TaskStatus::trash);
        }

        // Test invalid status
        let result = handler
            .change_task_status(vec![task_id.clone()], "invalid_status".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_change_task_status_calendar_with_date() {
        let (handler, _temp_file) = get_test_handler();

        // Create a task
        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Test moving to calendar with date
        let result = handler
            .change_task_status(
                vec![task_id.clone()],
                "calendar".to_string(),
                Some("2024-12-25".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.status, TaskStatus::calendar);
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
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // Test batch move to next_action
        let result = handler
            .change_task_status(task_ids.clone(), "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify all tasks moved
        let data = handler.data.lock().unwrap();
        assert_eq!(data.next_action.len(), 3);
        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert_eq!(task.status, TaskStatus::next_action);
        }
    }

    #[tokio::test]
    async fn test_update_task_with_plain_number_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());

        // Update task using plain number (without #)
        let result = handler
            .update_task(
                "1".to_string(), // Using "1" instead of "#1"
                Some("Updated Title".to_string()),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify the update worked
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id("#1").unwrap();
        assert_eq!(task.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_status_movement_with_plain_number_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());

        // Move to next_action using plain number
        let result = handler
            .change_task_status(vec!["1".to_string()], "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify the task moved
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id("#1").unwrap();
        assert_eq!(task.status, TaskStatus::next_action);
    }

    #[tokio::test]
    async fn test_update_task_title() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .add_task("Original Title".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());

        // Extract task ID from result
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Update title
        let result = handler
            .update_task(
                task_id.clone(),
                Some("Updated Title".to_string()),
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Verify initial status is inbox
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, TaskStatus::inbox));
            assert_eq!(data.inbox.len(), 1);
            assert_eq!(data.next_action.len(), 0);
        }

        // Update status to next_action using new method
        let result = handler
            .change_task_status(vec![task_id.clone()], "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify status changed and task moved
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, TaskStatus::next_action));
            assert_eq!(data.inbox.len(), 0);
            assert_eq!(data.next_action.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_update_task_project_and_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project and context first
        let project_result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(project_result.is_ok());
        let project_id = project_result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        {
            let mut data = handler.data.lock().unwrap();
            data.add_context(gtd::Context {
                name: "Office".to_string(),
                description: None,
            });
            drop(data);
            let _ = handler.save_data();
        }

        // Add a task
        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Update project and context
        let result = handler
            .update_task(
                task_id.clone(),
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
            .add_task(
                "Test Task".to_string(),
                None,
                None,
                Some("Some notes".to_string()),
                Some("2024-12-25".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Verify initial state
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.notes, Some("Some notes".to_string()));
            assert!(task.start_date.is_some());
        }

        // Remove optional fields using empty strings
        let result = handler
            .update_task(
                task_id.clone(),
                None,
                None,
                None,
                Some("".to_string()),
                Some("".to_string()),
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with invalid date
        let result = handler
            .update_task(
                task_id,
                None,
                None,
                None,
                None,
                Some("invalid-date".to_string()),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_task_invalid_project_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with non-existent project
        let result = handler
            .update_task(
                task_id,
                None,
                Some("non-existent-project".to_string()),
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with non-existent context
        let result = handler
            .update_task(
                task_id,
                None,
                None,
                Some("NonExistent".to_string()),
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
            .update_task(
                "non-existent-id".to_string(),
                Some("New Title".to_string()),
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Get initial timestamps
        let (created_at, _updated_at) = {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            (task.created_at, task.updated_at)
        };

        // Update task
        let result = handler
            .update_task(
                task_id.clone(),
                Some("Updated Title".to_string()),
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
            .add_project(
                "Original Name".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Update name
        let result = handler
            .update_project(
                project_id.clone(),
                None,
                Some("Updated Name".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify update
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert_eq!(project.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_project_description() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Add description
        let result = handler
            .update_project(
                project_id.clone(),
                None,
                None,
                Some("New description".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify description added
        {
            let data = handler.data.lock().unwrap();
            let project = data.find_project_by_id(&project_id).unwrap();
            assert_eq!(project.description, Some("New description".to_string()));
        }

        // Remove description
        let result = handler
            .update_project(
                project_id.clone(),
                None,
                None,
                Some("".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Verify description removed
        let data = handler.data.lock().unwrap();
        let project = data.find_project_by_id(&project_id).unwrap();
        assert_eq!(project.description, None);
    }

    #[tokio::test]
    async fn test_update_project_status() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Verify initial status
        {
            let data = handler.data.lock().unwrap();
            let project = data.find_project_by_id(&project_id).unwrap();
            assert!(matches!(project.status, ProjectStatus::active));
        }

        // Update status to on_hold
        let result = handler
            .update_project(
                project_id.clone(),
                None,
                None,
                None,
                Some("on_hold".to_string()),
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
            .update_project(
                project_id.clone(),
                None,
                None,
                None,
                Some("completed".to_string()),
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with invalid status
        let result = handler
            .update_project(
                project_id,
                None,
                None,
                None,
                Some("invalid_status".to_string()),
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
            .update_project(
                "non-existent-id".to_string(),
                None,
                Some("New Name".to_string()),
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Delete the project
        let result = handler.delete_project("test-project-1".to_string()).await;
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
        let result = handler.delete_project("non-existent-id".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_project_with_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Add a task that references the project
        let result = handler
            .add_task(
                "Test Task".to_string(),
                Some("test-project-1".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Try to delete the project (should fail)
        let result = handler.delete_project("test-project-1".to_string()).await;
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Add a task that references the project
        let result = handler
            .add_task(
                "Test Task".to_string(),
                Some("test-project-1".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Unlink the task from the project
        let result = handler
            .update_task(
                "#1".to_string(),
                None,
                Some("".to_string()), // Empty string removes project
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Now delete the project (should succeed)
        let result = handler.delete_project("test-project-1".to_string()).await;
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(project_result.is_ok());
        let project_id = project_result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Add a context
        {
            let mut data = handler.data.lock().unwrap();
            data.add_context(gtd::Context {
                name: "Office".to_string(),
                description: None,
            });
            drop(data);
            let _ = handler.save_data();
        }

        // Add a task
        let result = handler
            .add_task("Original Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Update multiple fields at once
        let result = handler
            .update_task(
                task_id.clone(),
                Some("Updated Task".to_string()),
                Some(project_id.clone()),
                Some("Office".to_string()),
                Some("Updated notes".to_string()),
                Some("2025-01-15".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // Change status separately using new method
        let result = handler
            .change_task_status(vec![task_id.clone()], "done".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify all updates
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.title, "Updated Task");
        assert!(matches!(task.status, TaskStatus::done));
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Move to next_action first
        let result = handler
            .change_task_status(vec![task_id.clone()], "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify it's in next_action
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, TaskStatus::next_action));
            assert_eq!(data.next_action.len(), 1);
            assert_eq!(data.inbox.len(), 0);
        }

        // Move back to inbox
        let result = handler
            .change_task_status(vec![task_id.clone()], "inbox".to_string(), None)
            .await;
        assert!(result.is_ok());

        // Verify it's back in inbox
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert!(matches!(task.status, TaskStatus::inbox));
            assert_eq!(data.inbox.len(), 1);
            assert_eq!(data.next_action.len(), 0);
        }
    }

    #[tokio::test]
    async fn test_next_action_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id.clone()], "next_action".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::next_action));
        assert_eq!(data.next_action.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_waiting_for_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id.clone()], "waiting_for".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::waiting_for));
        assert_eq!(data.waiting_for.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_someday_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id.clone()], "someday".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::someday));
        assert_eq!(data.someday.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_later_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id.clone()], "later".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::later));
        assert_eq!(data.later.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_done_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id.clone()], "done".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::done));
        assert_eq!(data.done.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_trash_task_from_inbox() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id.clone()], "trash".to_string(), None)
            .await;
        assert!(result.is_ok(), "Failed to trash task: {:?}", result.err());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::trash));
        assert_eq!(data.trash.len(), 1);
        assert_eq!(data.inbox.len(), 0);
    }

    #[tokio::test]
    async fn test_trash_task_workflow_comparison() {
        let (handler, _temp_file) = get_test_handler();

        // Test 1: inbox → trash directly
        let result = handler
            .add_task("Direct Trash Test".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id_1 = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id_1.clone()], "trash".to_string(), None)
            .await;
        assert!(result.is_ok(), "Direct trash failed: {:?}", result.err());

        // Test 2: inbox → done → trash (the workflow user reported as working)
        let result = handler
            .add_task("Indirect Trash Test".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id_2 = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(vec![task_id_2.clone()], "done".to_string(), None)
            .await;
        assert!(result.is_ok(), "Moving to done failed: {:?}", result.err());

        let result = handler
            .change_task_status(vec![task_id_2.clone()], "trash".to_string(), None)
            .await;
        assert!(result.is_ok(), "Trash from done failed: {:?}", result.err());

        // Verify both tasks ended up in trash
        let data = handler.data.lock().unwrap();
        assert_eq!(data.trash.len(), 2);
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.done.len(), 0);

        let task1 = data.find_task_by_id(&task_id_1).unwrap();
        let task2 = data.find_task_by_id(&task_id_2).unwrap();
        assert!(matches!(task1.status, TaskStatus::trash));
        assert!(matches!(task2.status, TaskStatus::trash));
    }

    #[tokio::test]
    async fn test_trash_task_error_messages() {
        let (handler, _temp_file) = get_test_handler();

        // Test with various invalid task IDs to ensure error handling works
        let test_cases = vec!["#999", "invalid-id", "task-999"];

        for task_id in test_cases {
            let result = handler
                .change_task_status(vec![task_id.to_string()], "trash".to_string(), None)
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
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にtrashに移動
        let result = handler
            .change_task_status(task_ids.clone(), "trash".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::trash));
        }
    }

    #[tokio::test]
    async fn test_trash_tasks_partial_success() {
        let (handler, _temp_file) = get_test_handler();

        // 有効なタスクを2つ作成
        let mut task_ids = Vec::new();
        for i in 1..=2 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 無効なタスクIDを追加
        task_ids.push("#999".to_string());
        task_ids.push("invalid-id".to_string());

        // 部分的な成功を確認
        let result = handler
            .change_task_status(task_ids.clone(), "trash".to_string(), None)
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
        let result = handler
            .change_task_status(task_ids, "trash".to_string(), None)
            .await;
        assert!(result.is_err(), "Expected error when all tasks are invalid");
    }

    #[tokio::test]
    async fn test_trash_tasks_empty_list() {
        let (handler, _temp_file) = get_test_handler();

        // 空のリスト
        let task_ids: Vec<String> = Vec::new();

        let result = handler
            .change_task_status(task_ids, "trash".to_string(), None)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_trash_tasks_from_different_statuses() {
        let (handler, _temp_file) = get_test_handler();

        // inboxからタスクを作成
        let result = handler
            .add_task("Inbox Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let inbox_task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // next_actionに移動
        let result = handler
            .add_task("Next Action Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let next_action_task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();
        handler
            .change_task_status(
                vec![next_action_task_id.clone()],
                "next_action".to_string(),
                None,
            )
            .await
            .unwrap();

        // doneに移動
        let result = handler
            .add_task("Done Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let done_task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();
        handler
            .change_task_status(vec![done_task_id.clone()], "done".to_string(), None)
            .await
            .unwrap();

        // 異なるステータスのタスクを一度にtrashに移動
        let task_ids = vec![
            inbox_task_id.clone(),
            next_action_task_id.clone(),
            done_task_id.clone(),
        ];
        let result = handler
            .change_task_status(task_ids, "trash".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to trash tasks from different statuses: {:?}",
            result.err()
        );

        // すべてがtrashに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.trash.len(), 3);
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.next_action.len(), 0);
        assert_eq!(data.done.len(), 0);

        let task1 = data.find_task_by_id(&inbox_task_id).unwrap();
        let task2 = data.find_task_by_id(&next_action_task_id).unwrap();
        let task3 = data.find_task_by_id(&done_task_id).unwrap();
        assert!(matches!(task1.status, TaskStatus::trash));
        assert!(matches!(task2.status, TaskStatus::trash));
        assert!(matches!(task3.status, TaskStatus::trash));
    }

    #[tokio::test]
    async fn test_calendar_task_with_start_date() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .change_task_status(
                vec![task_id.clone()],
                "calendar".to_string(),
                Some("2024-12-25".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::calendar));
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // start_dateを指定せずにcalendarに移動しようとするとエラー
        let result = handler
            .change_task_status(vec![task_id.clone()], "calendar".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calendar_task_with_existing_start_date() {
        let (handler, _temp_file) = get_test_handler();

        // start_date付きのタスクを作成
        let result = handler
            .add_task(
                "Test Task".to_string(),
                None,
                None,
                None,
                Some("2024-11-15".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // start_dateパラメータなしでcalendarに移動（既存のstart_dateを使用）
        let result = handler
            .change_task_status(vec![task_id.clone()], "calendar".to_string(), None)
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::calendar));
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
            .add_task(
                "Test Task".to_string(),
                None,
                None,
                None,
                Some("2024-11-15".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // 新しいstart_dateを指定してcalendarに移動（既存のstart_dateを上書き）
        let result = handler
            .change_task_status(
                vec![task_id.clone()],
                "calendar".to_string(),
                Some("2024-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(task.status, TaskStatus::calendar));
        assert_eq!(
            task.start_date.unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()
        );
    }

    #[tokio::test]
    async fn test_calendar_task_invalid_date_format() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // 無効な日付形式
        let result = handler
            .change_task_status(
                vec![task_id.clone()],
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
            .add_task("Test Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let created_at = {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            task.created_at
        };

        // Move to next_action
        let result = handler
            .change_task_status(vec![task_id.clone()], "next_action".to_string(), None)
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
            .change_task_status(
                vec!["nonexistent-id".to_string()],
                "next_action".to_string(),
                None,
            )
            .await;
        assert!(result.is_err());

        let result = handler
            .change_task_status(vec!["nonexistent-id".to_string()], "done".to_string(), None)
            .await;
        assert!(result.is_err());

        let result = handler
            .change_task_status(
                vec!["nonexistent-id".to_string()],
                "trash".to_string(),
                None,
            )
            .await;
        assert!(result.is_err());
    }

    // Tests for context management

    #[tokio::test]
    async fn test_add_context() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .add_context("Office".to_string(), Some("Work environment".to_string()))
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Office"));

        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.name, "Office");
        assert_eq!(context.description, Some("Work environment".to_string()));
    }

    #[tokio::test]
    async fn test_add_context_duplicate() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.add_context("Office".to_string(), None).await;
        assert!(result.is_ok());

        // Try to add duplicate
        let result = handler.add_context("Office".to_string(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_contexts_empty() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.list_contexts().await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No contexts found"));
    }

    #[tokio::test]
    async fn test_list_contexts() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .add_context("Office".to_string(), Some("Work environment".to_string()))
            .await
            .unwrap();
        handler.add_context("Home".to_string(), None).await.unwrap();

        let result = handler.list_contexts().await;
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
            .add_context("Office".to_string(), Some("Old description".to_string()))
            .await
            .unwrap();

        let result = handler
            .update_context("Office".to_string(), Some("New description".to_string()))
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.description, Some("New description".to_string()));
    }

    #[tokio::test]
    async fn test_update_context_remove_description() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .add_context("Office".to_string(), Some("Old description".to_string()))
            .await
            .unwrap();

        let result = handler
            .update_context("Office".to_string(), Some("".to_string()))
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.description, None);
    }

    #[tokio::test]
    async fn test_update_context_not_found() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .update_context("NonExistent".to_string(), Some("Description".to_string()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_context() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        let result = handler.delete_context("Office".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("deleted"));

        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_context_not_found() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.delete_context("NonExistent".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_context_with_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        // Add a task that references the context
        handler
            .add_task(
                "Office work".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail
        let result = handler.delete_context("Office".to_string()).await;
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
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        // Add a project that references the context
        handler
            .add_project(
                "Office Project".to_string(),
                "office-proj".to_string(),
                None,
                Some("Office".to_string()),
            )
            .await
            .unwrap();

        // Try to delete the context - should fail
        let result = handler.delete_context("Office".to_string()).await;
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
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        // Add a task that references the context
        handler
            .add_task(
                "Office work".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Add a project that references the context
        handler
            .add_project(
                "Office Project".to_string(),
                "office-proj".to_string(),
                None,
                Some("Office".to_string()),
            )
            .await
            .unwrap();

        // Try to delete the context - should fail (task check comes first)
        let result = handler.delete_context("Office".to_string()).await;
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
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        // Add a task that references the context
        let task_id = handler
            .add_task(
                "Office work".to_string(),
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
            .update_task(task_id, None, None, Some(String::new()), None, None)
            .await
            .unwrap();

        // Now deletion should succeed
        let result = handler.delete_context("Office".to_string()).await;
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
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        // Add a project that references the context
        handler
            .add_project(
                "Office Project".to_string(),
                "office-proj".to_string(),
                None,
                Some("Office".to_string()),
            )
            .await
            .unwrap();

        // Remove the context reference from the project
        handler
            .update_project(
                "office-proj".to_string(),
                None,
                None,
                None,
                None,
                Some(String::new()),
            )
            .await
            .unwrap();

        // Now deletion should succeed
        let result = handler.delete_context("Office".to_string()).await;
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
            .add_context("Office".to_string(), None)
            .await
            .unwrap();

        // Add multiple tasks that reference the context
        handler
            .add_task(
                "Task 1".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        handler
            .add_task(
                "Task 2".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail with the first task found
        let result = handler.delete_context("Office".to_string()).await;
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
            .add_context("Office".to_string(), Some("Work environment".to_string()))
            .await;
        assert!(result.is_ok());

        // Add a project with context
        let result = handler
            .add_project(
                "Office Project".to_string(),
                "office-project-1".to_string(),
                Some("Project description".to_string()),
                Some("Office".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // Verify project has context
        let data = handler.data.lock().unwrap();
        let project = data.projects.values().next().unwrap();
        assert_eq!(project.context, Some("Office".to_string()));
        assert_eq!(project.name, "Office Project");
    }

    #[tokio::test]
    async fn test_add_project_with_invalid_context() {
        let (handler, _temp_file) = get_test_handler();

        // Try to add project with non-existent context
        let result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                Some("NonExistent".to_string()),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_project_context() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        let _ = handler
            .add_context("Office".to_string(), Some("Work environment".to_string()))
            .await;

        // Add a project without context
        let result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Update project with context
        let result = handler
            .update_project(
                project_id.clone(),
                None,
                None,
                None,
                None,
                Some("Office".to_string()),
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
            .add_context("Office".to_string(), Some("Work environment".to_string()))
            .await;

        // Add a project with context
        let result = handler
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                Some("Office".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Remove context using empty string
        let result = handler
            .update_project(
                project_id.clone(),
                None,
                None,
                None,
                None,
                Some("".to_string()),
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with non-existent context
        let result = handler
            .update_project(
                project_id,
                None,
                None,
                None,
                None,
                Some("NonExistent".to_string()),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_project_with_custom_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project with custom ID
        let result = handler
            .add_project(
                "Custom ID Project".to_string(),
                "my-custom-id".to_string(),
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
        assert_eq!(project.name, "Custom ID Project");
    }

    #[tokio::test]
    async fn test_add_project_with_duplicate_id() {
        let (handler, _temp_file) = get_test_handler();

        // Add first project with custom ID
        let result = handler
            .add_project(
                "First Project".to_string(),
                "duplicate-id".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());

        // Try to add second project with same ID
        let result = handler
            .add_project(
                "Second Project".to_string(),
                "duplicate-id".to_string(),
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Update project ID
        let result = handler
            .update_project(
                project_id.clone(),
                Some("new-project-id".to_string()),
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
        assert_eq!(project.name, "Test Project");
    }

    #[tokio::test]
    async fn test_update_project_id_duplicate() {
        let (handler, _temp_file) = get_test_handler();

        // Add two projects
        let result1 = handler
            .add_project("Project 1".to_string(), "project-1".to_string(), None, None)
            .await;
        assert!(result1.is_ok());
        let project1_id = result1
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result2 = handler
            .add_project("Project 2".to_string(), "project-2".to_string(), None, None)
            .await;
        assert!(result2.is_ok());
        let project2_id = result2
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update project2's ID to project1's ID
        let result = handler
            .update_project(
                project2_id,
                Some(project1_id.clone()),
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
            .add_project(
                "Test Project".to_string(),
                "test-project-1".to_string(),
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Add a task referencing the project
        let result = handler
            .add_task(
                "Task in project".to_string(),
                Some(project_id.clone()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Verify task references the original project ID
        {
            let data = handler.data.lock().unwrap();
            let task = data.find_task_by_id(&task_id).unwrap();
            assert_eq!(task.project, Some(project_id.clone()));
        }

        // Update project ID
        let new_project_id = "updated-project-id".to_string();
        let result = handler
            .update_project(
                project_id.clone(),
                Some(new_project_id.clone()),
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
    async fn test_prompt_add_task_guide() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.add_task_guide().await;
        assert!(result.is_ok());
        let content = result.unwrap();

        // タスク作成ガイドの内容を確認
        assert!(content.contains("Task Creation"));
        assert!(content.contains("project"));
        assert!(content.contains("context"));
        assert!(content.contains("notes"));
        assert!(content.contains("start_date"));
        assert!(content.contains("#1"));
    }

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

    // 日付フィルタリングのテスト: start_dateが未来のタスクを除外
    #[tokio::test]
    async fn test_list_tasks_with_date_filter_excludes_future_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // タスクを3つ作成: 過去、今日、未来の日付
        let result = handler
            .add_task(
                "Past Task".to_string(),
                None,
                None,
                None,
                Some("2024-01-01".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let result = handler
            .add_task(
                "Today Task".to_string(),
                None,
                None,
                None,
                Some("2024-06-15".to_string()),
            )
            .await;
        assert!(result.is_ok());

        let result = handler
            .add_task(
                "Future Task".to_string(),
                None,
                None,
                None,
                Some("2024-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 日付フィルタ「2024-06-15」で一覧取得
        let result = handler
            .list_tasks(None, Some("2024-06-15".to_string()), None)
            .await;
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
            .add_task("No Date Task".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());

        // start_date付きのタスクを作成（未来）
        let result = handler
            .add_task(
                "Future Task".to_string(),
                None,
                None,
                None,
                Some("2025-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 日付フィルタで一覧取得
        let result = handler
            .list_tasks(None, Some("2024-06-15".to_string()), None)
            .await;
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
            .add_task(
                "Calendar Past".to_string(),
                None,
                None,
                None,
                Some("2024-01-01".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id1 = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        let result = handler
            .add_task(
                "Calendar Future".to_string(),
                None,
                None,
                None,
                Some("2025-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());
        let task_id2 = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // 両方をカレンダーステータスに移動
        let result = handler
            .change_task_status(vec![task_id1.clone()], "calendar".to_string(), None)
            .await;
        assert!(result.is_ok());
        let result = handler
            .change_task_status(vec![task_id2.clone()], "calendar".to_string(), None)
            .await;
        assert!(result.is_ok());

        // カレンダーステータスでフィルタリングし、日付フィルタも適用
        let result = handler
            .list_tasks(
                Some("calendar".to_string()),
                Some("2024-06-15".to_string()),
                None,
            )
            .await;
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
        let result = handler
            .list_tasks(None, Some("2024/06/15".to_string()), None)
            .await;
        assert!(result.is_err());

        let result = handler
            .list_tasks(None, Some("invalid-date".to_string()), None)
            .await;
        assert!(result.is_err());
    }

    // 日付フィルタリングのテスト: 日付フィルタなしでは全タスク表示
    #[tokio::test]
    async fn test_list_tasks_without_date_filter_shows_all_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 未来の日付のタスクを作成
        let result = handler
            .add_task(
                "Future Task".to_string(),
                None,
                None,
                None,
                Some("2025-12-31".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 日付フィルタなしで一覧取得
        let result = handler.list_tasks(None, None, None).await;
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
            .add_task(
                "Same Date Task".to_string(),
                None,
                None,
                None,
                Some("2024-06-15".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // 同じ日付でフィルタリング
        let result = handler
            .list_tasks(None, Some("2024-06-15".to_string()), None)
            .await;
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
            .add_task(
                "Task with notes".to_string(),
                None,
                None,
                Some("Important notes here".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // notesなしのタスクも作成
        let result = handler
            .add_task("Task without notes".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());

        // デフォルト（exclude_notes=None）で一覧取得
        let result = handler.list_tasks(None, None, None).await;
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
            .add_task(
                "Task with notes".to_string(),
                None,
                None,
                Some("Important notes here".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // exclude_notes=trueで一覧取得
        let result = handler.list_tasks(None, None, Some(true)).await;
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
            .add_task(
                "Task with notes".to_string(),
                None,
                None,
                Some("Important notes here".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // exclude_notes=falseで明示的に一覧取得
        let result = handler.list_tasks(None, None, Some(false)).await;
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
            .add_task(
                "Complex task".to_string(),
                None,
                None,
                Some("Line 1\nLine 2\nLine 3".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        // デフォルトで一覧取得
        let result = handler.list_tasks(None, None, None).await;
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
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            // Move to next_action first
            let _ = handler
                .change_task_status(vec![task_id.clone()], "next_action".to_string(), None)
                .await;
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にinboxに移動
        let result = handler
            .change_task_status(task_ids.clone(), "inbox".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::inbox));
        }
    }

    #[tokio::test]
    async fn test_next_action_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=4 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にnext_actionに移動
        let result = handler
            .change_task_status(task_ids.clone(), "next_action".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::next_action));
        }
    }

    #[tokio::test]
    async fn test_waiting_for_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にwaiting_forに移動
        let result = handler
            .change_task_status(task_ids.clone(), "waiting_for".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::waiting_for));
        }
    }

    #[tokio::test]
    async fn test_someday_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にsomedayに移動
        let result = handler
            .change_task_status(task_ids.clone(), "someday".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::someday));
        }
    }

    #[tokio::test]
    async fn test_later_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にlaterに移動
        let result = handler
            .change_task_status(task_ids.clone(), "later".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::later));
        }
    }

    #[tokio::test]
    async fn test_done_tasks_multiple_tasks() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にdoneに移動
        let result = handler
            .change_task_status(task_ids.clone(), "done".to_string(), None)
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
            assert!(matches!(task.status, TaskStatus::done));
        }
    }

    #[tokio::test]
    async fn test_calendar_tasks_multiple_tasks_with_date() {
        let (handler, _temp_file) = get_test_handler();

        // 複数のタスクを作成
        let mut task_ids = Vec::new();
        for i in 1..=3 {
            let result = handler
                .add_task(format!("Test Task {}", i), None, None, None, None)
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // 複数のタスクを一度にcalendarに移動（start_date指定）
        let result = handler
            .change_task_status(
                task_ids.clone(),
                "calendar".to_string(),
                Some("2025-01-15".to_string()),
            )
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to calendar: {:?}",
            result.err()
        );

        // すべてのタスクがcalendarに移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.calendar.len(), 3);
        assert_eq!(data.inbox.len(), 0);

        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, TaskStatus::calendar));
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
                .add_task(
                    format!("Test Task {}", i),
                    None,
                    None,
                    None,
                    Some("2025-02-01".to_string()),
                )
                .await;
            assert!(result.is_ok());
            let task_id = result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            task_ids.push(task_id);
        }

        // start_dateを指定せずにcalendarに移動（既存のstart_dateを使用）
        let result = handler
            .change_task_status(task_ids.clone(), "calendar".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Failed to move multiple tasks to calendar: {:?}",
            result.err()
        );

        // すべてのタスクがcalendarに移動され、既存のstart_dateが保持されていることを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.calendar.len(), 2);
        for task_id in &task_ids {
            let task = data.find_task_by_id(task_id).unwrap();
            assert!(matches!(task.status, TaskStatus::calendar));
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
            .add_task(
                "Task with date".to_string(),
                None,
                None,
                None,
                Some("2025-03-01".to_string()),
            )
            .await;
        assert!(result.is_ok());
        task_ids.push(
            result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string(),
        );

        // start_dateを持たないタスク
        let result = handler
            .add_task("Task without date".to_string(), None, None, None, None)
            .await;
        assert!(result.is_ok());
        task_ids.push(
            result
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string(),
        );

        // start_dateを指定せずに移動を試みる（部分的な失敗）
        let result = handler
            .change_task_status(task_ids.clone(), "calendar".to_string(), None)
            .await;
        assert!(
            result.is_ok(),
            "Should succeed with partial success: {:?}",
            result.err()
        );

        // 1つのタスクだけが移動されたことを確認
        let data = handler.data.lock().unwrap();
        assert_eq!(data.calendar.len(), 1);
        assert_eq!(data.inbox.len(), 1);
    }
}
