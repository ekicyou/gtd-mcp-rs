use crate::gtd::nota::{Nota, NotaStatus, local_date_today};
use std::collections::HashMap;

pub struct GtdData {
    /// Format version for the TOML file (current: 3)
    pub format_version: u32,

    /// All GTD items stored as Nota objects in a Vec
    ///
    /// Vec is used as the primary storage for several reasons:
    /// 1. Maintains insertion order for consistent TOML serialization
    /// 2. Enables predictable iteration order for UI/display
    /// 3. Git-friendly: produces stable diffs when serialized to TOML
    /// 4. Simple ownership model - Vec owns all data directly
    pub(crate) notas: Vec<Nota>,

    /// HashMap index for O(1) duplicate ID detection
    ///
    /// This map stores ID → Status for fast duplicate checking when adding new notas.
    /// It does NOT contain references to the actual Nota objects - that would require
    /// Arc<RefCell<Nota>> and add significant complexity without measurable benefit
    /// for personal GTD usage scales (100-500 items).
    ///
    /// The map is kept in sync with the Vec during all mutating operations:
    /// - add_nota: inserts into both Vec and HashMap
    /// - remove_nota: removes from both Vec and HashMap
    /// - move_status: updates status in both Vec and HashMap
    ///
    /// This is NOT serialized to TOML - it's rebuilt from notas during deserialization.
    pub(crate) nota_map: HashMap<String, NotaStatus>,

    /// Counter for generating unique task IDs
    pub task_counter: u32,

    /// Counter for generating unique project IDs
    pub project_counter: u32,
}

impl Default for GtdData {
    fn default() -> Self {
        Self {
            format_version: 3,
            notas: Vec::new(),
            nota_map: HashMap::new(),
            task_counter: 0,
            project_counter: 0,
        }
    }
}

// Serialize/Deserialize implementations are in serde_impl.rs

impl GtdData {
    /// Create a new empty GtdData instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a new unique task ID
    pub fn generate_task_id(&mut self) -> String {
        self.task_counter += 1;
        format!("#{}", self.task_counter)
    }

    /// Count total number of task notas across all task statuses
    #[allow(dead_code)]
    pub fn task_count(&self) -> usize {
        self.notas.iter().filter(|n| n.is_task()).count()
    }

    /// Find a nota by its ID
    ///
    /// # Arguments
    /// * `id` - The nota ID to search for
    ///
    /// # Returns
    /// An optional reference to the nota if found
    #[allow(dead_code)]
    fn find_nota_by_id(&self, id: &str) -> Option<&Nota> {
        self.notas.iter().find(|n| n.id == id)
    }

    /// Find a nota by its ID and return a mutable reference
    ///
    /// # Arguments
    /// * `id` - The nota ID to search for
    ///
    /// # Returns
    /// An optional mutable reference to the nota if found
    fn find_nota_by_id_mut(&mut self, id: &str) -> Option<&mut Nota> {
        self.notas.iter_mut().find(|n| n.id == id)
    }

    /// Find a task by its ID (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The task ID to search for (e.g., "#1")
    ///
    /// # Returns
    /// An optional Nota reference if found and it's a task
    #[allow(dead_code)]
    pub fn find_task_by_id(&self, id: &str) -> Option<&Nota> {
        self.notas.iter().find(|n| n.id == id && n.is_task())
    }

    /// Find a task by its ID and return a mutable reference (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The task ID to search for (e.g., "#1")
    ///
    /// # Returns
    /// An optional mutable Nota reference if found and it's a task
    pub fn find_task_by_id_mut(&mut self, id: &str) -> Option<&mut Nota> {
        self.notas.iter_mut().find(|n| n.id == id && n.is_task())
    }

    /// Add a nota to the collection
    ///
    /// # Arguments
    /// * `nota` - The nota to add
    pub fn add_nota(&mut self, nota: Nota) {
        let id = nota.id.clone();
        let status = nota.status.clone();

        // Add to nota_map for duplicate checking
        self.nota_map.insert(id, status);

        // Add to notas vector
        self.notas.push(nota);
    }

