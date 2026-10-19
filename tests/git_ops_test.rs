//! Unit tests for GitOps (Git version control layer)
//!
//! These tests verify the Git integration functionality,
//! including commit and sync operations.

use gtd_mcp::GitOps;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// git コマンドを実行して標準出力を返す
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// git リポジトリの初期化とテスト用ファイルの作成
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    git(temp_dir.path(), &["init", "-b", "main"]);

    // Configure git user for the test repo
    git(temp_dir.path(), &["config", "user.name", "Test User"]);
    git(
        temp_dir.path(),
        &["config", "user.email", "test@example.com"],
    );

    temp_dir
}

// 初期コミットを作成
fn create_initial_commit(temp_dir: &TempDir) {
    fs::write(temp_dir.path().join("test.txt"), "initial content").unwrap();
    git(temp_dir.path(), &["add", "test.txt"]);
    git(temp_dir.path(), &["commit", "-m", "Initial commit"]);
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
    let temp_dir = setup_test_repo();

    // Create an actual file in the repo so discover can work
    let file_path = temp_dir.path().join("test.toml");
    fs::write(&file_path, "test").unwrap();

    let git_ops = GitOps::new(&file_path);
    assert!(git_ops.is_git_managed());
}

// コミット機能のテスト
#[test]
fn test_commit() {
    let temp_dir = setup_test_repo();
    create_initial_commit(&temp_dir);

    let file_path = temp_dir.path().join("gtd.toml");
    fs::write(&file_path, "test content").unwrap();

    let git_ops = GitOps::new(&file_path);
    let result = git_ops.commit(&file_path, "Update gtd.toml");
    if let Err(e) = &result {
        eprintln!("Commit failed with error: {:?}", e);
    }
    assert!(result.is_ok(), "Commit should succeed");

    // Verify commit was created
    assert_eq!(
        git(temp_dir.path(), &["log", "-1", "--format=%s"]),
        "Update gtd.toml"
    );
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

// ベアリモートに対する pull → commit → push の同期テスト
#[test]
fn test_sync_with_remote() {
    let remote = TempDir::new().unwrap();
    git(remote.path(), &["init", "--bare", "-b", "main"]);

    let temp_dir = setup_test_repo();
    create_initial_commit(&temp_dir);
    let remote_path = remote.path().to_str().unwrap();
    git(temp_dir.path(), &["remote", "add", "origin", remote_path]);
    git(temp_dir.path(), &["push", "origin", "main"]);

    let file_path = temp_dir.path().join("gtd.toml");
    fs::write(&file_path, "synced").unwrap();

    let git_ops = GitOps::new(&file_path);
    git_ops.sync(&file_path, "Sync gtd.toml").unwrap();

    assert_eq!(
        git(remote.path(), &["log", "-1", "--format=%s", "main"]),
        "Sync gtd.toml"
    );

    // 変更なしで再同期してもエラーにならず、空コミットも作らない
    git_ops.sync(&file_path, "No change").unwrap();
    assert_eq!(
        git(remote.path(), &["log", "-1", "--format=%s", "main"]),
        "Sync gtd.toml"
    );
}
