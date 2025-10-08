mod gtd;
mod storage;

use anyhow::Result;
use chrono::NaiveDate;
use gtd::{GtdData, Project, ProjectStatus, Task, TaskStatus};
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

        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            status: TaskStatus::inbox,
            project,
            context,
            notes,
            start_date: parsed_start_date,
        };

        let mut data = self.data.lock().unwrap();
        let task_id = task.id.clone();
        data.tasks.insert(task_id.clone(), task);
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
        let mut tasks: Vec<&Task> = data.tasks.values().collect();

        if let Some(status_str) = status {
            tasks.retain(|task| match status_str.as_str() {
                "inbox" => matches!(task.status, TaskStatus::inbox),
                "next_action" => matches!(task.status, TaskStatus::next_action),
                "waiting_for" => matches!(task.status, TaskStatus::waiting_for),
                "someday" => matches!(task.status, TaskStatus::someday),
                "done" => matches!(task.status, TaskStatus::done),
                "trash" => matches!(task.status, TaskStatus::trash),
                _ => true,
            });
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

        if let Some(task) = data.tasks.get_mut(&task_id) {
            task.status = TaskStatus::trash;
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

        let trash_tasks: Vec<String> = data
            .tasks
            .iter()
            .filter(|(_, task)| matches!(task.status, TaskStatus::trash))
            .map(|(id, _)| id.clone())
            .collect();

        let count = trash_tasks.len();

        for task_id in trash_tasks {
            data.tasks.remove(&task_id);
        }

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
        data.projects.insert(project_id.clone(), project);
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
        let projects: Vec<&Project> = data.projects.values().collect();

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
