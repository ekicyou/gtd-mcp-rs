//! Migration functions for converting between GTD data format versions
//!
//! This module handles migrations between different versions of the GTD data format.

use super::legacy_types::{Context, Project, ProjectsFormat};
use crate::gtd::{Nota, NotaStatus};
use serde::Deserialize;
use std::collections::HashMap;

/// Helper struct for deserializing GTD data with migration support
#[derive(Deserialize)]
pub struct GtdDataMigrationHelper {
    #[serde(default)]
    // Used for format detection during deserialization (read from TOML but not directly referenced, causing warning)
    // デシリアライズ時のフォーマット検出に使用（TOMLから読み込まれるが、直接参照されないため警告が出る）
    #[allow(dead_code)]
    pub(crate) format_version: u32,
    // Current format: one array of notas per status ([[inbox]], [[project]], ...)
    #[serde(default)]
    pub(crate) inbox: Vec<Nota>,
    #[serde(default)]
    pub(crate) next_action: Vec<Nota>,
    #[serde(default)]
    pub(crate) waiting_for: Vec<Nota>,
    #[serde(default)]
    pub(crate) later: Vec<Nota>,
    #[serde(default)]
    pub(crate) calendar: Vec<Nota>,
    #[serde(default)]
    pub(crate) someday: Vec<Nota>,
    #[serde(default)]
    pub(crate) done: Vec<Nota>,
    #[serde(default)]
    pub(crate) reference: Vec<Nota>,
    #[serde(default)]
    pub(crate) context: Vec<Nota>,
    #[serde(default)]
    pub(crate) project: Vec<Nota>,
    #[serde(default)]
    pub(crate) trash: Vec<Nota>,
    // Legacy formats ([projects.id] / [[projects]], [contexts.name])
    #[serde(default)]
    pub(crate) projects: Option<ProjectsFormat>,
    #[serde(default)]
    pub(crate) contexts: HashMap<String, Context>,
    #[serde(default)]
    pub(crate) notas: Vec<Nota>,
    #[serde(default)]
    pub(crate) task_counter: u32,
    #[serde(default)]
    pub(crate) project_counter: u32,
}

impl GtdDataMigrationHelper {
    /// Take the status array whose TOML key matches `status`
    pub(crate) fn take_status_array(&mut self, status: &NotaStatus) -> Vec<Nota> {
        std::mem::take(match status {
            NotaStatus::inbox => &mut self.inbox,
            NotaStatus::next_action => &mut self.next_action,
            NotaStatus::waiting_for => &mut self.waiting_for,
            NotaStatus::later => &mut self.later,
            NotaStatus::calendar => &mut self.calendar,
            NotaStatus::someday => &mut self.someday,
            NotaStatus::done => &mut self.done,
            NotaStatus::reference => &mut self.reference,
            NotaStatus::context => &mut self.context,
            NotaStatus::project => &mut self.project,
            NotaStatus::trash => &mut self.trash,
        })
    }
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
