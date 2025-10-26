//! Migration module for GTD data format versions
//!
//! This module handles migrations between different versions of the GTD data format.
//! Each version migration is implemented as a separate function to maintain clarity
//! and enable step-by-step migrations from older versions to the latest.
//!
//! ## Migration Strategy
//!
//! When a new format version is introduced:
//! 1. Add a new migration function (e.g., `migrate_v2_to_v3`)
//! 2. Update the `migrate_to_latest` function to chain migrations
//! 3. Add tests for the new migration path
//!
//! ## Current Versions
//!
//! - **Version 1**: Projects stored as `Vec<Project>` (TOML: `[[projects]]`)
//! - **Version 2**: Projects stored as `HashMap<String, Project>` (TOML: `[projects.id]`), separate arrays for each status
//! - **Version 3**: Internal storage uses `Vec<Nota>`, serializes as separate status arrays (`[[inbox]]`, `[[next_action]]`, etc.)

use crate::gtd::{Nota, NotaStatus};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get the current date in local timezone
pub fn local_date_today() -> NaiveDate {
    Local::now().date_naive()
}

/// Default task status for deserialization
fn default_task_status() -> NotaStatus {
    NotaStatus::inbox
}

/// Default context status for deserialization
fn default_context_status() -> NotaStatus {
    NotaStatus::context
}

/// Check if status is context (for skipping serialization)
fn is_context_status(status: &NotaStatus) -> bool {
    *status == NotaStatus::context
}

/// A GTD (Getting Things Done) task (legacy, used for migration only)
///
/// Tasks represent individual actionable items in the GTD system.
/// This structure is kept for backward compatibility with old TOML formats.
/// New code should use Nota instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier (e.g., "#1", "#2")
    pub id: String,
    /// Task title describing the action
    pub title: String,
    /// Current status of the task (inbox, next_action, waiting_for, etc.)
    #[serde(skip, default = "default_task_status")]
    pub status: NotaStatus,
    /// Optional project ID this task belongs to
    pub project: Option<String>,
    /// Optional context where this task can be performed (e.g., "@office", "@home")
    pub context: Option<String>,
    /// Optional additional notes in Markdown format
    pub notes: Option<String>,
    /// Optional start date for the task (format: YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,
    /// Date when the task was created
    pub created_at: NaiveDate,
    /// Date when the task was last updated
    pub updated_at: NaiveDate,
}

/// A GTD project (legacy, used for migration only)
///
/// Projects represent multi-step outcomes that require more than one action.
/// This structure is kept for backward compatibility with old TOML formats.
/// New code should use Nota instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier (e.g., "project-1", "project-2")
    #[serde(default)]
    pub id: String,
    /// Project title
    pub title: String,
    /// Optional project notes
    pub notes: Option<String>,
    /// Optional parent project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Optional context where this project can be worked on
    pub context: Option<String>,
    /// Optional start date (for scheduled projects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NaiveDate>,
    /// Creation date
    #[serde(default = "local_date_today")]
    pub created_at: NaiveDate,
    /// Last update date
    #[serde(default = "local_date_today")]
    pub updated_at: NaiveDate,
}

/// A GTD context (legacy, used for migration only)
///
/// Contexts represent locations, tools, or situations where tasks can be performed.
/// This structure is kept for backward compatibility with old TOML formats.
/// New code should use Nota instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Context name (e.g., "Office", "Home") - serves as ID
    /// Can also be deserialized from "id" field for legacy format compatibility
    #[serde(default, alias = "id")]
    pub name: String,
    /// Context title (same as name for contexts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional notes about the context
    pub notes: Option<String>,
    /// Status (always NotaStatus::context for context notas)
    #[serde(
        default = "default_context_status",
        skip_serializing_if = "is_context_status"
    )]
    pub status: NotaStatus,
    /// Parent project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Parent context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Optional start date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NaiveDate>,
    /// Creation date
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<NaiveDate>,
    /// Last update date
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<NaiveDate>,
}

/// Intermediate format for deserializing projects that supports both old and new formats
#[derive(Deserialize)]
#[serde(untagged)]
pub enum ProjectsFormat {
    /// New format: HashMap with project ID as key
    Map(HashMap<String, Project>),
    /// Old format: Vec with ID inside each project
    Vec(Vec<Project>),
}

