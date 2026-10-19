//! Common test utilities for integration tests
// Each test crate uses a different subset of these helpers.
#![allow(dead_code)]

use gtd_mcp::gtd::{GtdData, Nota, NotaStatus, local_date_today};
use gtd_mcp::migration::{
    Context, Project, Task, nota_from_context, nota_from_project, nota_from_task,
};

/// Extract task ID from inbox() response message
/// Response format: "Nota created with ID: <id> (type: task)"
pub fn extract_id_from_response(response: &str) -> String {
    // Parse "Nota created with ID: <id> (type: ...)"
    if let Some(start) = response.find("ID: ") {
        let id_part = &response[start + 4..];
        if let Some(end) = id_part.find(" (") {
            return id_part[..end].trim().to_string();
        }
    }
    // Fallback: try to get last whitespace-separated token without parentheses
    response
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_end_matches(')')
        .to_string()
}

/// Test-only shorthands for GtdData built on its public API
pub trait GtdDataTestExt {
    fn inbox(&self) -> Vec<Nota>;
    fn next_action(&self) -> Vec<Nota>;
    fn waiting_for(&self) -> Vec<Nota>;
    fn later(&self) -> Vec<Nota>;
    fn calendar(&self) -> Vec<Nota>;
    fn someday(&self) -> Vec<Nota>;
    fn done(&self) -> Vec<Nota>;
    fn reference(&self) -> Vec<Nota>;
    fn trash(&self) -> Vec<Nota>;
    fn add_task(&mut self, task: Task);
    fn add_project(&mut self, project: Project);
    fn add_context(&mut self, context: Context);
    fn find_task_by_id(&self, id: &str) -> Option<Nota>;
    fn move_status(&mut self, id: &str, status: NotaStatus) -> Option<()>;
    fn task_count(&self) -> usize;
}

impl GtdDataTestExt for GtdData {
    fn inbox(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::inbox))
    }
    fn next_action(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::next_action))
    }
    fn waiting_for(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::waiting_for))
    }
    fn later(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::later))
    }
    fn calendar(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::calendar))
    }
    fn someday(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::someday))
    }
    fn done(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::done))
    }
    fn reference(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::reference))
    }
    fn trash(&self) -> Vec<Nota> {
        self.list_all(Some(NotaStatus::trash))
    }
    fn add_task(&mut self, task: Task) {
        self.add(nota_from_task(task));
    }
    fn add_project(&mut self, project: Project) {
        self.add(nota_from_project(project));
    }
    fn add_context(&mut self, context: Context) {
        self.add(nota_from_context(context));
    }
    fn find_task_by_id(&self, id: &str) -> Option<Nota> {
        self.find_by_id(id).filter(|n| n.is_task())
    }
    fn move_status(&mut self, id: &str, status: NotaStatus) -> Option<()> {
        let mut nota = self.find_by_id(id)?;
        nota.status = status;
        nota.updated_at = local_date_today();
        self.update(id, nota).map(|_| ())
    }
    fn task_count(&self) -> usize {
        self.list_all(None).iter().filter(|n| n.is_task()).count()
    }
}
