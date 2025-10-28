//! Legacy data types for migration support
//!
//! This module contains legacy GTD data structures (Task, Project, Context)
//! that are used for backward compatibility with old TOML formats.
//! New code should use the Nota structure from the gtd module.

use crate::gtd::NotaStatus;
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
///
/// ## Legacy Field Name Support
///
/// This struct supports legacy field names from older TOML formats:
/// - `name` is accepted as an alias for `title`
/// - `description` is accepted as an alias for `notes`
/// - `status` field is accepted but ignored during migration (was used in very old formats)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier (e.g., "project-1", "project-2")
    #[serde(default)]
    pub id: String,
    /// Project title (legacy: also accepts "name")
    #[serde(alias = "name")]
    pub title: String,
    /// Optional project notes (legacy: also accepts "description")
    #[serde(alias = "description")]
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
    /// Legacy status field - accepted during deserialization for backward compatibility
    ///
    /// Old TOML files (format_version = 2 and earlier) may contain a "status" field
    /// for projects. This field is accepted during deserialization but is not used
    /// in the current system (projects don't have status - only tasks/notas do).
    /// The field is never serialized back to TOML, ensuring clean migration to the
    /// new format where projects are simply identified by their `id` and contained
    /// in the `[projects.{id}]` section.
    ///
    /// Defaults to None if not specified in the TOML file.
    #[serde(skip_serializing, default)]
    pub status: Option<String>,
}

impl Project {
    /// Create a new Project with all required fields, status defaults to None
    ///
    /// This helper method is used during migration from legacy formats to ensure
    /// the status field is always set to None (it's not persisted in new format).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        title: String,
        notes: Option<String>,
        project: Option<String>,
        context: Option<String>,
        start_date: Option<NaiveDate>,
        created_at: NaiveDate,
        updated_at: NaiveDate,
    ) -> Self {
        Self {
            id,
            title,
            notes,
            project,
            context,
            start_date,
            created_at,
            updated_at,
            status: None,
        }
    }
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
