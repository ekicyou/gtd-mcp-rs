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

