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

mod legacy_types;
mod migrate;
mod normalize;

// Re-export public types and functions
pub use legacy_types::{Context, Project, ProjectsFormat, Task};
pub use migrate::{
    GtdDataMigrationHelper, migrate_notas_v3_to_internal, migrate_projects_to_latest,
    migrate_projects_v1_to_v2, populate_context_names, populate_project_ids,
};
pub use normalize::{
    normalize_context_line_endings, normalize_project_line_endings, normalize_string_line_endings,
    normalize_task_line_endings,
};

// Re-export utility functions
pub use legacy_types::local_date_today;