    /// Remove a nota from the collection and return it
    ///
    /// # Arguments
    /// * `id` - The nota ID to remove
    ///
    /// # Returns
    /// The removed nota if found
    #[allow(dead_code)]
    pub fn remove_nota(&mut self, id: &str) -> Option<Nota> {
        // Find and remove nota
        if let Some(pos) = self.notas.iter().position(|n| n.id == id) {
            let nota = self.notas.remove(pos);
            self.nota_map.remove(id);
            Some(nota)
        } else {
            None
        }
    }

    /// Move a nota to a different status
    ///
    /// This method updates the status of the nota.
    ///
    /// # Arguments
    /// * `id` - The nota ID to move
    /// * `new_status` - The target status
    ///
    /// # Returns
    /// `Some(())` if the nota was found and moved, `None` otherwise
    pub fn move_status(&mut self, id: &str, new_status: NotaStatus) -> Option<()> {
        if let Some(nota) = self.find_nota_by_id_mut(id) {
            nota.status = new_status.clone();
            nota.updated_at = local_date_today();
            self.nota_map.insert(id.to_string(), new_status);
            Some(())
        } else {
            None
        }
    }

    /// Find a project by its ID (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The project ID to search for (e.g., "project-1")
    ///
    /// # Returns
    /// An optional reference to the nota if found and it's a project
    #[allow(dead_code)]
    pub fn find_project_by_id(&self, id: &str) -> Option<&Nota> {
        self.notas
            .iter()
            .find(|n| n.id == id && n.status == NotaStatus::project)
    }

    /// Find a context by its name (for compatibility)
    ///
    /// # Arguments
    /// * `name` - The context name to search for (e.g., "Office")
    ///
    /// # Returns
    /// An optional reference to the nota if found and it's a context
    #[allow(dead_code)]
    pub fn find_context_by_name(&self, name: &str) -> Option<&Nota> {
        self.notas
            .iter()
            .find(|n| n.id == name && n.status == NotaStatus::context)
    }

    /// Validate that a nota's project reference exists (if specified)
    /// Returns true if the nota has no project reference or if the reference is valid
    pub fn validate_nota_project(&self, nota: &Nota) -> bool {
        match &nota.project {
            None => true,
            Some(project_id) => self.find_project_by_id(project_id).is_some(),
        }
    }

