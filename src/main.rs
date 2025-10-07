mod gtd;
mod storage;

use anyhow::Result;
use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::{server_runtime, ServerHandler};
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::schema::*;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::StdioTransport;
use rust_mcp_sdk::TransportOptions;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use gtd::{GtdData, Task, TaskStatus, Project, ProjectStatus};
use storage::Storage;

struct GtdServerHandler {
    data: Arc<Mutex<GtdData>>,
    storage: Arc<Storage>,
}

impl GtdServerHandler {
    fn new(storage_path: &str) -> Result<Self> {
        let storage = Arc::new(Storage::new(storage_path));
        let data = Arc::new(Mutex::new(storage.load()?));
        Ok(Self { data, storage })
    }

    async fn save_data(&self) -> Result<()> {
        let data = self.data.lock().await;
        self.storage.save(&data)?;
        Ok(())
    }
}

#[async_trait]
impl ServerHandler for GtdServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        // Helper to create property map from JSON
        let create_properties = |json_val: serde_json::Value| -> Option<std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>>> {
            if let serde_json::Value::Object(obj) = json_val {
                Some(obj.into_iter().filter_map(|(k, v)| {
                    if let serde_json::Value::Object(prop_map) = v {
                        Some((k, prop_map))
                    } else {
                        None
                    }
                }).collect())
            } else {
                None
            }
        };

        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "add_task".to_string(),
                    description: Some("Add a new task to the inbox".to_string()),
                    input_schema: ToolInputSchema::new(
                        vec!["title".to_string()],
                        create_properties(json!({
                            "title": {
                                "type": "string",
                                "description": "Task title"
                            },
                            "project": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "context": {
                                "type": "string",
                                "description": "Optional context ID"
                            },
                            "notes": {
                                "type": "string",
                                "description": "Optional notes"
                            }
                        })),
                    ),
                    title: None,
                    annotations: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "list_tasks".to_string(),
                    description: Some("List all tasks".to_string()),
                    input_schema: ToolInputSchema::new(
                        vec![],
                        create_properties(json!({
                            "status": {
                                "type": "string",
                                "description": "Optional status filter (Inbox, NextAction, WaitingFor, Someday, Done)"
                            }
                        })),
                    ),
                    title: None,
                    annotations: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "add_project".to_string(),
                    description: Some("Add a new project".to_string()),
                    input_schema: ToolInputSchema::new(
                        vec!["name".to_string()],
                        create_properties(json!({
                            "name": {
                                "type": "string",
                                "description": "Project name"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional project description"
                            }
                        })),
                    ),
                    title: None,
                    annotations: None,
                    meta: None,
                    output_schema: None,
                },
                Tool {
                    name: "list_projects".to_string(),
                    description: Some("List all projects".to_string()),
                    input_schema: ToolInputSchema::new(
                        vec![],
                        create_properties(json!({})),
                    ),
                    title: None,
                    annotations: None,
                    meta: None,
                    output_schema: None,
                },
            ],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        match request.params.name.as_str() {
            "add_task" => {
                let args = request.params.arguments.as_ref().ok_or_else(|| {
                    CallToolError::invalid_arguments("add_task", Some("Missing arguments".to_string()))
                })?;
                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CallToolError::invalid_arguments("add_task", Some("Missing title".to_string()))
                    })?;
                let project = args.get("project").and_then(|v| v.as_str()).map(|s| s.to_string());
                let context = args.get("context").and_then(|v| v.as_str()).map(|s| s.to_string());
                let notes = args.get("notes").and_then(|v| v.as_str()).map(|s| s.to_string());

                let task = Task {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: title.to_string(),
                    status: TaskStatus::Inbox,
                    project,
                    context,
                    notes,
                };

                let mut data = self.data.lock().await;
                let task_id = task.id.clone();
                data.tasks.insert(task_id.clone(), task);
                drop(data);

                self.save_data().await.map_err(|e| {
                    CallToolError::from_message(format!("Failed to save: {}", e))
                })?;

                Ok(CallToolResult::text_content(vec![TextContent::from(
                    format!("Task created with ID: {}", task_id),
                )]))
            }
            "list_tasks" => {
                let data = self.data.lock().await;
                let status_filter = request.params.arguments.as_ref()
                    .and_then(|args| args.get("status"))
                    .and_then(|v| v.as_str());

                let mut tasks: Vec<&Task> = data.tasks.values().collect();

                if let Some(status) = status_filter {
                    tasks.retain(|task| match status {
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

                Ok(CallToolResult::text_content(vec![TextContent::from(result)]))
            }
            "add_project" => {
                let args = request.params.arguments.as_ref().ok_or_else(|| {
                    CallToolError::invalid_arguments("add_project", Some("Missing arguments".to_string()))
                })?;
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CallToolError::invalid_arguments("add_project", Some("Missing name".to_string()))
                    })?;
                let description = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

                let project = Project {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    description,
                    status: ProjectStatus::Active,
                };

                let mut data = self.data.lock().await;
                let project_id = project.id.clone();
                data.projects.insert(project_id.clone(), project);
                drop(data);

                self.save_data().await.map_err(|e| {
                    CallToolError::from_message(format!("Failed to save: {}", e))
                })?;

                Ok(CallToolResult::text_content(vec![TextContent::from(
                    format!("Project created with ID: {}", project_id),
                )]))
            }
            "list_projects" => {
                let data = self.data.lock().await;
                let projects: Vec<&Project> = data.projects.values().collect();

                let mut result = String::new();
                for project in projects {
                    result.push_str(&format!(
                        "- [{}] {} (status: {:?})\n",
                        project.id, project.name, project.status
                    ));
                }

                Ok(CallToolResult::text_content(vec![TextContent::from(result)]))
            }
            _ => Err(CallToolError::unknown_tool(request.params.name)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let handler = GtdServerHandler::new("gtd.toml")?;

    let server_details = InitializeResult {
        server_info: Implementation {
            name: "gtd-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("GTD MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some("A GTD (Getting Things Done) MCP server with task and project management".to_string()),
        protocol_version: rust_mcp_sdk::schema::LATEST_PROTOCOL_VERSION.to_string(),
    };

    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Failed to create transport: {:?}", e))?;
    let server = server_runtime::create_server(server_details, transport, handler);
    
    server.start().await
        .map_err(|e| anyhow::anyhow!("Server error: {:?}", e))?;

    Ok(())
}

