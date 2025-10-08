mod gtd;
mod storage;

use anyhow::Result;
use chrono::NaiveDate;
use gtd::{GtdData, Project, ProjectStatus, Task, TaskStatus, local_date_today};
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

        let today = local_date_today();
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            status: TaskStatus::inbox,
            project,
            context,
            notes,
            start_date: parsed_start_date,
            created_at: today,
            updated_at: today,
        };

        let mut data = self.data.lock().unwrap();
        
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
                "- [{}] {} (status: {:?}){}\n",
                task.id, task.title, task.status, date_info
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

        if let Some(task) = data.find_task_by_id_mut(&task_id) {
            task.status = TaskStatus::trash;
            task.updated_at = local_date_today();
            drop(data);

            if let Err(e) = self.save_data() {
                bail!("Failed to save: {}", e);
            }

            Ok(format!("Task {} moved to trash", task_id))
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
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            status: ProjectStatus::active,
        };

        let mut data = self.data.lock().unwrap();
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let handler = GtdServerHandler::new("gtd.toml")?;
    serve_stdio(handler).await?;
    Ok(())
}
