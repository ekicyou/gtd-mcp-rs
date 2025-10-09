use crate::git_ops::GitOps;
use crate::gtd::GtdData;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Storage {
    pub file_path: PathBuf,
    git_ops: GitOps,
}

impl Storage {
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        let file_path = file_path.as_ref().to_path_buf();
        let git_ops = GitOps::new(&file_path);
        Self { file_path, git_ops }
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

        // Perform git operations if in a git repository
        if self.git_ops.is_git_managed() {
            // Try to sync with git, but don't fail if git operations fail
            // This allows the application to continue working even if git is not configured
            if let Err(e) = self.git_ops.sync(&self.file_path, "Update GTD data") {
                eprintln!("Warning: Git sync failed: {}. Data saved locally.", e);
            }
        }

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

    // Storageインスタンスの作成テスト
    // 指定したパスでStorageが正しく初期化されることを確認
    #[test]
    fn test_storage_new() {
        let test_path = get_test_path("test_gtd.toml");
        let storage = Storage::new(&test_path);
        assert_eq!(storage.file_path, test_path);
    }

    // 存在しないファイルの読み込みテスト
    // ファイルが存在しない場合、空のGtdDataが返されることを確認
    #[test]
    fn test_storage_load_nonexistent_file() {
        let test_path = get_test_path("nonexistent_test_gtd.toml");
        // Ensure file doesn't exist
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let result = storage.load();

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.task_count(), 0);
        assert!(data.projects.is_empty());
        assert!(data.contexts.is_empty());
    }

    // 空データの保存と読み込みテスト
    // 空のGtdDataを保存し、読み込んでも空のままであることを確認
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
        assert_eq!(loaded_data.task_count(), 0);
        assert!(loaded_data.projects.is_empty());
        assert!(loaded_data.contexts.is_empty());

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // タスクを含むデータの保存と読み込みテスト
    // タスクを含むGtdDataを保存し、読み込んで全フィールドが正確に復元されることを確認
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
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("context-1".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.task_count(), 1);

        let loaded_task = loaded_data.find_task_by_id("task-1").unwrap();
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

    // プロジェクトを含むデータの保存と読み込みテスト
    // プロジェクトを含むGtdDataを保存し、読み込んで全フィールドが正確に復元されることを確認
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
            status: ProjectStatus::active,
        };
        data.add_project(project.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.projects.len(), 1);

        let loaded_project = loaded_data.find_project_by_id("project-1").unwrap();
        assert_eq!(loaded_project.name, "Test Project");
        assert_eq!(
            loaded_project.description,
            Some("Test description".to_string())
        );

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // コンテキストを含むデータの保存と読み込みテスト
    // コンテキストを含むGtdDataを保存し、読み込んで正確に復元されることを確認
    #[test]
    fn test_storage_save_and_load_with_contexts() {
        let test_path = get_test_path("test_contexts_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        let context = Context {
            name: "Office".to_string(),
            description: None,
        };
        data.add_context(context.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.contexts.len(), 1);

        let loaded_context = loaded_data.find_context_by_name("Office").unwrap();
        assert_eq!(loaded_context.name, "Office");

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // 包括的なデータの保存と読み込みテスト
    // タスク、プロジェクト、コンテキストを含む完全なGtdDataを保存し、読み込んですべて正確に復元されることを確認
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
                status: TaskStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            };
            data.add_task(task);
        }

        // Add multiple projects
        for i in 1..=2 {
            let project = Project {
                id: format!("project-{}", i),
                name: format!("Project {}", i),
                description: None,
                status: ProjectStatus::active,
            };
            data.add_project(project);
        }

        // Add multiple contexts
        for i in 1..=2 {
            let context = Context {
                name: format!("Context {}", i),
                description: None,
            };
            data.add_context(context);
        }

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.task_count(), 3);
        assert_eq!(loaded_data.projects.len(), 2);
        assert_eq!(loaded_data.contexts.len(), 2);

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // 既存ファイルの上書きテスト
    // 既存のファイルに新しいデータを保存し、古いデータが上書きされることを確認
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
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data1.add_task(task1);
        storage.save(&data1).unwrap();

        // Second save (overwrite)
        let mut data2 = GtdData::new();
        let task2 = Task {
            id: "task-2".to_string(),
            title: "New Task".to_string(),
            status: TaskStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data2.add_task(task2);
        storage.save(&data2).unwrap();

        // Load and verify
        let loaded_data = storage.load().unwrap();
        assert_eq!(loaded_data.task_count(), 1);
        assert!(loaded_data.find_task_by_id("task-2").is_some());
        assert!(loaded_data.find_task_by_id("task-1").is_none());

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // 不正なTOMLファイルの読み込みテスト
    // 無効なTOML形式のファイルを読み込もうとするとエラーが返されることを確認
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

    // 異なるタスクステータス値の保存と読み込みテスト
    // 6種類のタスクステータスすべてが正しく保存・読み込みされることを確認
    #[test]
    fn test_storage_different_status_values() {
        let test_path = get_test_path("test_status_gtd.toml");
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path);
        let mut data = GtdData::new();

        let statuses = [
            TaskStatus::inbox,
            TaskStatus::next_action,
            TaskStatus::waiting_for,
            TaskStatus::someday,
            TaskStatus::done,
            TaskStatus::trash,
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
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            };
            data.add_task(task);
        }

        storage.save(&data).unwrap();
        let loaded_data = storage.load().unwrap();

        assert_eq!(loaded_data.task_count(), 6);

        // Clean up
        let _ = fs::remove_file(&test_path);
    }
}
