//! Unit tests for GitOps (Git version control layer)
//!
//! These tests verify the Git integration functionality,
//! including commit and sync operations.

use gtd_mcp::GitOps;
use git2::{Repository, Signature, Time};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// git リポジトリの初期化とテスト用ファイルの作成
fn setup_test_repo() -> (TempDir, Repository) {
    let temp_dir = TempDir::new().unwrap();
    let repo = Repository::init(temp_dir.path()).unwrap();

    // Configure git user for the test repo
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    (temp_dir, repo)
}

// 初期コミットを作成
fn create_initial_commit(repo: &Repository, temp_dir: &TempDir) {
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "initial content").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("test.txt")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    // Use a fixed time for signature to avoid CI issues
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
}

// git管理されていないディレクトリの検出テスト
#[test]
fn test_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.toml");

    let git_ops = GitOps::new(&file_path);
    assert!(!git_ops.is_git_managed());
}

// git管理されているディレクトリの検出テスト
#[test]
fn test_git_managed_directory() {
    let (temp_dir, _repo) = setup_test_repo();

    // Create an actual file in the repo so discover can work
    let file_path = temp_dir.path().join("test.toml");
    fs::write(&file_path, "test").unwrap();

    let git_ops = GitOps::new(&file_path);
    assert!(git_ops.is_git_managed());
}

// コミット機能のテスト
#[test]
fn test_commit() {
    let (temp_dir, repo) = setup_test_repo();
    create_initial_commit(&repo, &temp_dir);

    let file_path = temp_dir.path().join("gtd.toml");
    fs::write(&file_path, "test content").unwrap();

    let git_ops = GitOps::new(&file_path);
    let result = git_ops.commit(&file_path, "Update gtd.toml");
    if let Err(e) = &result {
        eprintln!("Commit failed with error: {:?}", e);
    }
    assert!(result.is_ok(), "Commit should succeed");

    // Verify commit was created
    let head = repo.head().unwrap();
    let commit = repo.find_commit(head.target().unwrap()).unwrap();
    assert_eq!(commit.message().unwrap(), "Update gtd.toml");
}

// git管理されていないファイルでのsync操作テスト
#[test]
fn test_sync_non_git_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.toml");
    fs::write(&file_path, "test").unwrap();

    let git_ops = GitOps::new(&file_path);
    let result = git_ops.sync(&file_path, "Test commit");
    // Should succeed but do nothing
    assert!(result.is_ok());
}
