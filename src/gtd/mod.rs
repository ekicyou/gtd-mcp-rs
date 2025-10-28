//! GTD domain models and business logic
//!
//! This module contains the core GTD data structures and their implementations.
//! It is split into submodules for better organization:
//! - `nota`: Unified nota structure (tasks, projects, contexts)
//! - `gtd_data`: Main data container with all GTD operations

mod gtd_data;
mod nota;

// Re-export all public types
pub use gtd_data::GtdData;
pub use nota::{Nota, NotaStatus, RecurrencePattern, local_date_today};
