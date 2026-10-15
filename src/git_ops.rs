use anyhow::{Context, Result};
use git2::{
    Config, Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks, Repository,
    Signature, Time,
};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
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

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(Self::remote_callbacks(repo.config().ok()));
        remote
            .fetch(&[&branch_name], Some(&mut fetch_options), None)
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
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(Self::remote_callbacks(repo.config().ok()));
        remote
            .push(&[&refspec], Some(&mut push_options))
            .context("Failed to push to origin")?;

        Ok(())
    }

    /// Build remote callbacks that supply credentials for fetch/push
    ///
    /// libgit2 does not consult git credential helpers by itself, so HTTPS
    /// remotes that rely on helpers (e.g. `gh auth git-credential`) fail with
    /// 401 unless credentials are provided here. Each method is tried at most
    /// once, because libgit2 re-invokes the callback after a rejected attempt.
    ///
    /// Order of attempts:
    /// 1. `git credential fill` (resolves helpers exactly like the git CLI)
    /// 2. git2's built-in credential helper support
    /// 3. SSH agent (for SSH remotes)
    /// 4. Default credentials (e.g. Negotiate/NTLM)
    ///
    /// # Arguments
    /// * `config` - Repository Git config used by the built-in helper fallback
    ///
    /// # Returns
    /// RemoteCallbacks with a credentials handler
    fn remote_callbacks(config: Option<Config>) -> RemoteCallbacks<'static> {
        let mut tried_git_credential = false;
        let mut tried_helper = false;
        let mut tried_ssh_agent = false;
        let mut tried_username = false;
        let mut tried_default = false;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |url, username_from_url, allowed| {
            if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
                if !tried_git_credential {
                    tried_git_credential = true;
                    if let Some((username, password)) = git_credential_fill(url, username_from_url)
                    {
                        return Cred::userpass_plaintext(&username, &password);
                    }
                }
                if !tried_helper {
                    tried_helper = true;
                    if let Some(config) = &config
                        && let Ok(cred) = Cred::credential_helper(config, url, username_from_url)
                    {
                        return Ok(cred);
                    }
                }
            }

            if allowed.contains(CredentialType::USERNAME) && !tried_username {
                tried_username = true;
                return Cred::username(username_from_url.unwrap_or("git"));
            }

            if allowed.contains(CredentialType::SSH_KEY) && !tried_ssh_agent {
                tried_ssh_agent = true;
                return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
            }

            if allowed.contains(CredentialType::DEFAULT) && !tried_default {
                tried_default = true;
                return Cred::default();
            }

            Err(git2::Error::from_str(&format!(
                "No valid credentials for {}. Configure a git credential helper \
                 (e.g. `gh auth setup-git`) or an SSH agent.",
                url
            )))
        });
        callbacks
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

/// Obtain credentials via `git credential fill`
///
/// Delegates helper resolution to the installed git CLI so that every helper
/// configuration it supports (quoted paths, `!` shell commands, helper reset
/// with an empty value, Git Credential Manager, `gh auth git-credential`, ...)
/// behaves the same as on the command line. Terminal prompts are disabled so
/// the MCP server never blocks waiting for interactive input.
///
/// # Arguments
/// * `url` - Remote URL requiring authentication
/// * `username` - Username embedded in the URL, if any
///
/// # Returns
/// Optional (username, password) pair; None if git is unavailable or no helper answered
fn git_credential_fill(url: &str, username: Option<&str>) -> Option<(String, String)> {
    let mut input = format!("url={}\n", url);
    if let Some(username) = username {
        input.push_str(&format!("username={}\n", username));
    }
    input.push('\n');

    let mut command = Command::new("git");
    command
        .args(["credential", "fill"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().ok()?;
    // Dropping stdin after writing closes the pipe so git can proceed
    child.stdin.take()?.write_all(input.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse_credential_output(&String::from_utf8_lossy(&output.stdout))
}

/// Parse the `key=value` output of `git credential fill`
///
/// # Arguments
/// * `output` - Standard output of `git credential fill`
///
/// # Returns
/// Optional (username, password) pair; None if the password is missing
fn parse_credential_output(output: &str) -> Option<(String, String)> {
    let mut username = None;
    let mut password = None;
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("username=") {
            username = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("password=") {
            password = Some(value.to_string());
        }
    }
    Some((username.unwrap_or_default(), password?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credential_output() {
        let output = "protocol=https\nhost=github.com\nusername=alice\npassword=secret\n";
        assert_eq!(
            parse_credential_output(output),
            Some(("alice".to_string(), "secret".to_string()))
        );
    }

    #[test]
    fn test_parse_credential_output_without_password() {
        let output = "protocol=https\nhost=github.com\nusername=alice\n";
        assert_eq!(parse_credential_output(output), None);
    }

    #[test]
    fn test_parse_credential_output_with_crlf() {
        let output = "username=alice\r\npassword=secret\r\n";
        assert_eq!(
            parse_credential_output(output),
            Some(("alice".to_string(), "secret".to_string()))
        );
    }
}
