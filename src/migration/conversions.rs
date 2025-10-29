//! Conversion functions between legacy types (Task, Project, Context) and Nota
//!
//! These conversion functions are used only during migration from old TOML formats
//! to the new unified Nota format. They should not be used in regular application code.

use super::legacy_types::{Context, Project, Task};
use crate::gtd::{Nota, NotaStatus, local_date_today};

/// Create a Nota from a legacy Task
///
/// This function is used during migration to convert old Task structures
/// to the new Nota format.
pub fn nota_from_task(task: Task) -> Nota {
    Nota {
        id: task.id,
        title: task.title,
        status: task.status,
        project: task.project,
        context: task.context,
        notes: task.notes,
        start_date: task.start_date,
        created_at: task.created_at,
        updated_at: task.updated_at,
        recurrence_pattern: None,
        recurrence_config: None,
    }
}

/// Create a Nota from a legacy Project
///
/// This function is used during migration to convert old Project structures
/// to the new Nota format.
pub fn nota_from_project(project: Project) -> Nota {
    Nota {
        id: project.id,
        title: project.title,
        status: NotaStatus::project,
        project: project.project,
        context: project.context,
        notes: project.notes,
        start_date: project.start_date,
        created_at: project.created_at,
        updated_at: project.updated_at,
        recurrence_pattern: None,
        recurrence_config: None,
    }
}

/// Create a Nota from a legacy Context
///
/// This function is used during migration to convert old Context structures
/// to the new Nota format.
pub fn nota_from_context(context: Context) -> Nota {
    Nota {
        id: context.name.clone(),
        title: context.title.unwrap_or(context.name),
        status: NotaStatus::context,
        project: context.project,
        context: context.context,
        notes: context.notes,
        start_date: context.start_date,
        created_at: context.created_at.unwrap_or_else(local_date_today),
        updated_at: context.updated_at.unwrap_or_else(local_date_today),
        recurrence_pattern: None,
        recurrence_config: None,
    }
}

/// Convert a Nota to a legacy Task (if status is task-related)
///
/// This function is used during migration to convert Nota back to Task format
/// for serialization in legacy formats.
pub fn nota_to_task(nota: &Nota) -> Option<Task> {
    match nota.status {
        NotaStatus::context | NotaStatus::project => None,
        _ => Some(Task {
            id: nota.id.clone(),
            title: nota.title.clone(),
            status: nota.status.clone(),
            project: nota.project.clone(),
            context: nota.context.clone(),
            notes: nota.notes.clone(),
            start_date: nota.start_date,
            created_at: nota.created_at,
            updated_at: nota.updated_at,
        }),
    }
}

/// Convert a Nota to a legacy Project (if status is project)
///
/// This function is used during migration to convert Nota back to Project format
/// for serialization in legacy formats.
pub fn nota_to_project(nota: &Nota) -> Option<Project> {
    if nota.status == NotaStatus::project {
        Some(Project::new(
            nota.id.clone(),
            nota.title.clone(),
            nota.notes.clone(),
            nota.project.clone(),
            nota.context.clone(),
            nota.start_date,
            nota.created_at,
            nota.updated_at,
        ))
    } else {
        None
    }
}

/// Convert a Nota to a legacy Context (if status is context)
///
/// This function is used during migration to convert Nota back to Context format
/// for serialization in legacy formats.
pub fn nota_to_context(nota: &Nota) -> Option<Context> {
    if nota.status == NotaStatus::context {
        Some(Context {
            name: nota.id.clone(),
            title: Some(nota.title.clone()),
            notes: nota.notes.clone(),
            status: NotaStatus::context,
            project: nota.project.clone(),
            context: nota.context.clone(),
            start_date: nota.start_date,
            created_at: Some(nota.created_at),
            updated_at: Some(nota.updated_at),
        })
    } else {
        None
    }
}
