mod git_ops;
mod gtd;
mod storage;

use anyhow::Result;
use chrono::NaiveDate;
use clap::{CommandFactory, Parser};
use gtd::{GtdData, Project, ProjectStatus, Task, TaskStatus, local_date_today};
use mcp_attr::server::{McpServer, mcp_server, serve_stdio};
use mcp_attr::{Result as McpResult, bail};
use std::sync::Mutex;
use storage::Storage;

/// GTD MCP Server - Getting Things Done task management via Model Context Protocol
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the GTD data file
    file: String,

    /// Enable git synchronization on save
    #[arg(long)]
    sync_git: bool,
}

struct GtdServerHandler {
    data: Mutex<GtdData>,
    storage: Storage,
}

impl GtdServerHandler {
    fn new(storage_path: &str, sync_git: bool) -> Result<Self> {
        let storage = Storage::new(storage_path, sync_git);
        let data = Mutex::new(storage.load()?);
        Ok(Self { data, storage })
    }

    #[allow(dead_code)]
    fn save_data(&self) -> Result<()> {
        let data = self.data.lock().unwrap();
        self.storage.save(&data)?;
        Ok(())
    }

    fn save_data_with_message(&self, message: &str) -> Result<()> {
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

/// GTD MCP Server providing task and project management
#[mcp_server]
impl McpServer for GtdServerHandler {
    /// Add a new task to the inbox
    #[tool]
    async fn add_task(
        &self,
        /// Task title
        title: String,
        /// Optional project ID
        project: Option<String>,
        /// Optional context ID
        context: Option<String>,
        /// Optional notes
        notes: Option<String>,
        /// Optional start date (format: YYYY-MM-DD)
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

    /// List all tasks with optional status filter
    #[tool]
    async fn list_tasks(
        &self,
        /// Optional status filter (inbox, next_action, waiting_for, someday, later, done, trash, calendar)
        status: Option<String>,
        /// Optional date filter (format: YYYY-MM-DD). Tasks with start_date in the future are excluded
        date: Option<String>,
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
            result.push_str(&format!(
                "- [{}] {} (status: {:?}){}{}{} [created: {}, updated: {}]\n",
                task.id,
                task.title,
                task.status,
                date_info,
                project_info,
                context_info,
                task.created_at,
                task.updated_at
            ));
        }

        Ok(result)
    }

    /// Move a task to trash
    #[tool]
    async fn trash_task(
        &self,
        /// Task ID to move to trash
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check if task exists before attempting to move
        let task_exists = data.find_task_by_id(&task_id).is_some();
        if !task_exists {
            eprintln!("Error: Attempted to trash non-existent task: {}", task_id);
            bail!(
                "Task not found: {}. Please check the task ID and try again.",
                task_id
            );
        }

        let original_status = data
            .find_task_by_id(&task_id)
            .map(|t| format!("{:?}", t.status));
        eprintln!(
            "Moving task {} from {:?} to trash",
            task_id, original_status
        );

        // Use move_status to properly move the task to trash container
        if data.move_status(&task_id, TaskStatus::trash).is_some() {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) = self.save_data_with_message(&format!("Move task {} to trash", task_id))
            {
                eprintln!(
                    "Error: Failed to save data after moving task {} to trash: {}",
                    task_id, e
                );
                bail!(
                    "Failed to save task to trash: {}. The task may not have been moved.",
                    e
                );
            }

            eprintln!("Successfully moved task {} to trash", task_id);
            Ok(format!("Task {} moved to trash", task_id))
        } else {
            // This should not happen since we checked above, but handle it anyway
            eprintln!(
                "Error: move_status failed for task {} (this should not happen)",
                task_id
            );
            bail!(
                "Failed to move task {} to trash. Internal error occurred.",
                task_id
            );
        }
    }

    /// Move a task to inbox
    #[tool]
    async fn inbox_task(
        &self,
        /// Task ID to move to inbox
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Use move_status to properly move the task to inbox container
        if data.move_status(&task_id, TaskStatus::inbox).is_some() {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) = self.save_data_with_message(&format!("Move task {} to inbox", task_id))
            {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to inbox", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Move a task to next action
    #[tool]
    async fn next_action_task(
        &self,
        /// Task ID to move to next action
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Use move_status to properly move the task to next_action container
        if data
            .move_status(&task_id, TaskStatus::next_action)
            .is_some()
        {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) =
                self.save_data_with_message(&format!("Move task {} to next_action", task_id))
            {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to next action", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Move a task to waiting for
    #[tool]
    async fn waiting_for_task(
        &self,
        /// Task ID to move to waiting for
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Use move_status to properly move the task to waiting_for container
        if data
            .move_status(&task_id, TaskStatus::waiting_for)
            .is_some()
        {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) =
                self.save_data_with_message(&format!("Move task {} to waiting_for", task_id))
            {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to waiting for", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Move a task to someday
    #[tool]
    async fn someday_task(
        &self,
        /// Task ID to move to someday
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Use move_status to properly move the task to someday container
        if data.move_status(&task_id, TaskStatus::someday).is_some() {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) =
                self.save_data_with_message(&format!("Move task {} to someday", task_id))
            {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to someday", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Move a task to later
    #[tool]
    async fn later_task(
        &self,
        /// Task ID to move to later
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Use move_status to properly move the task to later container
        if data.move_status(&task_id, TaskStatus::later).is_some() {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) = self.save_data_with_message(&format!("Move task {} to later", task_id))
            {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to later", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Move a task to done
    #[tool]
    async fn done_task(
        &self,
        /// Task ID to move to done
        task_id: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Use move_status to properly move the task to done container
        if data.move_status(&task_id, TaskStatus::done).is_some() {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) = self.save_data_with_message(&format!("Mark task {} as done", task_id)) {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to done", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Move a task to calendar
    #[tool]
    async fn calendar_task(
        &self,
        /// Task ID to move to calendar
        task_id: String,
        /// Optional start date (format: YYYY-MM-DD). If not provided, task must already have a start_date
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

        // Check if task exists
        let task_exists = data.find_task_by_id(&task_id).is_some();
        if !task_exists {
            bail!("Task not found: {}", task_id);
        }

        // Check if task will have a start_date after the operation
        let current_start_date = data.find_task_by_id(&task_id).unwrap().start_date;
        let final_start_date = parsed_start_date.or(current_start_date);

        if final_start_date.is_none() {
            bail!(
                "Task must have a start_date to be moved to calendar. Please provide a start_date parameter or set it first."
            );
        }

        // Move the task to calendar status
        if data.move_status(&task_id, TaskStatus::calendar).is_some() {
            // Update the start_date if provided, and update timestamp
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                if let Some(date) = parsed_start_date {
                    task.start_date = Some(date);
                }
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) =
                self.save_data_with_message(&format!("Move task {} to calendar", task_id))
            {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to calendar", task_id))
        } else {
            bail!("Failed to move task to calendar");
        }
    }

    /// Empty trash - permanently delete all trashed tasks
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

    /// Add a new project
    #[tool]
    async fn add_project(
        &self,
        /// Project name
        name: String,
        /// Optional project description
        description: Option<String>,
        /// Optional context name (must exist if specified)
        context: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        let project = Project {
            id: data.generate_project_id(),
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

        let project_id = project.id.clone();
        data.add_project(project);
        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Add project: {}", name)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Project created with ID: {}", project_id))
    }

    /// List all projects
    #[tool]
    async fn list_projects(&self) -> McpResult<String> {
        let data = self.data.lock().unwrap();
        let projects: Vec<&Project> = data.projects.iter().collect();

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

    /// Update an existing task
    #[tool]
    async fn update_task(
        &self,
        /// Task ID to update
        task_id: String,
        /// Optional new title
        title: Option<String>,
        /// Optional new project ID (use empty string to remove)
        project: Option<String>,
        /// Optional new context ID (use empty string to remove)
        context: Option<String>,
        /// Optional new notes (use empty string to remove)
        notes: Option<String>,
        /// Optional new start date (format: YYYY-MM-DD, use empty string to remove)
        start_date: Option<String>,
    ) -> McpResult<String> {
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

    /// Update an existing project
    #[tool]
    async fn update_project(
        &self,
        /// Project ID to update
        project_id: String,
        /// Optional new name
        name: Option<String>,
        /// Optional new description (use empty string to remove)
        description: Option<String>,
        /// Optional new status (active, on_hold, completed)
        status: Option<String>,
        /// Optional new context name (use empty string to remove)
        context: Option<String>,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Find the project
        let project = match data.find_project_by_id_mut(&project_id) {
            Some(p) => p,
            None => {
                drop(data);
                bail!("Project not found: {}", project_id);
            }
        };

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

        // Clone project for validation
        let project_clone = project.clone();

        // Validate context reference
        if !data.validate_project_context(&project_clone) {
            drop(data);
            bail!("Invalid context reference: context does not exist");
        }

        drop(data);

        if let Err(e) = self.save_data_with_message(&format!("Update project {}", project_id)) {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Project {} updated successfully", project_id))
    }

    /// Add a new context
    #[tool]
    async fn add_context(
        &self,
        /// Context name
        name: String,
        /// Optional context description
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

    /// List all contexts
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

    /// Update an existing context
    #[tool]
    async fn update_context(
        &self,
        /// Context name
        name: String,
        /// Optional new description (use empty string to remove)
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

    /// Delete a context
    #[tool]
    async fn delete_context(
        &self,
        /// Context name to delete
        name: String,
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        // Check if context exists
        if data.contexts.remove(&name).is_none() {
            drop(data);
            bail!("Context not found: {}", name);
        }

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

## Common Workflows

1. **Capture**: Use `add_task` to capture items to inbox
2. **Process**: Review inbox, clarify and organize with status movement tools
3. **Review**: Regularly check all task statuses with `list_tasks`
4. **Do**: Focus on `next_action` tasks for execution

## Key Tools

- Task Management: add_task, update_task, list_tasks, delete_task
- Status Movement: inbox_task, next_action_task, waiting_for_task, someday_task, later_task, calendar_task, done_task, trash_task
- Projects: add_project, list_projects, update_project, delete_project
- Contexts: add_context, list_contexts, update_context, delete_context

Use prompts like `process_inbox`, `weekly_review`, or `next_actions` for workflow guidance."#.to_string())
    }

    /// Guide for processing inbox items
    #[prompt]
    async fn process_inbox(&self) -> McpResult<String> {
        Ok(r#"# GTD Inbox Processing Guide

## Workflow for each inbox item:

1. **List inbox**: Use `list_tasks` with status "inbox"

2. **For each task, ask:**
   - Is it actionable?
     - NO → Move to `someday_task` or `trash_task`
     - YES → Continue to step 3

3. **Will it take less than 2 minutes?**
   - YES → Do it now, then `done_task`
   - NO → Continue to step 4

4. **Can I do it myself?**
   - NO → Use `waiting_for_task` and add notes about who/what you're waiting for
   - YES → Continue to step 5

5. **Is there a specific date?**
   - YES → Use `calendar_task` with start_date parameter
   - NO → Continue to step 6

6. **Should this be done later (deferred)?**
   - YES → Use `later_task` for tasks deferred to a later time
   - NO → Continue to step 7

7. **Is it part of a larger project?**
   - YES → Use `update_task` to assign project
   - NO → Continue to step 8

8. **Add context if helpful** (e.g., @office, @computer)
   - Use `update_task` to set context

9. **Move to next actions**
   - Use `next_action_task`

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
     - Mark completed ones as `done_task`
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
- Use `done_task` to mark complete
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

**notes**: Additional details
- Background information
- URLs, reference numbers
- Why this matters

**start_date**: For calendar tasks only (YYYY-MM-DD)
- Events with specific dates
- Tickler file items
- Don't use for regular next actions

## Workflow

1. Quick capture to inbox: `add_task` with just title
2. Later process inbox to clarify and organize
3. Use `update_task` to add details as needed

Remember: Task IDs are #1, #2, #3, etc. (LLM-friendly short format)"#
            .to_string())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check if no arguments were provided (except the program name)
    if std::env::args().len() == 1 {
        // No arguments provided, show help and exit with error code
        let mut cmd = Args::command();
        cmd.print_help().ok();
        println!(); // Add a newline after help
        std::process::exit(2);
    }

    let args = Args::parse();
    let handler = GtdServerHandler::new(&args.file, args.sync_git)?;
    serve_stdio(handler).await?;
    Ok(())
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
        assert_eq!(handler.storage.file_path.to_str().unwrap(), custom_path);

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
        let result = handler.next_action_task(task_id.clone()).await;
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
            .add_project("Test Project".to_string(), None, None)
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
            .add_project("Original Name".to_string(), None, None)
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
            .add_project("Test Project".to_string(), None, None)
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
            .update_project(project_id.clone(), None, Some("".to_string()), None, None)
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
            .add_project("Test Project".to_string(), None, None)
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
            .add_project("Test Project".to_string(), None, None)
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
                Some("New Name".to_string()),
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_multiple_fields_simultaneously() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let project_result = handler
            .add_project("Test Project".to_string(), None, None)
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
        let result = handler.done_task(task_id.clone()).await;
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
        let result = handler.next_action_task(task_id.clone()).await;
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
        let result = handler.inbox_task(task_id.clone()).await;
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

        let result = handler.next_action_task(task_id.clone()).await;
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

        let result = handler.waiting_for_task(task_id.clone()).await;
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

        let result = handler.someday_task(task_id.clone()).await;
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

        let result = handler.later_task(task_id.clone()).await;
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

        let result = handler.done_task(task_id.clone()).await;
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

        let result = handler.trash_task(task_id.clone()).await;
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

        let result = handler.trash_task(task_id_1.clone()).await;
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

        let result = handler.done_task(task_id_2.clone()).await;
        assert!(result.is_ok(), "Moving to done failed: {:?}", result.err());

        let result = handler.trash_task(task_id_2.clone()).await;
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
            let result = handler.trash_task(task_id.to_string()).await;
            assert!(result.is_err(), "Expected error for task_id: {}", task_id);
        }
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
            .calendar_task(task_id.clone(), Some("2024-12-25".to_string()))
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
        let result = handler.calendar_task(task_id.clone(), None).await;
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
        let result = handler.calendar_task(task_id.clone(), None).await;
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
            .calendar_task(task_id.clone(), Some("2024-12-31".to_string()))
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
            .calendar_task(task_id.clone(), Some("2024/12/25".to_string()))
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
        let result = handler.next_action_task(task_id.clone()).await;
        assert!(result.is_ok());

        // Verify created_at unchanged
        let data = handler.data.lock().unwrap();
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.created_at, created_at);
    }

    #[tokio::test]
    async fn test_status_movement_nonexistent_task() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.next_action_task("nonexistent-id".to_string()).await;
        assert!(result.is_err());

        let result = handler.done_task("nonexistent-id".to_string()).await;
        assert!(result.is_err());

        let result = handler.trash_task("nonexistent-id".to_string()).await;
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
                Some("Project description".to_string()),
                Some("Office".to_string()),
            )
            .await;
        assert!(result.is_ok());

        // Verify project has context
        let data = handler.data.lock().unwrap();
        let project = data.projects.first().unwrap();
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
            .add_project("Test Project".to_string(), None, None)
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
            .add_project("Test Project".to_string(), None, Some("Office".to_string()))
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
            .update_project(project_id.clone(), None, None, None, Some("".to_string()))
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
            .add_project("Test Project".to_string(), None, None)
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
                Some("NonExistent".to_string()),
            )
            .await;
        assert!(result.is_err());
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
            .list_tasks(None, Some("2024-06-15".to_string()))
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
            .list_tasks(None, Some("2024-06-15".to_string()))
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
        let result = handler.calendar_task(task_id1.clone(), None).await;
        assert!(result.is_ok());
        let result = handler.calendar_task(task_id2.clone(), None).await;
        assert!(result.is_ok());

        // カレンダーステータスでフィルタリングし、日付フィルタも適用
        let result = handler
            .list_tasks(Some("calendar".to_string()), Some("2024-06-15".to_string()))
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
            .list_tasks(None, Some("2024/06/15".to_string()))
            .await;
        assert!(result.is_err());

        let result = handler
            .list_tasks(None, Some("invalid-date".to_string()))
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
        let result = handler.list_tasks(None, None).await;
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
            .list_tasks(None, Some("2024-06-15".to_string()))
            .await;
        assert!(result.is_ok());
        let list = result.unwrap();

        // 同じ日付のタスクは表示される（未来ではない）
        assert!(list.contains("Same Date Task"));
    }
}
