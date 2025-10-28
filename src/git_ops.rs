use anyhow::{Context, Result};
use git2::{Repository, Signature, Time};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Git operations handler for automatic version control
///
/// Handles Git operations like commit, pull, and push for automatic versioning
/// of GTD data files. Detects if a file is in a Git repository and provides
/// operations to synchronize changes with a remote repository.
pub struct GitOps {
    /// Optional Git repository (None if file is not in a Git repository)
    repo_path: Option<Arc<Mutex<Repository>>>,
}

impl GitOps {
    /// Create a new GitOps instance by detecting if the path is in a Git repository
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to check for Git management
    ///
    /// # Returns
    /// A new GitOps instance
    pub fn new(file_path: &Path) -> Self {
        // Always use the parent directory for discovery, whether the file exists or not
        let file_dir = if file_path.is_file() {
            file_path.parent().unwrap_or(file_path).to_path_buf()
        } else {
            // If not a file, assume it's meant to be a file and use its parent
            file_path.parent().unwrap_or(file_path).to_path_buf()
        };

        let repo_path = Self::find_repository(&file_dir).map(|r| Arc::new(Mutex::new(r)));
        Self { repo_path }
    }

    /// Check if the file is under Git version control
    ///
    /// # Returns
    /// `true` if the file is in a Git repository, `false` otherwise
    pub fn is_git_managed(&self) -> bool {
        self.repo_path.is_some()
    }

    /// Find the Git repository containing the given path
    ///
    /// # Arguments
    /// * `dir` - Directory to search for a Git repository
    ///
    /// # Returns
    /// Optional Repository if found
    fn find_repository(dir: &Path) -> Option<Repository> {
        Repository::discover(dir).ok()
    }

    /// Pull changes from remote repository
    ///
    /// Fetches changes from the origin remote and performs a fast-forward merge
    /// if possible. Returns an error if a normal merge is required.
    ///
    /// # Returns
    /// Result indicating success or an error
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
    ///
    /// Stages the specified file and creates a commit with the given message.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to commit
    /// * `message` - Commit message
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn commit(&self, file_path: &Path, message: &str) -> Result<()> {
        let repo = match &self.repo_path {
            Some(r) => r.lock().unwrap(),
            None => return Ok(()), // Not a git repo, skip
        };

        // Get the file path relative to the repository
        // Canonicalize both paths to handle symlinks and platform differences
        let repo_workdir = repo
            .workdir()
            .context("Repository has no working directory")?;
        let canonical_workdir = repo_workdir
            .canonicalize()
            .context("Failed to canonicalize repository path")?;
        let canonical_file = file_path
            .canonicalize()
            .context("Failed to canonicalize file path")?;
        let relative_path = canonical_file
            .strip_prefix(&canonical_workdir)
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
    ///
    /// Pushes the current branch to the origin remote.
    ///
    /// # Returns
    /// Result indicating success or an error
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

    /// Get or create a Git signature for commits
    ///
    /// Uses the configured user.name and user.email from Git config,
    /// or falls back to default values if not configured.
    ///
    /// # Arguments
    /// * `repo` - The Git repository
    ///
    /// # Returns
    /// Result containing a Signature or an error
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

    /// Perform full Git synchronization: pull, commit, and push
    ///
    /// This is the main sync operation that ensures the file is up to date,
    /// commits changes, and pushes them to the remote repository.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to commit
    /// * `commit_message` - Commit message to use
    ///
    /// # Returns
    /// Result indicating success or an error
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