    /// Validate that a nota's context reference exists (if specified)
    /// Returns true if the nota has no context reference or if the reference is valid
    pub fn validate_nota_context(&self, nota: &Nota) -> bool {
        match &nota.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Update project ID references in all notas
    ///
    /// When a project ID changes, this method updates all nota references
    /// from the old ID to the new ID.
    ///
    /// # Arguments
    /// * `old_id` - The old project ID
    /// * `new_id` - The new project ID
    pub fn update_project_id_in_notas(&mut self, old_id: &str, new_id: &str) {
        for nota in self.notas.iter_mut() {
            if let Some(ref project_id) = nota.project
                && project_id == old_id
            {
                nota.project = Some(new_id.to_string());
            }
        }
    }

    /// Add a nota (unified task/project/context)
    ///
    /// # Arguments
    /// * `nota` - The nota to add
    #[allow(dead_code)]
    pub fn add(&mut self, nota: Nota) {
        self.add_nota(nota);
    }

    /// Find a nota by its ID
    ///
    /// Searches across all notas.
    ///
    /// # Arguments
    /// * `id` - The nota ID to search for
    ///
    /// # Returns
    /// An optional Nota if found
    #[allow(dead_code)]
    pub fn find_by_id(&self, id: &str) -> Option<Nota> {
        self.find_nota_by_id(id).cloned()
    }

    /// Update a nota by its ID
    ///
    /// # Arguments
    /// * `id` - The nota ID to update
    /// * `nota` - The new nota data
    ///
    /// # Returns
    /// The old nota if found and replaced
    pub fn update(&mut self, id: &str, nota: Nota) -> Option<Nota> {
        if let Some(pos) = self.notas.iter().position(|n| n.id == id) {
            let old_nota = self.notas.remove(pos);
            self.notas.push(nota.clone());
            self.nota_map.insert(nota.id.clone(), nota.status.clone());
            Some(old_nota)
        } else {
            None
        }
    }

    /// List all notas with optional status filter
    ///
    /// # Arguments
    /// * `status_filter` - Optional status to filter by
    ///
    /// # Returns
    /// Vector of Nota objects matching the filter
    #[allow(dead_code)]
    pub fn list_all(&self, status_filter: Option<NotaStatus>) -> Vec<Nota> {
        if let Some(status) = status_filter {
            self.notas
                .iter()
                .filter(|n| n.status == status)
                .cloned()
                .collect()
        } else {
            self.notas.clone()
        }
    }

    /// Check if a nota ID is referenced by other notas
    ///
    /// Returns true if the ID is used in any nota's project or context fields.
    ///
    /// # Arguments
    /// * `id` - The nota ID to check
    ///
    /// # Returns
    /// True if the ID is referenced by other notas
    #[allow(dead_code)]
    pub fn is_referenced(&self, id: &str) -> bool {
        self.notas
            .iter()
            .any(|nota| nota.project.as_deref() == Some(id) || nota.context.as_deref() == Some(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::Task;
    use chrono::NaiveDate;

    // Tests for task_map HashMap functionality
    #[test]
    fn test_task_map_prevents_duplicate_ids() {
        let mut data = GtdData::new();

        // Add a task
        let task1 = Task {
            id: "test-task".to_string(),
            title: "Test Task 1".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task1);

        // Verify task is in map
        assert!(data.nota_map.contains_key("test-task"));
        assert_eq!(data.nota_map.get("test-task"), Some(&NotaStatus::inbox));

        // Try to add another task with same ID in a different status
        let task2 = Task {
            id: "test-task".to_string(),
            title: "Test Task 2".to_string(),
            status: NotaStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        // This would add a duplicate - the application layer (lib.rs) should check
        // the task_map before calling add_task
        // Here we just verify that task_map gets updated
        data.add_task(task2);

        // The task_map should now show the new status (last one wins)
        assert_eq!(
            data.nota_map.get("test-task"),
            Some(&NotaStatus::next_action)
        );

        // But there are actually TWO tasks with same ID (one in inbox, one in next_action)
        // This demonstrates why the application layer MUST check task_map before adding
        assert_eq!(data.inbox().len(), 1);
        assert_eq!(data.next_action().len(), 1);
    }

    #[test]
    fn test_task_map_updated_on_remove() {
        let mut data = GtdData::new();

        let task = Task {
            id: "remove-test".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);

        // Verify task is in map
        assert!(data.nota_map.contains_key("remove-test"));

        // Remove task
        let removed = data.remove_task("remove-test");
        assert!(removed.is_some());

        // Verify task is removed from map
        assert!(!data.nota_map.contains_key("remove-test"));
    }

    #[test]
    fn test_task_map_updated_on_status_change() {
        let mut data = GtdData::new();

        let task = Task {
            id: "status-test".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);

        // Verify initial status
        assert_eq!(data.nota_map.get("status-test"), Some(&NotaStatus::inbox));

        // Move to next_action
        data.move_status("status-test", NotaStatus::next_action);

        // Verify status updated in map
        assert_eq!(
            data.nota_map.get("status-test"),
            Some(&NotaStatus::next_action)
        );
    }

    #[test]
    fn test_task_map_rebuilt_from_toml() {
        // Test that task_map is correctly rebuilt when loading from TOML (format version 2)
        let toml_str = r#"
    format_version = 2

    [[inbox]]
    id = "task-1"
    title = "First task"
    created_at = "2024-01-01"
    updated_at = "2024-01-01"

    [[next_action]]
    id = "task-2"
    title = "Second task"
    created_at = "2024-01-01"
    updated_at = "2024-01-01"
    "#;

        let data: GtdData = toml::from_str(toml_str).unwrap();

        // Verify both tasks are in task_map with correct statuses
        assert_eq!(data.nota_map.len(), 2);
        assert_eq!(data.nota_map.get("task-1"), Some(&NotaStatus::inbox));
        assert_eq!(data.nota_map.get("task-2"), Some(&NotaStatus::next_action));
    }

    // ============================================================================
    // Design Validation Tests: HashMap vs Arc Pattern
    // ============================================================================
    //
    // These tests validate the design decision to use HashMap<String, NotaStatus>
    // for duplicate checking only, rather than Arc<RefCell<Nota>> for data access.
    //
    // The current design trades O(n) lookup for simplicity and maintainability,
    // which is appropriate for personal GTD usage (100-500 items).

    /// Test that nota_map correctly tracks all nota IDs and statuses
    ///
    /// This validates that the HashMap is properly synchronized with the Vec
    /// during all operations (add, remove, status change).
    #[test]
    fn test_nota_map_synchronization() {
        let mut data = GtdData::new();

        // Add various nota types
        data.add(Nota {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });

        data.add(Nota {
            id: "proj-1".to_string(),
            title: "Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });

        data.add(Nota {
            id: "Office".to_string(),
            title: "Office".to_string(),
            status: NotaStatus::context,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });

        // Verify HashMap matches Vec
        assert_eq!(data.nota_map.len(), data.notas.len());
        assert_eq!(data.nota_map.len(), 3);

        // Verify all IDs are in map with correct status
        assert_eq!(data.nota_map.get("task-1"), Some(&NotaStatus::inbox));
        assert_eq!(data.nota_map.get("proj-1"), Some(&NotaStatus::project));
        assert_eq!(data.nota_map.get("Office"), Some(&NotaStatus::context));

        // Move status and verify map is updated
        data.move_status("task-1", NotaStatus::next_action);
        assert_eq!(data.nota_map.get("task-1"), Some(&NotaStatus::next_action));

        // Remove nota and verify map is updated
        data.remove_nota("proj-1");
        assert_eq!(data.nota_map.len(), 2);
        assert!(!data.nota_map.contains_key("proj-1"));
    }

    /// Test O(1) duplicate detection performance
    ///
    /// This validates that duplicate checking is fast (O(1)) even with many notas,
    /// which is the primary purpose of nota_map.
    #[test]
    fn test_nota_map_duplicate_detection() {
        let mut data = GtdData::new();

        // Add 100 notas
        for i in 0..100 {
            data.add(Nota {
                id: format!("nota-{}", i),
                title: format!("Nota {}", i),
                status: NotaStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
                ..Default::default()
            });
        }

        assert_eq!(data.nota_map.len(), 100);

        // Test duplicate detection is O(1) - doesn't scan the Vec
        assert!(data.nota_map.contains_key("nota-50"));
        assert!(data.nota_map.contains_key("nota-99"));
        assert!(!data.nota_map.contains_key("nota-100"));

        // Test that status is tracked correctly
        assert_eq!(data.nota_map.get("nota-50"), Some(&NotaStatus::inbox));
    }

    /// Test that Vec maintains order for Git-friendly TOML output
    ///
    /// This validates a key benefit of Vec over HashMap - insertion order is preserved,
    /// making TOML diffs predictable and Git-friendly.
    #[test]
    fn test_vec_maintains_insertion_order() {
        let mut data = GtdData::new();

        // Add notas in specific order
        let ids = vec!["first", "second", "third", "fourth", "fifth"];
        for id in &ids {
            data.add(Nota {
                id: id.to_string(),
                title: format!("Nota {}", id),
                status: NotaStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
                ..Default::default()
            });
        }

        // Verify Vec maintains insertion order
        for (i, nota) in data.notas.iter().enumerate() {
            assert_eq!(nota.id, ids[i]);
        }

        // HashMap does NOT guarantee order (that's why we don't use it for primary storage)
        // This is a key design decision - Vec for ordered storage, HashMap for fast lookups
    }

    /// Test that nota_map is correctly rebuilt from TOML deserialization
    ///
    /// This validates that the HashMap is properly reconstructed when loading
    /// data from disk, maintaining synchronization with Vec.
    #[test]
    fn test_nota_map_rebuilt_on_deserialize() {
        let mut data = GtdData::new();

        // Add some notas
        data.add(Nota {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });

        data.add(Nota {
            id: "proj-1".to_string(),
            title: "Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });

        // Serialize to TOML
        let toml_str = toml::to_string(&data).unwrap();

        // nota_map should NOT be in TOML (it's not serialized)
        assert!(!toml_str.contains("nota_map"));

        // Deserialize
        let loaded: GtdData = toml::from_str(&toml_str).unwrap();

        // Verify nota_map was rebuilt correctly
        assert_eq!(loaded.nota_map.len(), 2);
        assert_eq!(loaded.nota_map.get("task-1"), Some(&NotaStatus::inbox));
        assert_eq!(loaded.nota_map.get("proj-1"), Some(&NotaStatus::project));

        // Verify Vec and HashMap are in sync
        assert_eq!(loaded.notas.len(), loaded.nota_map.len());
    }
}
