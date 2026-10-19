use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git operations handler for automatic version control
///
/// Handles Git operations like commit, pull, and push for automatic versioning
/// of GTD data files by running the git CLI found on PATH. Authentication,
/// credential helpers (e.g. `gh auth git-credential`) and commit identity all
/// follow the user's git configuration, exactly as on the command line.
pub struct GitOps {
    /// Directory containing the file (None if it is not in a Git work tree)
    repo_dir: Option<PathBuf>,
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
        let dir = file_path.parent().unwrap_or(file_path);
        let repo_dir = run_git(dir, &["rev-parse", "--show-toplevel"])
            .ok()
            .map(|_| dir.to_path_buf());
        Self { repo_dir }
    }

    /// Check if the file is under Git version control
    ///
    /// # Returns
    /// `true` if the file is in a Git repository, `false` otherwise
    pub fn is_git_managed(&self) -> bool {
        self.repo_dir.is_some()
    }

    /// Pull changes from remote repository
    ///
    /// Fetches the current branch from the origin remote and performs a
    /// fast-forward merge if possible. Returns an error if a normal merge is required.
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn pull(&self) -> Result<()> {
        let Some(dir) = &self.repo_dir else {
            return Ok(()); // Not a git repo, skip
        };

        let branch = current_branch(dir)?;
        run_git(dir, &["fetch", "origin", &branch]).context("Failed to fetch from origin")?;
        run_git(dir, &["merge", "--ff-only", "FETCH_HEAD"]).context(
            "Merge required but automatic merge is not supported. Please resolve manually.",
        )?;
        Ok(())
    }

    /// Commit changes to the repository
    ///
    /// Stages the specified file and creates a commit with the given message.
    /// Does nothing if there are no staged changes.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to commit
    /// * `message` - Commit message
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn commit(&self, file_path: &Path, message: &str) -> Result<()> {
        if self.repo_dir.is_none() {
            return Ok(()); // Not a git repo, skip
        }

        // Run from the file's own directory so no repository-relative path is needed
        let dir = file_path.parent().unwrap_or(Path::new(""));
        let file_name = file_path
            .file_name()
            .context("File path has no file name")?
            .to_string_lossy();
        run_git(dir, &["add", "--", &file_name])?;

        if run_git(dir, &["diff", "--cached", "--quiet"]).is_ok() {
            return Ok(()); // Nothing to commit
        }

        // Fall back to a fixed identity only where git has none configured (e.g. CI)
        let mut args = Vec::new();
        for (key, fallback) in [
            ("user.name", "GTD MCP Server"),
            ("user.email", "gtd-mcp@localhost"),
        ] {
            if run_git(dir, &["config", "--get", key]).is_err() {
                args.push("-c".to_string());
                args.push(format!("{}={}", key, fallback));
            }
        }
        args.extend(["commit", "-m", message].map(String::from));
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_git(dir, &args)?;
        Ok(())
    }

    /// Push changes to remote repository
    ///
    /// Pushes the current branch to the origin remote.
    ///
    /// # Returns
    /// Result indicating success or an error
    pub fn push(&self) -> Result<()> {
        let Some(dir) = &self.repo_dir else {
            return Ok(()); // Not a git repo, skip
        };

        let refspec = format!("refs/heads/{}", current_branch(dir)?);
        run_git(dir, &["push", "origin", &refspec]).context("Failed to push to origin")?;
        Ok(())
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

/// Get the name of the checked-out branch
fn current_branch(dir: &Path) -> Result<String> {
    run_git(dir, &["symbolic-ref", "--short", "HEAD"]).context("Failed to get branch name")
}

/// Run `git -C <dir> <args>` and return its trimmed standard output
///
/// Output is captured so the MCP stdio stream stays clean, terminal prompts are
/// disabled so the server never blocks on interactive input, and on Windows no
/// console window is opened. A failure carries git's stderr.
///
/// # Arguments
/// * `dir` - Directory to run git in
/// * `args` - Arguments after `-C <dir>`
///
/// # Returns
/// Result containing standard output or an error with git's stderr
fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .context("Failed to run git; --sync-git requires git on PATH")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
