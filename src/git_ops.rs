use anyhow::{Context, Result};
use git2::{Repository, Signature, Time};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Git operations handler for automatic version control
pub struct GitOps {
    repo_path: Option<Arc<Mutex<Repository>>>,
}

impl GitOps {
    /// Create a new GitOps instance by detecting if the path is in a git repository
    pub fn new(file_path: &Path) -> Self {
        let file_dir = if file_path.is_file() {
            file_path.parent().unwrap_or(file_path).to_path_buf()
        } else {
            file_path.to_path_buf()
        };

        let repo_path = Self::find_repository(&file_dir).map(|r| Arc::new(Mutex::new(r)));
        Self { repo_path }
    }

    /// Check if the file is under git version control
    pub fn is_git_managed(&self) -> bool {
        self.repo_path.is_some()
    }

    /// Find the git repository containing the given path
    fn find_repository(dir: &Path) -> Option<Repository> {
        Repository::discover(dir).ok()
    }

    /// Pull changes from remote repository
    pub fn pull(&self) -> Result<()> {
        let repo = match &self.repo_path {
            Some(r) => r.lock().unwrap(),
            None => return Ok(()), // Not a git repo, skip
        };

        // Get the current branch
        let head = repo.head().context("Failed to get HEAD")?;
        let branch_name = head
            .shorthand()
            .context("Failed to get branch name")?
            .to_string();

        // Fetch from origin
        let mut remote = repo
            .find_remote("origin")
            .context("Failed to find remote 'origin'")?;

        remote
            .fetch(&[&branch_name], None, None)
            .context("Failed to fetch from origin")?;

        // Get the fetch head
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

        // Perform merge analysis
        let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

        if analysis.is_up_to_date() {
            // Already up to date
            return Ok(());
        }

        if analysis.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else if analysis.is_normal() {
            // Normal merge - for simplicity, we'll skip this case
            // In a real implementation, you might want to handle conflicts
            return Err(anyhow::anyhow!(
                "Merge required but automatic merge is not supported. Please resolve manually."
            ));
        }

        Ok(())
    }

    /// Commit changes to the repository
    pub fn commit(&self, file_path: &Path, message: &str) -> Result<()> {
        let repo = match &self.repo_path {
            Some(r) => r.lock().unwrap(),
            None => return Ok(()), // Not a git repo, skip
        };

        // Get the file path relative to the repository
        let repo_workdir = repo
            .workdir()
            .context("Repository has no working directory")?;
        let relative_path = file_path
            .strip_prefix(repo_workdir)
            .context("File is not in repository")?;

        // Add the file to the index
        let mut index = repo.index()?;
        index.add_path(relative_path)?;
        index.write()?;

        // Check if there are changes to commit
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get the current HEAD commit
        let parent_commit = match repo.head() {
            Ok(head) => {
                let oid = head.target().context("HEAD has no target")?;
                Some(repo.find_commit(oid)?)
            }
            Err(_) => None, // Initial commit
        };

        // Create signature
        let signature = Self::get_signature(&repo)?;

        // Create the commit
        let parents: Vec<_> = parent_commit.iter().collect();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        Ok(())
    }

    /// Push changes to remote repository
    pub fn push(&self) -> Result<()> {
        let repo = match &self.repo_path {
            Some(r) => r.lock().unwrap(),
            None => return Ok(()), // Not a git repo, skip
        };

        // Get the current branch
        let head = repo.head().context("Failed to get HEAD")?;
        let branch_name = head
            .shorthand()
            .context("Failed to get branch name")?
            .to_string();

        // Get remote
        let mut remote = repo
            .find_remote("origin")
            .context("Failed to find remote 'origin'")?;

        // Push to remote
        let refspec = format!("refs/heads/{}", branch_name);
        remote.push(&[&refspec], None)?;

        Ok(())
    }

    /// Get or create a git signature for commits
    fn get_signature(repo: &Repository) -> Result<Signature<'_>> {
        // Try to use the configured user name and email
        let config = repo.config()?;

        let name = config
            .get_string("user.name")
            .unwrap_or_else(|_| "GTD MCP Server".to_string());

        let email = config
            .get_string("user.email")
            .unwrap_or_else(|_| "gtd-mcp@localhost".to_string());

        // Use Signature::now() but with a fallback to a fixed time if it fails
        match Signature::now(&name, &email) {
            Ok(sig) => Ok(sig),
            Err(_) => {
                // Fallback to a fixed time if now() fails (e.g., on some CI systems)
                let time = Time::new(1_700_000_000, 0); // Roughly Nov 2023
                Signature::new(&name, &email, &time)
                    .context("Failed to create signature with fixed time")
            }
        }
    }

    /// Perform full git sync: pull, commit, and push
    pub fn sync(&self, file_path: &Path, commit_message: &str) -> Result<()> {
        if !self.is_git_managed() {
            return Ok(());
        }

        // Pull first to get latest changes
        self.pull().context("Failed to pull changes")?;

        // Commit the changes
        self.commit(file_path, commit_message)
            .context("Failed to commit changes")?;

        // Push to remote
        self.push().context("Failed to push changes")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
}
