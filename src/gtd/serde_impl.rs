//! Serialization and deserialization implementations for GtdData
//!
//! This module contains the custom Serialize and Deserialize implementations
//! for the GtdData structure. These are separated from the main gtd_data.rs
//! to improve modularity and maintainability.

use super::gtd_data::GtdData;
use super::nota::{Nota, NotaStatus};
use crate::migration::{
    // Helper type for migration
    GtdDataMigrationHelper,
    // Migration functions
    migrate_projects_to_latest,
    // Normalization functions
    normalize_context_line_endings,
    normalize_project_line_endings,
    normalize_task_line_endings,
    // Populate functions
    populate_context_names,
    populate_project_ids,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

impl<'de> Deserialize<'de> for GtdData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = GtdDataMigrationHelper::deserialize(deserializer)?;

        // Start with notas from Version 4/5 format if available
        let mut notas = helper.notas;

        // If notas is empty, we need to migrate from older formats or Version 3 status-based arrays
        if notas.is_empty() {
            // Initialize collections for migration
            let mut inbox = helper.inbox;
            let mut next_action = helper.next_action;
            let mut waiting_for = helper.waiting_for;
            let mut later = helper.later;
            let mut calendar = helper.calendar;
            let mut someday = helper.someday;
            let mut done = helper.done;
            let mut reference = helper.reference;
            let mut trash = helper.trash;
            let mut projects = migrate_projects_to_latest(helper.projects);
            let mut contexts = helper.contexts;

            // If this is Version 3 format with Vec arrays for projects/contexts, convert to HashMap
            if !helper.project.is_empty() {
                for project in helper.project {
                    projects.insert(project.id.clone(), project);
                }
            }
            if !helper.context.is_empty() {
                for context in helper.context {
                    contexts.insert(context.name.clone(), context);
                }
            }

            // Populate the name/id fields
            populate_context_names(&mut contexts);
            populate_project_ids(&mut projects);

            // Normalize line endings in all string fields
            normalize_task_line_endings(&mut inbox);
            normalize_task_line_endings(&mut next_action);
            normalize_task_line_endings(&mut waiting_for);
            normalize_task_line_endings(&mut later);
            normalize_task_line_endings(&mut calendar);
            normalize_task_line_endings(&mut someday);
            normalize_task_line_endings(&mut done);
            normalize_task_line_endings(&mut reference);
            normalize_task_line_endings(&mut trash);
            normalize_project_line_endings(&mut projects);
            normalize_context_line_endings(&mut contexts);

            // Set the status field for each task based on which collection it's in
            for task in &mut inbox {
                task.status = NotaStatus::inbox;
            }
            for task in &mut next_action {
                task.status = NotaStatus::next_action;
            }
            for task in &mut waiting_for {
                task.status = NotaStatus::waiting_for;
            }
            for task in &mut later {
                task.status = NotaStatus::later;
            }
            for task in &mut calendar {
                task.status = NotaStatus::calendar;
            }
            for task in &mut someday {
                task.status = NotaStatus::someday;
            }
            for task in &mut done {
                task.status = NotaStatus::done;
            }
            for task in &mut reference {
                task.status = NotaStatus::reference;
            }
            for task in &mut trash {
                task.status = NotaStatus::trash;
            }

            // Convert all old structures to Nota
            for task in inbox {
                notas.push(Nota::from_task(task));
            }
            for task in next_action {
                notas.push(Nota::from_task(task));
            }
            for task in waiting_for {
                notas.push(Nota::from_task(task));
            }
            for task in later {
                notas.push(Nota::from_task(task));
            }
            for task in calendar {
                notas.push(Nota::from_task(task));
            }
            for task in someday {
                notas.push(Nota::from_task(task));
            }
            for task in done {
                notas.push(Nota::from_task(task));
            }
            for task in reference {
                notas.push(Nota::from_task(task));
            }
            for task in trash {
                notas.push(Nota::from_task(task));
            }
            for project in projects.into_values() {
                notas.push(Nota::from_project(project));
            }
            for context in contexts.into_values() {
                notas.push(Nota::from_context(context));
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
        use std::collections::HashMap;

        let mut state = serializer.serialize_struct("GtdData", 13)?;
        state.serialize_field("format_version", &self.format_version)?;

        // Separate notas by status in a single pass (Version 3 format)
        let mut status_map: HashMap<NotaStatus, Vec<&Nota>> = HashMap::new();
        for nota in &self.notas {
            status_map
                .entry(nota.status.clone())
                .or_default()
                .push(nota);
        }

        // Serialize each status array (only if non-empty), in the order they appear in the enum
        if let Some(inbox) = status_map.get(&NotaStatus::inbox) {
            state.serialize_field("inbox", inbox)?;
        }
        if let Some(next_action) = status_map.get(&NotaStatus::next_action) {
            state.serialize_field("next_action", next_action)?;
        }
        if let Some(waiting_for) = status_map.get(&NotaStatus::waiting_for) {
            state.serialize_field("waiting_for", waiting_for)?;
        }
        if let Some(later) = status_map.get(&NotaStatus::later) {
            state.serialize_field("later", later)?;
        }
        if let Some(calendar) = status_map.get(&NotaStatus::calendar) {
            state.serialize_field("calendar", calendar)?;
        }
        if let Some(someday) = status_map.get(&NotaStatus::someday) {
            state.serialize_field("someday", someday)?;
        }
        if let Some(done) = status_map.get(&NotaStatus::done) {
            state.serialize_field("done", done)?;
        }
        if let Some(reference) = status_map.get(&NotaStatus::reference) {
            state.serialize_field("reference", reference)?;
        }
        if let Some(context) = status_map.get(&NotaStatus::context) {
            state.serialize_field("context", context)?;
        }
        if let Some(project) = status_map.get(&NotaStatus::project) {
            state.serialize_field("project", project)?;
        }
        if let Some(trash) = status_map.get(&NotaStatus::trash) {
            state.serialize_field("trash", trash)?;
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
