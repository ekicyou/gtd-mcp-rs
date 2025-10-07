mod gtd;
mod storage;

use anyhow::Result;
use mcp_attr::server::{mcp_server, McpServer, serve_stdio};
use mcp_attr::{Result as McpResult, bail};
use std::sync::Mutex;
use gtd::{GtdData, Task, TaskStatus, Project, ProjectStatus};
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
    ) -> McpResult<String> {
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            status: TaskStatus::Inbox,
            project,
            context,
            notes,
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
        /// Optional status filter (Inbox, NextAction, WaitingFor, Someday, Done)
        status: Option<String>,
    ) -> McpResult<String> {
        let data = self.data.lock().unwrap();
        let mut tasks: Vec<&Task> = data.tasks.values().collect();

        if let Some(status_str) = status {
            tasks.retain(|task| match status_str.as_str() {
                "Inbox" => matches!(task.status, TaskStatus::Inbox),
                "NextAction" => matches!(task.status, TaskStatus::NextAction),
                "WaitingFor" => matches!(task.status, TaskStatus::WaitingFor),
                "Someday" => matches!(task.status, TaskStatus::Someday),
                "Done" => matches!(task.status, TaskStatus::Done),
                _ => true,
            });
        }

        let mut result = String::new();
        for task in tasks {
            result.push_str(&format!(
                "- [{}] {} (status: {:?})\n",
                task.id, task.title, task.status
            ));
        }

        Ok(result)
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
            status: ProjectStatus::Active,
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

