use crate::git_ops::GitOps;
#[allow(unused_imports)]
use crate::gtd::{GtdData, local_date_today};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize line endings to LF (\n) for internal use
/// This ensures consistent behavior when deserializing
fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

/// Convert line endings to OS-native format for file output
/// On Windows: LF (\n) -> CRLF (\r\n)
/// On Unix/Linux/macOS: LF (\n) remains as-is
#[cfg(target_os = "windows")]
fn to_native_line_endings(content: &str) -> String {
    // First normalize to LF, then convert to CRLF
    let normalized = normalize_line_endings(content);
    normalized.replace('\n', "\r\n")
}

#[cfg(not(target_os = "windows"))]
fn to_native_line_endings(content: &str) -> String {
    // On Unix-like systems, just normalize to LF
    normalize_line_endings(content)
}

/// Storage handler for GTD data persistence
///
/// Handles reading and writing GTD data to TOML files with optional Git synchronization.
/// Automatically manages line endings:
/// - Normalizes to LF on read for consistent parsing
/// - Converts to OS-native format on write (CRLF on Windows, LF on Unix)
pub struct Storage {
    /// Path to the GTD data file
    file_path: PathBuf,
    /// Git operations handler
    git_ops: GitOps,
    /// Whether to enable Git synchronization
    sync_git: bool,
}

impl Storage {
    /// Create a new Storage instance
    ///
    /// # Arguments
    /// * `file_path` - Path to the GTD data file
    /// * `sync_git` - Whether to enable automatic Git synchronization
    pub fn new(file_path: impl AsRef<Path>, sync_git: bool) -> Self {
        let file_path = file_path.as_ref().to_path_buf();
        let git_ops = GitOps::new(&file_path);
        Self {
            file_path,
            git_ops,
            sync_git,
        }
    }

    /// Get the path to the GTD data file
    ///
    /// # Returns
    /// Reference to the file path
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Load GTD data from the storage file
    ///
    /// If Git sync is enabled, pulls changes from remote before loading.
    /// Returns an empty GtdData instance if the file doesn't exist.
    ///
    /// # Returns
    /// Result containing the loaded GtdData or an error
    pub fn load(&self) -> Result<GtdData> {
        // Pull from git before loading if sync is enabled
        if self.sync_git && self.git_ops.is_git_managed() {
            self.git_ops.pull()?;
        }

        if !self.file_path.exists() {
            return Ok(GtdData::new());
        }

        let content = fs::read_to_string(&self.file_path)?;
        // Normalize line endings to LF for consistent parsing
        let normalized_content = normalize_line_endings(&content);
        let data: GtdData = toml::from_str(&normalized_content)?;
        Ok(data)
    }

    /// Save GTD data to the storage file with a default commit message
    ///
    /// # Arguments
    /// * `data` - The GtdData to save
    ///
    /// # Returns
    /// Result indicating success or an error
    #[allow(dead_code)]
    pub fn save(&self, data: &GtdData) -> Result<()> {
        self.save_with_message(data, "Update GTD data")
    }

    /// Save GTD data to the storage file with a custom commit message
    ///
    /// Serializes the data to TOML format with OS-native line endings.
    /// If Git sync is enabled and the file is in a Git repository,
    /// automatically commits and syncs changes.
    ///
    /// # Arguments
    /// * `data` - The GtdData to save
    /// * `commit_message` - Git commit message to use
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn save_with_message(&self, data: &GtdData, commit_message: &str) -> Result<()> {
        let content = toml::to_string_pretty(data)?;

        // Convert to OS-native line endings for file output
        let native_content = to_native_line_endings(&content);

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.file_path, native_content)?;

        // Perform git operations only if sync_git flag is enabled and in a git repository
        if self.sync_git && self.git_ops.is_git_managed() {
            // Propagate git errors to the caller so they can be returned to MCP client
            self.git_ops.sync(&self.file_path, commit_message)?;
        }