/// Helper struct for deserializing GTD data with migration support
#[derive(Deserialize)]
pub struct GtdDataMigrationHelper {
    #[serde(default)]
    #[allow(dead_code)] // Used for format detection during deserialization
    pub(crate) format_version: u32,
    // Version 2/3 format fields (separate arrays by status)
    #[serde(default)]
    pub(crate) inbox: Vec<Task>,
    #[serde(default)]
    pub(crate) next_action: Vec<Task>,
    #[serde(default)]
    pub(crate) waiting_for: Vec<Task>,
    #[serde(default)]
    pub(crate) later: Vec<Task>,
    #[serde(default)]
    pub(crate) calendar: Vec<Task>,
    #[serde(default)]
    pub(crate) someday: Vec<Task>,
    #[serde(default)]
    pub(crate) done: Vec<Task>,
    #[serde(default)]
    pub(crate) reference: Vec<Task>,
    #[serde(default)]
    pub(crate) trash: Vec<Task>,
    #[serde(default)]
    pub(crate) projects: Option<ProjectsFormat>,
    #[serde(default)]
    pub(crate) contexts: HashMap<String, Context>,
    // Legacy intermediate format fields (for backward compatibility)
    #[serde(default)]
    pub(crate) project: Vec<Project>,
    #[serde(default)]
    pub(crate) context: Vec<Context>,
    #[serde(default)]
    pub(crate) notas: Vec<Nota>,
    #[serde(default)]
    pub(crate) task_counter: u32,
    #[serde(default)]
    pub(crate) project_counter: u32,
}

/// Migrate projects from Version 1 format (Vec) to Version 2 format (HashMap)
///
/// Version 1 stored projects as an array in TOML (`[[projects]]`) with the ID
/// as a field in each project. Version 2 stores projects as a HashMap where
/// the key is the project ID and the ID field is not serialized.
///
/// # Arguments
///
/// * `projects_vec` - Vector of projects from Version 1 format
///
/// # Returns
///
/// HashMap of projects with ID as the key
pub fn migrate_projects_v1_to_v2(projects_vec: Vec<Project>) -> HashMap<String, Project> {
    let mut projects_map = HashMap::new();
    for project in projects_vec {
        projects_map.insert(project.id.clone(), project);
    }
    projects_map
}

/// Convert projects from the intermediate format to the current HashMap format
///
/// This function handles both old (Vec) and new (HashMap) formats and returns
/// the appropriate HashMap representation.
///
/// # Arguments
///
/// * `projects_format` - Optional intermediate projects format
///
/// # Returns
///
/// HashMap of projects with ID as the key
pub fn migrate_projects_to_latest(
    projects_format: Option<ProjectsFormat>,
) -> HashMap<String, Project> {
    match projects_format {
        Some(ProjectsFormat::Map(map)) => map,
        Some(ProjectsFormat::Vec(vec)) => migrate_projects_v1_to_v2(vec),
        None => HashMap::new(),
    }
}

/// Populate the ID field in each project from the HashMap key
///
/// Since the ID is not serialized in the TOML file (it's used as the HashMap key),
/// we need to populate it from the key after deserialization.
///
/// # Arguments
///
/// * `projects` - Mutable reference to the projects HashMap
pub fn populate_project_ids(projects: &mut HashMap<String, Project>) {
    for (key, project) in projects.iter_mut() {
        project.id = key.clone();
    }
}

/// Populate the name field in each context from the HashMap key
///
/// Since the name is not serialized in the TOML file (it's used as the HashMap key),
/// we need to populate it from the key after deserialization.
///
/// # Arguments
///
/// * `contexts` - Mutable reference to the contexts HashMap
pub fn populate_context_names(contexts: &mut HashMap<String, Context>) {
    for (key, context) in contexts.iter_mut() {
        context.name = key.clone();
    }
}

/// Normalize line endings in a string to LF (\n)
///
/// This handles cases where TOML files contain \r escape sequences that get
/// unescaped to CR bytes during deserialization. We normalize these to LF
/// so they can be properly converted to OS-native format on save.
///
/// # Arguments
///
/// * `s` - String to normalize
///
/// # Returns
///
/// String with normalized line endings
pub fn normalize_string_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Normalize line endings in all string fields of tasks
///
/// # Arguments
///
/// * `tasks` - Mutable reference to a slice of tasks
pub fn normalize_task_line_endings(tasks: &mut [Task]) {
    for task in tasks.iter_mut() {
        if let Some(notes) = &task.notes {
            task.notes = Some(normalize_string_line_endings(notes));
        }
    }
}

/// Normalize line endings in all string fields of projects
///
/// # Arguments
///
/// * `projects` - Mutable reference to the projects HashMap
pub fn normalize_project_line_endings(projects: &mut HashMap<String, Project>) {
    for project in projects.values_mut() {
        if let Some(notes) = &project.notes {
            project.notes = Some(normalize_string_line_endings(notes));
        }
    }
}

/// Normalize line endings in all string fields of contexts
///
/// # Arguments
///
/// * `contexts` - Mutable reference to the contexts HashMap
pub fn normalize_context_line_endings(contexts: &mut HashMap<String, Context>) {
    for context in contexts.values_mut() {
        if let Some(notes) = &context.notes {
            context.notes = Some(normalize_string_line_endings(notes));
        }
    }
}

