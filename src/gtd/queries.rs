//! Query methods for GtdData
//!
//! This module contains query methods for filtering and accessing GtdData.
//! These are separated from the main gtd_data.rs to improve modularity.

use super::gtd_data::GtdData;
use super::nota::{Nota, NotaStatus};
use std::collections::HashMap;

impl GtdData {
    /// Get projects map (for compatibility)
    pub fn projects(&self) -> HashMap<String, &Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::project)
            .map(|n| (n.id.clone(), n))
            .collect()
    }

    /// Get contexts map (for compatibility)
    pub fn contexts(&self) -> HashMap<String, &Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::context)
            .map(|n| (n.id.clone(), n))
            .collect()
    }
}
