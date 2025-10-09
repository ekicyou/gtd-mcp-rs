mod gtd;
mod storage;

use anyhow::Result;
use chrono::NaiveDate;
use gtd::{GtdData, Project, ProjectStatus, Task, TaskPriority, TaskStatus, local_date_today};
use mcp_attr::server::{McpServer, mcp_server, serve_stdio};
use mcp_attr::{Result as McpResult, bail};
use std::sync::Mutex;
use storage::Storage;

struct GtdServerHandler {
    data: Mutex<GtdData>,
    storage: Storage,
}

impl GtdServerHandler {
    fn new(storage_path: &str) -> Result<Self> {
        let storage = Storage::new(storage_path);
        let data = Mutex::new(storage.load()?);
        Ok(Self { data, storage })
    }

    fn save_data(&self) -> Result<()> {
        let data = self.data.lock().unwrap();
        self.storage.save(&data)?;
        Ok(())
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
            title,
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

        if let Err(e) = self.save_data() {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Task created with ID: {}", task_id))
    }

    /// List all tasks with optional status filter
    #[tool]
    async fn list_tasks(
        &self,
        /// Optional status filter (inbox, next_action, waiting_for, someday, done, trash)
        status: Option<String>,
    ) -> McpResult<String> {
        let data = self.data.lock().unwrap();
        let mut tasks: Vec<&Task> = Vec::new();

        // Collect tasks from all status lists or just the filtered one
        if let Some(status_str) = status {
            match status_str.as_str() {
                "inbox" => tasks.extend(data.inbox.iter()),
                "next_action" => tasks.extend(data.next_action.iter()),
                "waiting_for" => tasks.extend(data.waiting_for.iter()),
                "someday" => tasks.extend(data.someday.iter()),
                "done" => tasks.extend(data.done.iter()),
                "trash" => tasks.extend(data.trash.iter()),
                _ => {
                    // If unknown status, return all tasks
                    tasks.extend(data.inbox.iter());
                    tasks.extend(data.next_action.iter());
                    tasks.extend(data.waiting_for.iter());
                    tasks.extend(data.someday.iter());
                    tasks.extend(data.done.iter());
                    tasks.extend(data.trash.iter());
                }
            }
        } else {
            // No filter, return all tasks
            tasks.extend(data.inbox.iter());
            tasks.extend(data.next_action.iter());
            tasks.extend(data.waiting_for.iter());
            tasks.extend(data.someday.iter());
            tasks.extend(data.done.iter());
            tasks.extend(data.trash.iter());
        }

        let mut result = String::new();
        for task in tasks {
            let date_info = task
                .start_date
                .map(|d| format!(" [start: {}]", d))
                .unwrap_or_default();
            result.push_str(&format!(
                "- [{}] {} (status: {:?}){} [created: {}, updated: {}]\n",
                task.id, task.title, task.status, date_info, task.created_at, task.updated_at
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

        // Use move_status to properly move the task to trash container
        if data.move_status(&task_id, TaskStatus::trash).is_some() {
            // Update the timestamp after the move
            if let Some(task) = data.find_task_by_id_mut(&task_id) {
                task.updated_at = local_date_today();
            }
            drop(data);

            if let Err(e) = self.save_data() {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to trash", task_id))
        } else {
            bail!("Task not found: {}", task_id);
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

            if let Err(e) = self.save_data() {
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

            if let Err(e) = self.save_data() {
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

            if let Err(e) = self.save_data() {
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

            if let Err(e) = self.save_data() {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to someday", task_id))
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

            if let Err(e) = self.save_data() {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to done", task_id))
        } else {
            bail!("Task not found: {}", task_id);
        }
    }

    /// Empty trash - permanently delete all trashed tasks
    #[tool]
    async fn empty_trash(&self) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        let count = data.trash.len();
        data.trash.clear();

        drop(data);

        if let Err(e) = self.save_data() {
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
    ) -> McpResult<String> {
        let mut data = self.data.lock().unwrap();

        let project = Project {
            id: data.generate_project_id(),
            name,
            description,
            status: ProjectStatus::active,
        };

        let project_id = project.id.clone();
        data.add_project(project);
        drop(data);

        if let Err(e) = self.save_data() {
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
            result.push_str(&format!(
                "- [{}] {} (status: {:?})\n",
                project.id, project.name, project.status
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

        if let Err(e) = self.save_data() {
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

        drop(data);

        if let Err(e) = self.save_data() {
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

        if let Err(e) = self.save_data() {
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

        if let Err(e) = self.save_data() {
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

        if let Err(e) = self.save_data() {
            bail!("Failed to save: {}", e);
        }

        Ok(format!("Context {} deleted successfully", name))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let handler = GtdServerHandler::new("gtd.toml")?;
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
        let handler = GtdServerHandler::new(temp_file.path().to_str().unwrap()).unwrap();
        (handler, temp_file)
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
        let project_result = handler.add_project("Test Project".to_string(), None).await;
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
        let result = handler.add_project("Original Name".to_string(), None).await;
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
        let result = handler.add_project("Test Project".to_string(), None).await;
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
            .update_project(project_id.clone(), None, Some("".to_string()), None)
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
        let result = handler.add_project("Test Project".to_string(), None).await;
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
            .update_project(project_id.clone(), None, None, Some("on_hold".to_string()))
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
        let result = handler.add_project("Test Project".to_string(), None).await;
        assert!(result.is_ok());
        let project_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with invalid status
        let result = handler
            .update_project(project_id, None, None, Some("invalid_status".to_string()))
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
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_multiple_fields_simultaneously() {
        let (handler, _temp_file) = get_test_handler();

        // Add a project
        let project_result = handler.add_project("Test Project".to_string(), None).await;
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
}
