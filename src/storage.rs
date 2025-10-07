use crate::gtd::GtdData;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Storage {
    file_path: PathBuf,
}

impl Storage {
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<GtdData> {
        if !self.file_path.exists() {
            return Ok(GtdData::new());
        }

        let content = fs::read_to_string(&self.file_path)?;
        let data: GtdData = toml::from_str(&content)?;
        Ok(data)
    }

    pub fn save(&self, data: &GtdData) -> Result<()> {
        let content = toml::to_string_pretty(data)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtd::{Context, Project, ProjectStatus, Task, TaskStatus};
    use chrono::NaiveDate;
    use std::env;
    use std::fs;

    fn get_test_path(filename: &str) -> PathBuf {
        env::temp_dir().join(filename)
    }

    #[test]
    fn test_storage_new() {
        let test_path = get_test_path("test_gtd.toml");
        let storage = Storage::new(&test_path);
        assert_eq!(storage.file_path, test_path);
    }

    #[test]
    fn test_storage_load_nonexistent_file() {
        let test_path = get_test_path("nonexistent_test_gtd.toml");
        // Ensure file doesn't exist
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let result = storage.load();

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.tasks.is_empty());
        assert!(data.projects.is_empty());
        assert!(data.contexts.is_empty());
    }

    #[test]
    fn test_storage_save_and_load_empty_data() {
        let test_path = get_test_path("test_empty_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let data = GtdData::new();

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert!(loaded_data.tasks.is_empty());
        assert!(loaded_data.projects.is_empty());
        assert!(loaded_data.contexts.is_empty());

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_save_and_load_with_tasks() {
        let test_path = get_test_path("test_tasks_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Inbox,
            project: Some("project-1".to_string()),
            context: Some("context-1".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
        };
        data.tasks.insert(task.id.clone(), task.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.tasks.len(), 1);

        let loaded_task = loaded_data.tasks.get("task-1").unwrap();
        assert_eq!(loaded_task.title, "Test Task");
        assert_eq!(loaded_task.project, Some("project-1".to_string()));
        assert_eq!(loaded_task.context, Some("context-1".to_string()));
        assert_eq!(loaded_task.notes, Some("Test notes".to_string()));
        assert_eq!(
            loaded_task.start_date,
            NaiveDate::from_ymd_opt(2024, 12, 25)
        );

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_save_and_load_with_projects() {
        let test_path = get_test_path("test_projects_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: Some("Test description".to_string()),
            status: ProjectStatus::Active,
        };
        data.projects.insert(project.id.clone(), project.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.projects.len(), 1);

        let loaded_project = loaded_data.projects.get("project-1").unwrap();
        assert_eq!(loaded_project.name, "Test Project");
        assert_eq!(
            loaded_project.description,
            Some("Test description".to_string())
        );

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_save_and_load_with_contexts() {
        let test_path = get_test_path("test_contexts_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        let context = Context {
            id: "context-1".to_string(),
            name: "Office".to_string(),
        };
        data.contexts.insert(context.id.clone(), context.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.contexts.len(), 1);

        let loaded_context = loaded_data.contexts.get("context-1").unwrap();
        assert_eq!(loaded_context.name, "Office");

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_save_and_load_comprehensive() {
        let test_path = get_test_path("test_comprehensive_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        // Add multiple tasks
        for i in 1..=3 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: TaskStatus::Inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
            };
            data.tasks.insert(task.id.clone(), task);
        }

        // Add multiple projects
        for i in 1..=2 {
            let project = Project {
                id: format!("project-{}", i),
                name: format!("Project {}", i),
                description: None,
                status: ProjectStatus::Active,
            };
            data.projects.insert(project.id.clone(), project);
        }

        // Add multiple contexts
        for i in 1..=2 {
            let context = Context {
                id: format!("context-{}", i),
                name: format!("Context {}", i),
            };
            data.contexts.insert(context.id.clone(), context);
        }

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.tasks.len(), 3);
        assert_eq!(loaded_data.projects.len(), 2);
        assert_eq!(loaded_data.contexts.len(), 2);

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_overwrite_existing_file() {
        let test_path = get_test_path("test_overwrite_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);

        // First save
        let mut data1 = GtdData::new();
        let task1 = Task {
            id: "task-1".to_string(),
            title: "Original Task".to_string(),
            status: TaskStatus::Inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };
        data1.tasks.insert(task1.id.clone(), task1);
        storage.save(&data1).unwrap();

        // Second save (overwrite)
        let mut data2 = GtdData::new();
        let task2 = Task {
            id: "task-2".to_string(),
            title: "New Task".to_string(),
            status: TaskStatus::NextAction,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };
        data2.tasks.insert(task2.id.clone(), task2);
        storage.save(&data2).unwrap();

        // Load and verify
        let loaded_data = storage.load().unwrap();
        assert_eq!(loaded_data.tasks.len(), 1);
        assert!(loaded_data.tasks.contains_key("task-2"));
        assert!(!loaded_data.tasks.contains_key("task-1"));

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_invalid_toml() {
        let test_path = get_test_path("test_invalid_gtd.toml");

        // Write invalid TOML
        fs::write(&test_path, "this is not valid toml {{{{").unwrap();

        let storage = Storage::new(&test_path);
        let load_result = storage.load();

        assert!(load_result.is_err());

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    #[test]
    fn test_storage_different_status_values() {
        let test_path = get_test_path("test_status_gtd.toml");
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        let statuses = vec![
            TaskStatus::Inbox,
            TaskStatus::NextAction,
            TaskStatus::WaitingFor,
            TaskStatus::Someday,
            TaskStatus::Done,
            TaskStatus::Trash,
        ];

        for (i, status) in statuses.iter().enumerate() {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: status.clone(),
                project: None,
                context: None,
                notes: None,
                start_date: None,
            };
            data.tasks.insert(task.id.clone(), task);
        }

        storage.save(&data).unwrap();
        let loaded_data = storage.load().unwrap();

        assert_eq!(loaded_data.tasks.len(), 6);

        // Clean up
        let _ = fs::remove_file(&test_path);
    }
}