/// Convert unified notas from legacy format to separate arrays (internal format)
///
/// Some legacy files may have stored all items (tasks, projects, contexts) in a single `[[notas]]` array.
/// This function separates them into the internal format for easier processing.
///
/// # Arguments
///
/// * `notas` - Vector of notas from legacy format
/// * `inbox` - Output vector for inbox tasks
/// * `next_action` - Output vector for next_action tasks
/// * `waiting_for` - Output vector for waiting_for tasks
/// * `later` - Output vector for later tasks
/// * `calendar` - Output vector for calendar tasks
/// * `someday` - Output vector for someday tasks
/// * `done` - Output vector for done tasks
/// * `reference` - Output vector for reference tasks
/// * `trash` - Output vector for trash tasks
/// * `projects` - Output HashMap for projects
/// * `contexts` - Output HashMap for contexts
#[allow(dead_code, clippy::too_many_arguments)]
pub fn migrate_notas_v3_to_internal(
    notas: Vec<Nota>,
    inbox: &mut Vec<Task>,
    next_action: &mut Vec<Task>,
    waiting_for: &mut Vec<Task>,
    later: &mut Vec<Task>,
    calendar: &mut Vec<Task>,
    someday: &mut Vec<Task>,
    done: &mut Vec<Task>,
    reference: &mut Vec<Task>,
    trash: &mut Vec<Task>,
    projects: &mut HashMap<String, Project>,
    contexts: &mut HashMap<String, Context>,
) {
    use crate::gtd::NotaStatus;

    for nota in notas {
        match nota.status {
            NotaStatus::inbox => {
                if let Some(task) = nota.to_task() {
                    inbox.push(task);
                }
            }
            NotaStatus::next_action => {
                if let Some(task) = nota.to_task() {
                    next_action.push(task);
                }
            }
            NotaStatus::waiting_for => {
                if let Some(task) = nota.to_task() {
                    waiting_for.push(task);
                }
            }
            NotaStatus::later => {
                if let Some(task) = nota.to_task() {
                    later.push(task);
                }
            }
            NotaStatus::calendar => {
                if let Some(task) = nota.to_task() {
                    calendar.push(task);
                }
            }
            NotaStatus::someday => {
                if let Some(task) = nota.to_task() {
                    someday.push(task);
                }
            }
            NotaStatus::done => {
                if let Some(task) = nota.to_task() {
                    done.push(task);
                }
            }
            NotaStatus::reference => {
                if let Some(task) = nota.to_task() {
                    reference.push(task);
                }
            }
            NotaStatus::trash => {
                if let Some(task) = nota.to_task() {
                    trash.push(task);
                }
            }
            NotaStatus::project => {
                if let Some(project) = nota.to_project() {
                    projects.insert(project.id.clone(), project);
                }
            }
            NotaStatus::context => {
                if let Some(context) = nota.to_context() {
                    contexts.insert(context.name.clone(), context);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_projects_v1_to_v2() {
        let projects_vec = vec![
            Project {
                id: "project-1".to_string(),
                title: "First Project".to_string(),
                notes: Some("Notes 1".to_string()),
                project: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
                context: None,
            },
            Project {
                id: "project-2".to_string(),
                title: "Second Project".to_string(),
                notes: None,
                project: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
                context: Some("Office".to_string()),
            },
        ];

        let projects_map = migrate_projects_v1_to_v2(projects_vec);

        assert_eq!(projects_map.len(), 2);
        assert!(projects_map.contains_key("project-1"));
        assert!(projects_map.contains_key("project-2"));

        let project1 = &projects_map["project-1"];
        assert_eq!(project1.title, "First Project");
        assert_eq!(project1.notes, Some("Notes 1".to_string()));
    }

    #[test]
    fn test_normalize_string_line_endings() {
        assert_eq!(
            normalize_string_line_endings("hello\r\nworld"),
            "hello\nworld"
        );
        assert_eq!(
            normalize_string_line_endings("hello\rworld"),
            "hello\nworld"
        );
        assert_eq!(
            normalize_string_line_endings("hello\nworld"),
            "hello\nworld"
        );
        assert_eq!(
            normalize_string_line_endings("line1\r\nline2\rline3\nline4"),
            "line1\nline2\nline3\nline4"
        );
    }

    #[test]
    fn test_populate_project_ids() {
        let mut projects = HashMap::new();
        projects.insert(
            "proj-1".to_string(),
            Project {
                id: String::new(), // ID is empty before population
                title: "Test".to_string(),
                notes: None,
                project: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
                context: None,
            },
        );

        populate_project_ids(&mut projects);

        assert_eq!(projects["proj-1"].id, "proj-1");
    }
}