        Ok(())
    }

    /// Push changes to Git on shutdown
    ///
    /// Called when the server is shutting down to ensure all local commits
    /// are pushed to the remote repository.
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn shutdown(&self) -> Result<()> {
        if self.sync_git && self.git_ops.is_git_managed() {
            self.git_ops.push()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtd::NotaStatus;
    use crate::migration::{Context, Project, Task};
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
        let storage = Storage::new(&test_path, false);
        assert_eq!(storage.file_path(), test_path);
    }

    // 存在しないファイルの読み込みテスト
    // ファイルが存在しない場合、空のGtdDataが返されることを確認
    #[test]
    fn test_storage_load_nonexistent_file() {
        let test_path = get_test_path("nonexistent_test_gtd.toml");
        // Ensure file doesn't exist
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path, false);
        let result = storage.load();

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.task_count(), 0);
        assert!(data.projects().is_empty());
        assert!(data.contexts().is_empty());
    }

    // 空データの保存と読み込みテスト
    // 空のGtdDataを保存し、読み込んでも空のままであることを確認
    #[test]
    fn test_storage_save_and_load_empty_data() {
        let test_path = get_test_path("test_empty_gtd.toml");
        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path, false);
        let data = GtdData::new();

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.task_count(), 0);
        assert!(loaded_data.projects().is_empty());
        assert!(loaded_data.contexts().is_empty());

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

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        let project = Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: Some("Test description".to_string()),
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
            status: None,
        };
        data.add_project(project.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.projects().len(), 1);

        let loaded_project = loaded_data.find_project_by_id("project-1").unwrap();
        assert_eq!(loaded_project.title, "Test Project");
        assert_eq!(loaded_project.notes, Some("Test description".to_string()));

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

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        let context = Context {
            name: "Office".to_string(),
            notes: None,
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        };
        data.add_context(context.clone());

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.contexts().len(), 1);

        let loaded_context = loaded_data.find_context_by_name("Office").unwrap();
        assert_eq!(loaded_context.id, "Office");

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

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        // Add multiple tasks
        for i in 1..=3 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: NotaStatus::inbox,
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
                title: format!("Project {}", i),
                notes: None,
                project: None,
                context: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
                status: None,
            };
            data.add_project(project);
        }

        // Add multiple contexts
        for i in 1..=2 {
            let context = Context {
                name: format!(
                    "Context {
}",
                    i
                ),
                notes: None,
                title: None,
                status: NotaStatus::context,
                project: None,
                context: None,
                start_date: None,
                created_at: None,
                updated_at: None,
            };
            data.add_context(context);
        }

        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        let load_result = storage.load();
        assert!(load_result.is_ok());

        let loaded_data = load_result.unwrap();
        assert_eq!(loaded_data.task_count(), 3);
        assert_eq!(loaded_data.projects().len(), 2);
        assert_eq!(loaded_data.contexts().len(), 2);

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

        let storage = Storage::new(&test_path, false);

        // First save
        let mut data1 = GtdData::new();
        let task1 = Task {
            id: "task-1".to_string(),
            title: "Original Task".to_string(),
            status: NotaStatus::inbox,
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
            status: NotaStatus::next_action,
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

        let storage = Storage::new(&test_path, false);
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

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        let statuses = [
            NotaStatus::inbox,
            NotaStatus::next_action,
            NotaStatus::waiting_for,
            NotaStatus::later,
            NotaStatus::calendar,
            NotaStatus::someday,
            NotaStatus::done,
            NotaStatus::trash,
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

        assert_eq!(loaded_data.task_count(), 8);

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // sync_gitフラグがfalseの場合、git同期が行われないことを確認
    #[test]
    fn test_storage_sync_git_flag_false() {
        let test_path = get_test_path("test_sync_git_false_gtd.toml");
        let _ = fs::remove_file(&test_path);

        // sync_git=falseでStorageを作成
        let storage = Storage::new(&test_path, false);
        let data = GtdData::new();

        // 保存が成功することを確認（git同期は行われない）
        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        // ファイルが作成されていることを確認
        assert!(test_path.exists());

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // sync_gitフラグがtrueの場合でも、git管理されていないファイルでは問題なく動作することを確認
    #[test]
    fn test_storage_sync_git_flag_true_non_git() {
        let test_path = get_test_path("test_sync_git_true_gtd.toml");
        let _ = fs::remove_file(&test_path);

        // sync_git=trueでStorageを作成（ただしgit管理されていない場所）
        let storage = Storage::new(&test_path, true);
        let data = GtdData::new();

        // 保存が成功することを確認（git管理されていないので同期は行われない）
        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        // ファイルが作成されていることを確認
        assert!(test_path.exists());

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // 親ディレクトリが存在しない場合でもファイル作成が成功することを確認
    #[test]
    fn test_storage_create_file_with_missing_parent_directory() {
        let test_dir = env::temp_dir().join("test_gtd_nested_dir");
        let test_path = test_dir.join("subdir").join("test_gtd.toml");

        // Clean up if exists
        let _ = fs::remove_dir_all(&test_dir);

        // 親ディレクトリが存在しないことを確認
        assert!(!test_path.parent().unwrap().exists());

        let storage = Storage::new(&test_path, false);
        let data = GtdData::new();

        // 保存が成功することを確認（親ディレクトリが自動作成される）
        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        // ファイルが作成されていることを確認
        assert!(test_path.exists());

        // 読み込みも成功することを確認
        let load_result = storage.load();
        assert!(load_result.is_ok());

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);
    }

    // 存在しないファイルに対してsaveを実行すると、ファイルが作成されることを確認
    #[test]
    fn test_storage_save_creates_file() {
        let test_path = get_test_path("test_create_new_file_gtd.toml");

        // Clean up if exists
        let _ = fs::remove_file(&test_path);

        // ファイルが存在しないことを確認
        assert!(!test_path.exists());

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        // タスクを追加
        let task = Task {
            id: "test-1".to_string(),
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

        // 保存が成功することを確認
        let save_result = storage.save(&data);
        assert!(save_result.is_ok());

        // ファイルが作成されていることを確認
        assert!(test_path.exists());

        // 読み込んで内容が一致することを確認
        let loaded_data = storage.load().unwrap();
        assert_eq!(loaded_data.task_count(), 1);

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // git管理下でのload時のpull動作テスト
    #[test]
    fn test_storage_git_pull_on_load() {
        use git2::{Repository, Signature, Time};
        use tempfile::TempDir;

        // テスト用のgitリポジトリを作成
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Configure git user
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create an initial commit first so HEAD exists
        let dummy_file = temp_dir.path().join("dummy.txt");
        fs::write(&dummy_file, "dummy").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("dummy.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let time = Time::new(1_700_000_000, 0);
        let signature = Signature::new("Test User", "test@example.com", &time).unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

        // Now create gtd.toml
        let test_path = temp_dir.path().join("gtd.toml");
        fs::write(&test_path, "# test").unwrap();

        let storage = Storage::new(&test_path, true);

        // loadを呼び出すと、pullがremoteを探してエラーが返されることを確認
        // (Before fix: errors were silently ignored. After fix: errors are propagated)
        let loaded_data = storage.load();
        assert!(loaded_data.is_err());
    }

    // git管理下でのsave時のsync動作テスト
    #[test]
    fn test_storage_git_sync_on_save() {
        use git2::{Repository, Signature, Time};
        use tempfile::TempDir;

        // テスト用のgitリポジトリを作成
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Configure git user
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // 初期コミットを作成
        let dummy_file = temp_dir.path().join("dummy.txt");
        fs::write(&dummy_file, "dummy").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("dummy.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let time = Time::new(1_700_000_000, 0);
        let signature = Signature::new("Test User", "test@example.com", &time).unwrap();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

        // gtd.tomlファイルを作成して保存
        let test_path = temp_dir.path().join("gtd.toml");
        let storage = Storage::new(&test_path, true);
        let mut data = GtdData::new();

        let task = Task {
            id: "test-1".to_string(),
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

        // 保存を試みるが、git syncがremoteを探してエラーで失敗することを確認
        // (Before fix: git errors were silently ignored. After fix: errors are propagated)
        let save_result = storage.save(&data);
        assert!(save_result.is_err());

        // ファイルは作成されているが、gitコミットは完了していない
        assert!(test_path.exists());

        // sync_git=falseで再度読み込めば、ファイルの内容は読める
        let storage_no_sync = Storage::new(&test_path, false);
        let loaded_data = storage_no_sync.load().unwrap();
        assert_eq!(loaded_data.task_count(), 1);
    }

    // shutdown時のpush動作テスト
    #[test]
    fn test_storage_git_push_on_shutdown() {
        use git2::Repository;
        use tempfile::TempDir;

        // テスト用のgitリポジトリを作成
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Configure git user
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create a commit so HEAD exists
        let dummy_file = temp_dir.path().join("dummy.txt");
        fs::write(&dummy_file, "dummy").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("dummy.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let time = git2::Time::new(1_700_000_000, 0);
        let signature = git2::Signature::new("Test User", "test@example.com", &time).unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

        let test_path = temp_dir.path().join("gtd.toml");
        let storage = Storage::new(&test_path, true);

        // Verify that sync_git is enabled and git is detected
        assert!(storage.sync_git);
        assert!(storage.git_ops.is_git_managed());

        // shutdownを呼び出すと、remoteがないのでエラーが返されることを確認
        // (Before fix: errors were silently ignored. After fix: errors are propagated)
        let shutdown_result = storage.shutdown();
        // With HEAD but no remote, push will fail at find_remote
        assert!(shutdown_result.is_err());
    }

    // Test line ending normalization on load
    #[test]
    fn test_storage_normalize_line_endings_on_load() {
        let test_path = get_test_path("test_line_endings_gtd.toml");
        let _ = fs::remove_file(&test_path);

        // Create TOML file with CRLF line endings
        let toml_with_crlf = "[[inbox]]\r\nid = \"#1\"\r\ntitle = \"Test Task\"\r\ncreated_at = \"2024-01-01\"\r\nupdated_at = \"2024-01-01\"\r\n";
        fs::write(&test_path, toml_with_crlf).unwrap();

        let storage = Storage::new(&test_path, false);
        let load_result = storage.load();

        // Should load successfully despite CRLF
        assert!(load_result.is_ok());
        let data = load_result.unwrap();
        assert_eq!(data.task_count(), 1);

        let task = data.find_task_by_id("#1").unwrap();
        assert_eq!(task.title, "Test Task");

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // Test line endings in multi-line strings (notes field)
    #[test]
    fn test_storage_multiline_notes_line_endings() {
        let test_path = get_test_path("test_multiline_notes_gtd.toml");
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        // Create task with multi-line notes
        let task = Task {
            id: "#1".to_string(),
            title: "Task with notes".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: Some("Line 1\nLine 2\nLine 3".to_string()),
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);

        // Save and load
        storage.save(&data).unwrap();
        let loaded_data = storage.load().unwrap();

        // Verify notes are preserved with normalized line endings
        let loaded_task = loaded_data.find_task_by_id("#1").unwrap();
        assert_eq!(
            loaded_task.notes,
            Some("Line 1\nLine 2\nLine 3".to_string())
        );

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // Test that file is written with OS-native line endings
    #[test]
    fn test_storage_file_has_native_line_endings() {
        let test_path = get_test_path("test_native_endings_gtd.toml");
        let _ = fs::remove_file(&test_path);

        let storage = Storage::new(&test_path, false);
        let mut data = GtdData::new();

        let task = Task {
            id: "#1".to_string(),
            title: "Test".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);

        storage.save(&data).unwrap();

        // Read raw file content to check line endings
        let raw_content = fs::read_to_string(&test_path).unwrap();

        #[cfg(target_os = "windows")]
        {
            // On Windows, should have CRLF
            assert!(
                raw_content.contains("\r\n"),
                "File should contain CRLF on Windows"
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix-like systems, should only have LF
            assert!(
                !raw_content.contains("\r\n"),
                "File should not contain CRLF on Unix"
            );
            assert!(raw_content.contains('\n'), "File should contain LF");
        }

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // Test that CR characters in existing TOML files are properly handled
    // This reproduces issue #87 where files with \r already saved don't get
    // converted back to CRLF on Windows
    #[test]
    fn test_storage_cr_in_notes_converted_to_native() {
        let test_path = get_test_path("test_cr_notes_gtd.toml");
        let _ = fs::remove_file(&test_path);

        // Create TOML file with CR characters in notes (simulating old data)
        let toml_with_cr = "[[inbox]]\nid = \"#1\"\ntitle = \"Test Task\"\nnotes = \"\"\"\nLine 1\rLine 2\rLine 3\r\"\"\"\ncreated_at = \"2024-01-01\"\nupdated_at = \"2024-01-01\"\n";
        fs::write(&test_path, toml_with_cr).unwrap();

        println!("Original file content:");
        let original = fs::read_to_string(&test_path).unwrap();
        for (i, ch) in original.chars().enumerate() {
            if ch == '\r' {
                println!("  Position {}: CR", i);
            }
        }

        // Load the data
        let storage = Storage::new(&test_path, false);
        let data = storage.load().unwrap();

        // Notes should be normalized to LF internally
        let task = data.find_task_by_id("#1").unwrap();
        println!("Loaded notes: {:?}", task.notes);
        assert_eq!(task.notes, Some("Line 1\nLine 2\nLine 3\n".to_string()));

        // Save the data back
        storage.save(&data).unwrap();

        // Read raw file to check line endings
        let raw_content = fs::read_to_string(&test_path).unwrap();
        println!("\nAfter save - file content:");
        for (i, ch) in raw_content.chars().enumerate() {
            if ch == '\r' {
                println!("  Position {}: CR", i);
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, the notes content should have CRLF
            assert!(
                raw_content.contains("Line 1\r\n"),
                "Notes should contain CRLF on Windows after resave"
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, notes should have LF only, no CR
            assert!(
                !raw_content.contains('\r'),
                "Notes should not contain any CR on Unix after resave"
            );
        }

        // Clean up
        let _ = fs::remove_file(&test_path);
    }

    // Test for issue #87: TOML files with \r escape sequences should be properly converted
    #[test]
    fn test_storage_toml_escaped_cr_normalized() {
        let test_path = get_test_path("test_escaped_cr_gtd.toml");
        let _ = fs::remove_file(&test_path);

        // Create TOML file with \r escape sequences as they would appear on disk
        // This simulates the problem described in issue #87
        let toml_with_escaped_cr = concat!(
            "[[later]]\n",
            "id = \"#32\"\n",
            "title = \"クラウドを育成する\"\n",
            "project = \"project-4\"\n",
            "context = \"自宅\"\n",
            "notes = \"\"\"https://example.com\\r\\r",
            "Line 1\\r",
            "Line 2\\r",
            "Line 3\"\"\"\n",
            "created_at = \"2025-10-12\"\n",
            "updated_at = \"2025-10-12\"\n"
        );
        fs::write(&test_path, toml_with_escaped_cr).unwrap();

        // Verify the file contains \r escape sequences
        let file_content = fs::read_to_string(&test_path).unwrap();
        assert!(
            file_content.contains("\\r"),
            "Test file should contain \\r escape sequences"
        );

        // Load the data - the normalization should happen during deserialization
        let storage = Storage::new(&test_path, false);
        let data = storage.load().unwrap();

        // Notes should be normalized to LF internally
        let task = data.find_task_by_id("#32").unwrap();
        assert!(task.notes.is_some());
        let notes = task.notes.as_ref().unwrap();

        // Should not contain CR bytes
        assert!(
            !notes.as_bytes().contains(&b'\r'),
            "Loaded notes should not contain CR bytes"
        );
        // Should contain LF bytes
        assert!(
            notes.contains('\n'),
            "Loaded notes should contain LF characters"
        );

        // Save the data back
        storage.save(&data).unwrap();

        // Read raw file to verify line endings
        let raw_content = fs::read_to_string(&test_path).unwrap();

        #[cfg(target_os = "windows")]
        {
            // On Windows, should have CRLF, not escaped \r
            assert!(
                !raw_content.contains("\\r"),
                "Saved file should not contain \\r escape sequences on Windows"
            );
            assert!(
                raw_content.contains("\r\n"),
                "Saved file should contain CRLF on Windows"
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, should have LF, not escaped \r or actual CR
            assert!(
                !raw_content.contains("\\r"),
                "Saved file should not contain \\r escape sequences on Unix"
            );
            assert!(
                !raw_content.contains('\r'),
                "Saved file should not contain CR bytes on Unix"
            );
        }

        // Clean up
        let _ = fs::remove_file(&test_path);
    }
}

#[cfg(test)]
mod test_line_ending_normalization {
    #[allow(unused_imports)]
    use crate::gtd::{GtdData, local_date_today};

    // Test that CR normalization works for project descriptions
    #[test]
    fn test_project_description_cr_normalized() {
        // Create TOML with \r in project description
        let toml_input = concat!(
            "[[projects]]\n",
            "id = \"project-1\"\n",
            "title = \"Test Project\"\n",
            "notes = \"Line 1\\rLine 2\\rLine 3\"\n",
            "status = \"active\"\n"
        );

        let data: GtdData = toml::from_str(toml_input).unwrap();
        let project = data.find_project_by_id("project-1").unwrap();

        // Description should be normalized to LF
        assert!(project.notes.is_some());
        let desc = project.notes.as_ref().unwrap();
        assert!(!desc.as_bytes().contains(&b'\r'), "Should not contain CR");
        assert!(desc.contains('\n'), "Should contain LF");
    }

    // Test that CR normalization works for context descriptions
    #[test]
    fn test_context_description_cr_normalized() {
        // Create TOML with \r in context description
        let toml_input = concat!(
            "[contexts.Office]\n",
            "notes = \"Line 1\\rLine 2\\rLine 3\"\n"
        );

        let data: GtdData = toml::from_str(toml_input).unwrap();
        let context = data.find_context_by_name("Office").unwrap();

        // Description should be normalized to LF
        assert!(context.notes.is_some());
        let desc = context.notes.as_ref().unwrap();
        assert!(!desc.as_bytes().contains(&b'\r'), "Should not contain CR");
        assert!(desc.contains('\n'), "Should contain LF");
    }
}
