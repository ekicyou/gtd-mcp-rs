//! Query and compatibility methods for GtdData
//!
//! This module contains query methods and legacy compatibility functions
//! for filtering and accessing GtdData. These are separated from the main
//! gtd_data.rs to improve modularity.

use super::gtd_data::GtdData;
use super::nota::{Nota, NotaStatus};
use crate::migration::{Context, Project, Task};
use std::collections::HashMap;

impl GtdData {
    // Query methods by status
    /// Get inbox notas (for compatibility)
    #[allow(dead_code)]
    pub fn inbox(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::inbox)
            .collect()
    }

    /// Get next_action notas (for compatibility)
    #[allow(dead_code)]
    pub fn next_action(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::next_action)
            .collect()
    }

    /// Get waiting_for notas (for compatibility)
    #[allow(dead_code)]
    pub fn waiting_for(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::waiting_for)
            .collect()
    }

    /// Get later notas (for compatibility)
    #[allow(dead_code)]
    pub fn later(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::later)
            .collect()
    }

    /// Get calendar notas (for compatibility)
    #[allow(dead_code)]
    pub fn calendar(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::calendar)
            .collect()
    }

    /// Get someday notas (for compatibility)
    #[allow(dead_code)]
    pub fn someday(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::someday)
            .collect()
    }

    /// Get done notas (for compatibility)
    #[allow(dead_code)]
    pub fn done(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::done)
            .collect()
    }

    /// Get reference notas (for compatibility)
    #[allow(dead_code)]
    pub fn reference(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::reference)
            .collect()
    }

    /// Get trash notas (for compatibility)
    #[allow(dead_code)]
    pub fn trash(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::trash)
            .collect()
    }

    /// Get projects map (for compatibility)
    #[allow(dead_code)]
    pub fn projects(&self) -> HashMap<String, &Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::project)
            .map(|n| (n.id.clone(), n))
            .collect()
    }

    /// Get contexts map (for compatibility)
    #[allow(dead_code)]
    pub fn contexts(&self) -> HashMap<String, &Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::context)
            .map(|n| (n.id.clone(), n))
            .collect()
    }

    // Legacy type compatibility methods
    /// Add a task (for compatibility with tests)
    #[allow(dead_code)]
    pub fn add_task(&mut self, task: Task) {
        self.add(Nota::from_task(task));
    }

    /// Remove a task (for compatibility with tests)
    #[allow(dead_code)]
    pub fn remove_task(&mut self, id: &str) -> Option<Task> {
        self.remove_nota(id).and_then(|n| n.to_task())
    }

    /// Add a project (for compatibility with tests)
    #[allow(dead_code)]
    pub fn add_project(&mut self, project: Project) {
        self.add(Nota::from_project(project));
    }

    /// Add a context (for compatibility with tests)
    #[allow(dead_code)]
    pub fn add_context(&mut self, context: Context) {
        self.add(Nota::from_context(context));
    }

    /// Validate task project (for compatibility)
    pub fn validate_task_project(&self, task: &Task) -> bool {
        match &task.project {
            None => true,
            Some(project_id) => self.find_project_by_id(project_id).is_some(),
        }
    }

    /// Validate task context (for compatibility)
    pub fn validate_task_context(&self, task: &Task) -> bool {
        match &task.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Validate task references (for compatibility)
    pub fn validate_task_references(&self, task: &Task) -> bool {
        self.validate_task_project(task) && self.validate_task_context(task)
    }

    /// Validate project context (for compatibility)
    pub fn validate_project_context(&self, project: &Project) -> bool {
        match &project.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }
}
