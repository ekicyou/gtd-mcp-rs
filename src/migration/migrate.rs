//! Migration functions for converting between GTD data format versions
//!
//! This module handles migrations between different versions of the GTD data format.

use super::legacy_types::{Context, Project, ProjectsFormat, Task};
use crate::gtd::{Nota, NotaStatus};
use serde::Deserialize;
use std::collections::HashMap;

/// Helper struct for deserializing GTD data with migration support
#[derive(Deserialize)]
pub struct GtdDataMigrationHelper {
    #[serde(default)]
    // デシリアライズ時のフォーマット検出に使用（TOMLから読み込まれるが、直接参照されないため警告が出る）
    #[allow(dead_code)]
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
#[allow(clippy::too_many_arguments)]
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
