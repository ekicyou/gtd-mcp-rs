//! Serialization and deserialization implementations for GtdData
//!
//! This module contains the custom Serialize and Deserialize implementations
//! for the GtdData structure. These are separated from the main gtd_data.rs
//! to improve modularity and maintainability.

use super::gtd_data::GtdData;
use super::nota::{Nota, NotaStatus};
use crate::migration::{
    GtdDataMigrationHelper, migrate_projects_to_latest, normalize_string_line_endings,
    nota_from_context, nota_from_project, populate_context_names, populate_project_ids,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// TOML array key for each status, in the order the arrays are written to the file
const STATUS_ARRAYS: [(NotaStatus, &str); 11] = [
    (NotaStatus::inbox, "inbox"),
    (NotaStatus::next_action, "next_action"),
    (NotaStatus::waiting_for, "waiting_for"),
    (NotaStatus::later, "later"),
    (NotaStatus::calendar, "calendar"),
    (NotaStatus::someday, "someday"),
    (NotaStatus::done, "done"),
    (NotaStatus::reference, "reference"),
    (NotaStatus::context, "context"),
    (NotaStatus::project, "project"),
    (NotaStatus::trash, "trash"),
];

impl<'de> Deserialize<'de> for GtdData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut helper = GtdDataMigrationHelper::deserialize(deserializer)?;

        // Legacy unified [[notas]] array takes precedence over status arrays
        let mut notas = std::mem::take(&mut helper.notas);

        if notas.is_empty() {
            // Current format: the status comes from the array the nota is stored in
            for (status, _) in &STATUS_ARRAYS {
                for mut nota in helper.take_status_array(status) {
                    nota.status = status.clone();
                    // Legacy [[context]] entries had only `name` (read into id), no title
                    if nota.status == NotaStatus::context && nota.title.is_empty() {
                        nota.title = nota.id.clone();
                    }
                    notas.push(nota);
                }
            }

            // Legacy formats: [projects.id] / [[projects]] and [contexts.name]
            let mut projects = migrate_projects_to_latest(helper.projects);
            populate_project_ids(&mut projects);
            notas.extend(projects.into_values().map(nota_from_project));

            let mut contexts = helper.contexts;
            populate_context_names(&mut contexts);
            notas.extend(contexts.into_values().map(nota_from_context));
        }

        // Normalize \r / \r\n in notes to \n so they can be written back OS-natively
        for nota in &mut notas {
            if let Some(notes) = &nota.notes {
                nota.notes = Some(normalize_string_line_endings(notes));
            }
        }

        // Build nota_map from all notas for duplicate checking
        let mut nota_map = HashMap::new();
        for nota in &notas {
            nota_map.insert(nota.id.clone(), nota.status.clone());
        }

        Ok(GtdData {
            format_version: 3, // Use version 3 for in-memory representation
            notas,
            nota_map,
            task_counter: helper.task_counter,
            project_counter: helper.project_counter,
        })
    }
}

impl Serialize for GtdData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("GtdData", 13)?;
        state.serialize_field("format_version", &self.format_version)?;

        // Serialize each status array (only if non-empty)
        for (status, key) in &STATUS_ARRAYS {
            let notas: Vec<&Nota> = self.notas.iter().filter(|n| &n.status == status).collect();
            if !notas.is_empty() {
                state.serialize_field(key, &notas)?;
            }
        }

        if self.task_counter != 0 {
            state.serialize_field("task_counter", &self.task_counter)?;
        }
        if self.project_counter != 0 {
            state.serialize_field("project_counter", &self.project_counter)?;
        }

        state.end()
    }
}
