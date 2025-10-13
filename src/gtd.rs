use chrono::{Local, NaiveDate};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// A GTD (Getting Things Done) task
///
/// Tasks represent individual actionable items in the GTD system.
/// Each task has a unique ID, title, status (inbox, next_action, etc.),
/// and optional metadata like project association, context, notes, and start date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier (e.g., "#1", "#2")
    pub id: String,
    /// Task title describing the action
    pub title: String,
    /// Current status of the task (inbox, next_action, waiting_for, etc.)
    #[serde(skip, default = "default_task_status")]
    pub status: TaskStatus,
    /// Optional project ID this task belongs to
    pub project: Option<String>,
    /// Optional context where this task can be performed (e.g., "@office", "@home")
    pub context: Option<String>,
    /// Optional additional notes in Markdown format
    pub notes: Option<String>,
    /// Optional start date for the task (format: YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,
    /// Date when the task was created
    pub created_at: NaiveDate,
    /// Date when the task was last updated
    pub updated_at: NaiveDate,
}

/// Default task status for deserialization
fn default_task_status() -> TaskStatus {
    TaskStatus::inbox
}

/// Get the current date in local timezone
pub fn local_date_today() -> NaiveDate {
    Local::now().date_naive()
}

/// Task status in the GTD workflow
///
/// Represents the different states a task can be in according to GTD methodology.
/// Uses snake_case naming to match TOML serialization format.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Tasks that have not been processed yet
    inbox,
    /// Tasks that are ready to be done immediately
    next_action,
    /// Tasks waiting for someone else or an external event
    waiting_for,
    /// Tasks to be done later (not immediately actionable)
    later,
    /// Tasks scheduled for a specific date
    calendar,
    /// Tasks that might be done someday but not now
    someday,
    /// Completed tasks
    done,
    /// Deleted or discarded tasks
    trash,
}

/// A GTD project
///
/// Projects represent multi-step outcomes that require more than one action.
/// Each project has a unique ID, name, status, and optional description and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier (e.g., "project-1", "project-2")
    /// Not serialized to TOML (used as HashMap key)
    #[serde(skip_serializing, default)]
    pub id: String,
    /// Project name
    pub name: String,
    /// Optional project description
    pub description: Option<String>,
    /// Current status of the project
    pub status: ProjectStatus,
    /// Optional context where this project can be worked on
    pub context: Option<String>,
}

/// Project status
///
/// Represents the current state of a project.
/// Uses snake_case naming to match TOML serialization format.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    /// Project is currently being worked on
    active,
    /// Project is paused or waiting
    on_hold,
    /// Project has been finished
    completed,
}

/// A GTD context
///
/// Contexts represent locations, tools, or situations where tasks can be performed
/// (e.g., "@office", "@home", "@computer", "@phone", "@errands").
/// The name field is maintained internally but not serialized to TOML
/// to avoid redundancy with the HashMap key in GtdData.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Context name (e.g., "Office", "Home") - not serialized to TOML
    #[serde(skip_serializing, default)]
    pub name: String,
    /// Optional description of the context
    pub description: Option<String>,
}

/// The main GTD data structure
///
/// This struct holds all tasks organized by status, along with projects and contexts.
/// Tasks are stored in separate vectors based on their status to facilitate
/// efficient serialization to TOML with a clear, human-readable structure.
///
/// The data is designed to be serialized to/from TOML format for persistent storage.
///
/// ## Format Versions
///
/// - Version 1 (default for old files): Projects stored as `Vec<Project>` (TOML: `[[projects]]`)
/// - Version 2 (current): Projects stored as `HashMap<String, Project>` (TOML: `[projects.id]`)
///
/// The deserializer automatically migrates from version 1 to version 2 on load.
#[derive(Debug, Serialize)]
pub struct GtdData {
    /// Format version for the TOML file (current: 2)
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    /// Tasks in the inbox (not yet processed)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inbox: Vec<Task>,
    /// Tasks marked as next actions (ready to do)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_action: Vec<Task>,
    /// Tasks waiting for external input
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_for: Vec<Task>,
    /// Tasks to be done later
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub later: Vec<Task>,
    /// Tasks scheduled for specific dates
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calendar: Vec<Task>,
    /// Tasks that might be done someday
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub someday: Vec<Task>,
    /// Completed tasks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<Task>,
    /// Deleted tasks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trash: Vec<Task>,
    /// All projects (keyed by ID)
    pub projects: HashMap<String, Project>,
    /// All contexts (keyed by name)
    pub contexts: HashMap<String, Context>,
    /// Counter for generating unique task IDs
    #[serde(default, skip_serializing_if = "is_zero")]
    pub task_counter: u32,
    /// Counter for generating unique project IDs
    #[serde(default, skip_serializing_if = "is_zero")]
    pub project_counter: u32,
}

impl Default for GtdData {
    fn default() -> Self {
        Self {
            format_version: 2,
            inbox: Vec::new(),
            next_action: Vec::new(),
            waiting_for: Vec::new(),
            later: Vec::new(),
            calendar: Vec::new(),
            someday: Vec::new(),
            done: Vec::new(),
            trash: Vec::new(),
            projects: HashMap::new(),
            contexts: HashMap::new(),
            task_counter: 0,
            project_counter: 0,
        }
    }
}

/// Default format version for new files
#[allow(dead_code)] // Used by serde
fn default_format_version() -> u32 {
    2
}

/// Check if a counter value is zero (used for skipping serialization)
fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl<'de> Deserialize<'de> for GtdData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use crate::migration::{
            GtdDataMigrationHelper, migrate_projects_to_latest, normalize_context_line_endings,
            normalize_project_line_endings, normalize_task_line_endings, populate_context_names,
            populate_project_ids,
        };

        let mut helper = GtdDataMigrationHelper::deserialize(deserializer)?;

        // Migrate projects to latest format (HashMap)
        let mut projects = migrate_projects_to_latest(helper.projects);

        // Populate the name field in each Context from the HashMap key
        populate_context_names(&mut helper.contexts);

        // Populate the id field in each Project from the HashMap key
        populate_project_ids(&mut projects);

        // Normalize line endings in all string fields
        normalize_task_line_endings(&mut helper.inbox);
        normalize_task_line_endings(&mut helper.next_action);
        normalize_task_line_endings(&mut helper.waiting_for);
        normalize_task_line_endings(&mut helper.later);
        normalize_task_line_endings(&mut helper.calendar);
        normalize_task_line_endings(&mut helper.someday);
        normalize_task_line_endings(&mut helper.done);
        normalize_task_line_endings(&mut helper.trash);
        normalize_project_line_endings(&mut projects);
        normalize_context_line_endings(&mut helper.contexts);

        // Set the status field for each task based on which collection it's in
        for task in &mut helper.inbox {
            task.status = TaskStatus::inbox;
        }
        for task in &mut helper.next_action {
            task.status = TaskStatus::next_action;
        }
        for task in &mut helper.waiting_for {
            task.status = TaskStatus::waiting_for;
        }
        for task in &mut helper.later {
            task.status = TaskStatus::later;
        }
        for task in &mut helper.calendar {
            task.status = TaskStatus::calendar;
        }
        for task in &mut helper.someday {
            task.status = TaskStatus::someday;
        }
        for task in &mut helper.done {
            task.status = TaskStatus::done;
        }
        for task in &mut helper.trash {
            task.status = TaskStatus::trash;
        }

        Ok(GtdData {
            format_version: 2, // Always use version 2 for in-memory representation
            inbox: helper.inbox,
            next_action: helper.next_action,
            waiting_for: helper.waiting_for,
            later: helper.later,
            calendar: helper.calendar,
            someday: helper.someday,
            done: helper.done,
            trash: helper.trash,
            projects,
            contexts: helper.contexts,
            task_counter: helper.task_counter,
            project_counter: helper.project_counter,
        })
    }
}

impl GtdData {
    /// Create a new empty GtdData instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a new unique task ID
    pub fn generate_task_id(&mut self) -> String {
        self.task_counter += 1;
        format!("#{}", self.task_counter)
    }

    /// Get a reference to the task list for the given status
    #[allow(dead_code)]
    fn get_task_list(&self, status: &TaskStatus) -> &Vec<Task> {
        match status {
            TaskStatus::inbox => &self.inbox,
            TaskStatus::next_action => &self.next_action,
            TaskStatus::waiting_for => &self.waiting_for,
            TaskStatus::someday => &self.someday,
            TaskStatus::later => &self.later,
            TaskStatus::done => &self.done,
            TaskStatus::trash => &self.trash,
            TaskStatus::calendar => &self.calendar,
        }
    }

    /// Get a mutable reference to the task list for the given status
    fn get_task_list_mut(&mut self, status: &TaskStatus) -> &mut Vec<Task> {
        match status {
            TaskStatus::inbox => &mut self.inbox,
            TaskStatus::next_action => &mut self.next_action,
            TaskStatus::waiting_for => &mut self.waiting_for,
            TaskStatus::someday => &mut self.someday,
            TaskStatus::later => &mut self.later,
            TaskStatus::done => &mut self.done,
            TaskStatus::trash => &mut self.trash,
            TaskStatus::calendar => &mut self.calendar,
        }
    }

    /// Get all task lists as an array of references
    fn all_task_lists(&self) -> [&Vec<Task>; 8] {
        [
            &self.inbox,
            &self.next_action,
            &self.waiting_for,
            &self.someday,
            &self.later,
            &self.done,
            &self.trash,
            &self.calendar,
        ]
    }

    /// Get all task lists as an array of mutable references
    fn all_task_lists_mut(&mut self) -> [&mut Vec<Task>; 8] {
        [
            &mut self.inbox,
            &mut self.next_action,
            &mut self.waiting_for,
            &mut self.someday,
            &mut self.later,
            &mut self.done,
            &mut self.trash,
            &mut self.calendar,
        ]
    }

    /// Get all tasks as a single vector (for testing and compatibility)
    #[allow(dead_code)]
    pub fn all_tasks(&self) -> Vec<&Task> {
        let mut tasks = Vec::new();
        for list in self.all_task_lists() {
            tasks.extend(list.iter());
        }
        tasks
    }

    /// Count total number of tasks across all statuses
    #[allow(dead_code)]
    pub fn task_count(&self) -> usize {
        self.inbox.len()
            + self.next_action.len()
            + self.waiting_for.len()
            + self.someday.len()
            + self.later.len()
            + self.done.len()
            + self.trash.len()
            + self.calendar.len()
    }

    /// Find a task by its ID across all status containers
    ///
    /// # Arguments
    /// * `id` - The task ID to search for (e.g., "#1")
    ///
    /// # Returns
    /// An optional reference to the task if found
    #[allow(dead_code)]
    pub fn find_task_by_id(&self, id: &str) -> Option<&Task> {
        for list in self.all_task_lists() {
            if let Some(task) = list.iter().find(|t| t.id == id) {
                return Some(task);
            }
        }
        None
    }

    /// Find a task by its ID and return a mutable reference
    ///
    /// # Arguments
    /// * `id` - The task ID to search for (e.g., "#1")
    ///
    /// # Returns
    /// An optional mutable reference to the task if found
    pub fn find_task_by_id_mut(&mut self, id: &str) -> Option<&mut Task> {
        for list in self.all_task_lists_mut() {
            if let Some(task) = list.iter_mut().find(|t| t.id == id) {
                return Some(task);
            }
        }
        None
    }

    /// Add a task to the appropriate status container
    ///
    /// The task will be added to the container matching its status field.
    ///
    /// # Arguments
    /// * `task` - The task to add
    pub fn add_task(&mut self, task: Task) {
        let status = task.status.clone();
        self.get_task_list_mut(&status).push(task);
    }

    /// Remove a task from its container and return it
    ///
    /// # Arguments
    /// * `id` - The task ID to remove (e.g., "#1")
    ///
    /// # Returns
    /// The removed task if found
    #[allow(dead_code)]
    pub fn remove_task(&mut self, id: &str) -> Option<Task> {
        for list in self.all_task_lists_mut() {
            if let Some(pos) = list.iter().position(|t| t.id == id) {
                return Some(list.remove(pos));
            }
        }
        None
    }

    /// Move a task to a different status container
    ///
    /// This method removes the task from its current container, updates its status,
    /// and adds it to the new container.
    ///
    /// # Arguments
    /// * `id` - The task ID to move (e.g., "#1")
    /// * `new_status` - The target status
    ///
    /// # Returns
    /// `Some(())` if the task was found and moved, `None` otherwise
    pub fn move_status(&mut self, id: &str, new_status: TaskStatus) -> Option<()> {
        // Remove task from its current container
        let mut task = self.remove_task(id)?;

        // Update the task's status
        task.status = new_status;

        // Add task to the new status container
        self.add_task(task);

        Some(())
    }

    /// Find a project by its ID
    ///
    /// # Arguments
    /// * `id` - The project ID to search for (e.g., "project-1")
    ///
    /// # Returns
    /// An optional reference to the project if found
    #[allow(dead_code)]
    pub fn find_project_by_id(&self, id: &str) -> Option<&Project> {
        self.projects.get(id)
    }

    /// Find a project by its ID and return a mutable reference
    ///
    /// # Arguments
    /// * `id` - The project ID to search for (e.g., "project-1")
    ///
    /// # Returns
    /// An optional mutable reference to the project if found
    #[allow(dead_code)]
    pub fn find_project_by_id_mut(&mut self, id: &str) -> Option<&mut Project> {
        self.projects.get_mut(id)
    }

    /// Add a project to the projects map
    ///
    /// # Arguments
    /// * `project` - The project to add (will be keyed by its ID)
    pub fn add_project(&mut self, project: Project) {
        self.projects.insert(project.id.clone(), project);
    }

    /// Find a context by its name
    ///
    /// # Arguments
    /// * `name` - The context name to search for (e.g., "Office")
    ///
    /// # Returns
    /// An optional reference to the context if found
    #[allow(dead_code)]
    pub fn find_context_by_name(&self, name: &str) -> Option<&Context> {
        self.contexts.get(name)
    }

    /// Add a context to the contexts map
    ///
    /// # Arguments
    /// * `context` - The context to add (will be keyed by its name)
    #[allow(dead_code)]
    pub fn add_context(&mut self, context: Context) {
        self.contexts.insert(context.name.clone(), context);
    }

    /// Validate that a task's project reference exists (if specified)
    /// Returns true if the task has no project reference or if the reference is valid
    pub fn validate_task_project(&self, task: &Task) -> bool {
        match &task.project {
            None => true,
            Some(project_id) => self.find_project_by_id(project_id).is_some(),
        }
    }

    /// Validate that a task's context reference exists (if specified)
    /// Returns true if the task has no context reference or if the reference is valid
    pub fn validate_task_context(&self, task: &Task) -> bool {
        match &task.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Validate that a task's references (project and context) exist
    /// Returns true if all references are valid or not specified
    pub fn validate_task_references(&self, task: &Task) -> bool {
        self.validate_task_project(task) && self.validate_task_context(task)
    }

    /// Validate that a project's context reference exists (if specified)
    /// Returns true if the project has no context reference or if the reference is valid
    pub fn validate_project_context(&self, project: &Project) -> bool {
        match &project.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Update project ID references in all tasks
    ///
    /// When a project ID changes, this method updates all task references
    /// from the old ID to the new ID.
    ///
    /// # Arguments
    /// * `old_id` - The old project ID
    /// * `new_id` - The new project ID
    pub fn update_project_id_in_tasks(&mut self, old_id: &str, new_id: &str) {
        for task_list in self.all_task_lists_mut() {
            for task in task_list.iter_mut() {
                if let Some(ref project_id) = task.project
                    && project_id == old_id
                {
                    task.project = Some(new_id.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate};

    // GtdDataの新規作成テスト
    // 空のタスク、プロジェクト、コンテキストのHashMapが初期化されることを確認
    #[test]
    fn test_gtd_data_new() {
        let data = GtdData::new();
        assert!(data.inbox.is_empty());
        assert!(data.next_action.is_empty());
        assert!(data.waiting_for.is_empty());
        assert!(data.someday.is_empty());
        assert!(data.later.is_empty());
        assert!(data.done.is_empty());
        assert!(data.trash.is_empty());
        assert!(data.projects.is_empty());
        assert!(data.contexts.is_empty());
    }

    // GtdDataへのタスク挿入テスト
    // タスクを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_task() {
        let mut data = GtdData::new();
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task.clone());
        assert_eq!(data.task_count(), 1);
        assert_eq!(data.inbox.len(), 1);
        assert_eq!(data.find_task_by_id("task-1").unwrap().title, "Test Task");
    }

    // 複数タスクの挿入テスト
    // 5つのタスクを追加し、すべて正しく格納されることを確認
    #[test]
    fn test_gtd_data_insert_multiple_tasks() {
        let mut data = GtdData::new();

        for i in 1..=5 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Test Task {}", i),
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

        assert_eq!(data.task_count(), 5);
        assert_eq!(data.inbox.len(), 5);
    }

    // タスクステータスの更新テスト
    // タスクのステータスをInboxからNextActionに更新し、正しく反映されることを確認
    #[test]
    fn test_gtd_data_update_task_status() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);

        // Update status
        if let Some(task) = data.find_task_by_id_mut(&task_id) {
            task.status = TaskStatus::next_action;
        }

        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            TaskStatus::next_action
        ));
    }

    // タスクの削除テスト
    // タスクを追加後、削除して正しく削除されることを確認
    #[test]
    fn test_gtd_data_remove_task() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.task_count(), 1);
        assert_eq!(data.inbox.len(), 1);

        data.remove_task(&task_id);
        assert_eq!(data.task_count(), 0);
        assert_eq!(data.inbox.len(), 0);
    }

    // ステータス移動テスト - inbox から trash への移動
    // タスクが inbox から trash に正しく移動されることを確認
    #[test]
    fn test_gtd_data_move_status_inbox_to_trash() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.inbox.len(), 1);
        assert_eq!(data.trash.len(), 0);

        // Move task to trash
        let result = data.move_status(&task_id, TaskStatus::trash);
        assert!(result.is_some());

        // Verify task was moved
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.trash.len(), 1);
        assert_eq!(data.task_count(), 1);

        // Verify task status was updated
        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(moved_task.status, TaskStatus::trash));
    }

    // ステータス移動テスト - next_action から done への移動
    // タスクが next_action から done に正しく移動されることを確認
    #[test]
    fn test_gtd_data_move_status_next_action_to_done() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.next_action.len(), 1);
        assert_eq!(data.done.len(), 0);

        // Move task to done
        let result = data.move_status(&task_id, TaskStatus::done);
        assert!(result.is_some());

        // Verify task was moved
        assert_eq!(data.next_action.len(), 0);
        assert_eq!(data.done.len(), 1);
        assert_eq!(data.task_count(), 1);

        // Verify task status was updated
        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(moved_task.status, TaskStatus::done));
    }

    // ステータス移動テスト - 複数のステータス間の移動
    // タスクが複数のステータス間を正しく移動できることを確認
    #[test]
    fn test_gtd_data_move_status_multiple_transitions() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);

        // inbox -> next_action
        data.move_status(&task_id, TaskStatus::next_action);
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.next_action.len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            TaskStatus::next_action
        ));

        // next_action -> waiting_for
        data.move_status(&task_id, TaskStatus::waiting_for);
        assert_eq!(data.next_action.len(), 0);
        assert_eq!(data.waiting_for.len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            TaskStatus::waiting_for
        ));

        // waiting_for -> done
        data.move_status(&task_id, TaskStatus::done);
        assert_eq!(data.waiting_for.len(), 0);
        assert_eq!(data.done.len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            TaskStatus::done
        ));

        // done -> trash
        data.move_status(&task_id, TaskStatus::trash);
        assert_eq!(data.done.len(), 0);
        assert_eq!(data.trash.len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            TaskStatus::trash
        ));
    }

    // ステータス移動テスト - カレンダーへの移動
    // タスクをカレンダーステータスに移動し、正しくcalendarコンテナに格納されることを確認
    #[test]
    fn test_gtd_data_move_status_to_calendar() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        let task = Task {
            id: task_id.clone(),
            title: "Future Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: Some(date),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.inbox.len(), 1);

        // inbox -> calendar
        let result = data.move_status(&task_id, TaskStatus::calendar);
        assert!(result.is_some());
        assert_eq!(data.inbox.len(), 0);
        assert_eq!(data.calendar.len(), 1);

        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(moved_task.status, TaskStatus::calendar));
        assert_eq!(moved_task.start_date.unwrap(), date);
    }

    // ステータス移動テスト - 存在しないタスク
    // 存在しないタスクの移動がNoneを返すことを確認
    #[test]
    fn test_gtd_data_move_status_nonexistent_task() {
        let mut data = GtdData::new();
        let result = data.move_status("nonexistent-id", TaskStatus::trash);
        assert!(result.is_none());
    }

    // ステータス移動テスト - タスクのプロパティが保持される
    // ステータス移動時にタスクの他のプロパティが保持されることを確認
    #[test]
    fn test_gtd_data_move_status_preserves_properties() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Important Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("Office".to_string()),
            notes: Some("Important notes".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);

        // Move task to next_action
        data.move_status(&task_id, TaskStatus::next_action);

        // Verify all properties are preserved
        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(moved_task.title, "Important Task");
        assert_eq!(moved_task.project, Some("project-1".to_string()));
        assert_eq!(moved_task.context, Some("Office".to_string()));
        assert_eq!(moved_task.notes, Some("Important notes".to_string()));
        assert_eq!(moved_task.start_date, NaiveDate::from_ymd_opt(2024, 12, 25));
        assert_eq!(
            moved_task.created_at,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert_eq!(
            moved_task.updated_at,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert!(matches!(moved_task.status, TaskStatus::next_action));
    }

    // プロジェクトとコンテキスト付きタスクのテスト
    // プロジェクト、コンテキスト、ノートが正しく設定されることを確認
    #[test]
    fn test_task_with_project_and_context() {
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("context-1".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert_eq!(task.project.as_ref().unwrap(), "project-1");
        assert_eq!(task.context.as_ref().unwrap(), "context-1");
        assert_eq!(task.notes.as_ref().unwrap(), "Test notes");
    }

    // 開始日付付きタスクのテスト
    // タスクに開始日を設定し、正しく格納されることを確認
    #[test]
    fn test_task_with_start_date() {
        let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: Some(date),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert_eq!(task.start_date.unwrap(), date);
    }

    // カレンダーステータスのタスクテスト
    // カレンダーステータスのタスクが正しく作成され、start_dateが設定されることを確認
    #[test]
    fn test_calendar_task_with_start_date() {
        let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "Christmas Task".to_string(),
            status: TaskStatus::calendar,
            project: None,
            context: None,
            notes: None,
            start_date: Some(date),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(matches!(task.status, TaskStatus::calendar));
        assert_eq!(task.start_date.unwrap(), date);
    }

    // タスクステータスの全バリアントテスト
    // 8種類のタスクステータス（Inbox、NextAction、WaitingFor、Someday、Later、Done、Trash、Calendar）がすべて正しく動作することを確認
    #[test]
    fn test_task_status_variants() {
        let statuses = vec![
            TaskStatus::inbox,
            TaskStatus::next_action,
            TaskStatus::waiting_for,
            TaskStatus::someday,
            TaskStatus::later,
            TaskStatus::done,
            TaskStatus::trash,
            TaskStatus::calendar,
        ];

        for status in statuses {
            let task = Task {
                id: "task-1".to_string(),
                title: "Test Task".to_string(),
                status: status.clone(),
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            };

            match status {
                TaskStatus::inbox => assert!(matches!(task.status, TaskStatus::inbox)),
                TaskStatus::next_action => assert!(matches!(task.status, TaskStatus::next_action)),
                TaskStatus::waiting_for => assert!(matches!(task.status, TaskStatus::waiting_for)),
                TaskStatus::someday => assert!(matches!(task.status, TaskStatus::someday)),
                TaskStatus::later => assert!(matches!(task.status, TaskStatus::later)),
                TaskStatus::done => assert!(matches!(task.status, TaskStatus::done)),
                TaskStatus::trash => assert!(matches!(task.status, TaskStatus::trash)),
                TaskStatus::calendar => assert!(matches!(task.status, TaskStatus::calendar)),
            }
        }
    }

    // プロジェクトの作成テスト
    // プロジェクトを作成し、ID、名前、説明、ステータスが正しく設定されることを確認
    #[test]
    fn test_project_creation() {
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: Some("Test description".to_string()),
            status: ProjectStatus::active,
            context: None,
        };

        assert_eq!(project.id, "project-1");
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description.as_ref().unwrap(), "Test description");
        assert!(matches!(project.status, ProjectStatus::active));
    }

    // 説明なしプロジェクトのテスト
    // 説明を持たないプロジェクトが正しく作成されることを確認
    #[test]
    fn test_project_without_description() {
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        };

        assert!(project.description.is_none());
    }

    // プロジェクトステータスの全バリアントテスト
    // 3種類のプロジェクトステータス（Active、OnHold、Completed）がすべて正しく動作することを確認
    #[test]
    fn test_project_status_variants() {
        let statuses = vec![
            ProjectStatus::active,
            ProjectStatus::on_hold,
            ProjectStatus::completed,
        ];

        for status in statuses {
            let project = Project {
                id: "project-1".to_string(),
                name: "Test Project".to_string(),
                description: None,
                status: status.clone(),
                context: None,
            };

            match status {
                ProjectStatus::active => assert!(matches!(project.status, ProjectStatus::active)),
                ProjectStatus::on_hold => assert!(matches!(project.status, ProjectStatus::on_hold)),
                ProjectStatus::completed => {
                    assert!(matches!(project.status, ProjectStatus::completed))
                }
            }
        }
    }

    // GtdDataへのプロジェクト挿入テスト
    // プロジェクトを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_project() {
        let mut data = GtdData::new();
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        };

        data.add_project(project.clone());
        assert_eq!(data.projects.len(), 1);
        assert_eq!(
            data.find_project_by_id("project-1").unwrap().name,
            "Test Project"
        );
    }

    // プロジェクトステータスの更新テスト
    // プロジェクトのステータスをActiveからCompletedに更新し、正しく反映されることを確認
    #[test]
    fn test_gtd_data_update_project_status() {
        let mut data = GtdData::new();
        let project_id = "project-1".to_string();
        let project = Project {
            id: project_id.clone(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        };

        data.add_project(project);

        // Update status
        if let Some(project) = data.find_project_by_id_mut(&project_id) {
            project.status = ProjectStatus::completed;
        }

        assert!(matches!(
            data.find_project_by_id(&project_id).unwrap().status,
            ProjectStatus::completed
        ));
    }

    // コンテキストの作成テスト
    // コンテキストを作成し、IDと名前が正しく設定されることを確認
    #[test]
    fn test_context_creation() {
        let context = Context {
            name: "Office".to_string(),
            description: None,
        };

        assert_eq!(context.name, "Office");
        assert_eq!(context.description, None);
    }

    // コンテキストの説明付き作成テスト
    // 説明フィールドを持つコンテキストが正しく作成されることを確認
    #[test]
    fn test_context_with_description() {
        let context = Context {
            name: "Office".to_string(),
            description: Some("Work environment with desk and computer".to_string()),
        };

        assert_eq!(context.name, "Office");
        assert_eq!(
            context.description,
            Some("Work environment with desk and computer".to_string())
        );
    }

    // GtdDataへのコンテキスト挿入テスト
    // コンテキストを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_context() {
        let mut data = GtdData::new();
        let context = Context {
            name: "Office".to_string(),
            description: None,
        };

        data.add_context(context.clone());
        assert_eq!(data.contexts.len(), 1);
        assert_eq!(data.find_context_by_name("Office").unwrap().name, "Office");
    }

    // 複数コンテキストの挿入テスト
    // 4つのコンテキスト（Office、Home、Phone、Errands）を追加し、すべて正しく格納されることを確認
    #[test]
    fn test_gtd_data_insert_multiple_contexts() {
        let mut data = GtdData::new();
        let contexts = vec!["Office", "Home", "Phone", "Errands"];

        for name in contexts {
            let context = Context {
                name: name.to_string(),
                description: None,
            };
            data.add_context(context);
        }

        assert_eq!(data.contexts.len(), 4);
    }

    // タスクのシリアライゼーションテスト
    // タスクをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    #[test]
    fn test_task_serialization() {
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

        let serialized = toml::to_string(&task).unwrap();
        let deserialized: Task = toml::from_str(&serialized).unwrap();

        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.title, deserialized.title);
        assert_eq!(task.project, deserialized.project);
        assert_eq!(task.context, deserialized.context);
        assert_eq!(task.notes, deserialized.notes);
        assert_eq!(task.start_date, deserialized.start_date);
    }

    // プロジェクトのシリアライゼーションテスト
    // プロジェクトをGtdData経由でTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    // プロジェクトは現在HashMapとして保存されるため、GtdData全体でのテストが必要
    #[test]
    fn test_project_serialization() {
        let mut data = GtdData::new();
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: Some("Test description".to_string()),
            status: ProjectStatus::active,
            context: None,
        };

        data.add_project(project.clone());

        let serialized = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&serialized).unwrap();

        let deserialized_project = deserialized.projects.get("project-1").unwrap();
        assert_eq!(project.id, deserialized_project.id);
        assert_eq!(project.name, deserialized_project.name);
        assert_eq!(project.description, deserialized_project.description);
        assert_eq!(project.status, deserialized_project.status);
    }

    // コンテキストのシリアライゼーションテスト
    // コンテキストをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    // Note: name フィールドは skip_serializing されるため、TOML には含まれない
    #[test]
    fn test_context_serialization() {
        let context = Context {
            name: "Office".to_string(),
            description: None,
        };

        let serialized = toml::to_string(&context).unwrap();
        // name フィールドは serialization でスキップされるため、TOML には含まれない
        assert!(
            !serialized.contains("name"),
            "name field should not be serialized"
        );

        let deserialized: Context = toml::from_str(&serialized).unwrap();
        // standalone でデシリアライズすると name は空文字列になる（default）
        assert_eq!(deserialized.name, "");
        assert_eq!(deserialized.description, None);
    }

    // GtdData全体のシリアライゼーションテスト
    // タスク、プロジェクト、コンテキストを含むGtdDataをTOML形式にシリアライズし、デシリアライズして各要素数が一致することを確認
    #[test]
    fn test_gtd_data_serialization() {
        let mut data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        };
        data.add_project(project);

        let context = Context {
            name: "Office".to_string(),
            description: None,
        };
        data.add_context(context);

        let serialized = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&serialized).unwrap();

        assert_eq!(data.task_count(), deserialized.task_count());
        assert_eq!(data.projects.len(), deserialized.projects.len());
        assert_eq!(data.contexts.len(), deserialized.contexts.len());
    }

    // ステータスによるタスクフィルタリングテスト
    // 複数のステータスを持つタスクを追加し、特定のステータスでフィルタリングできることを確認
    #[test]
    fn test_task_filter_by_status() {
        let mut data = GtdData::new();

        let statuses = [
            TaskStatus::inbox,
            TaskStatus::next_action,
            TaskStatus::waiting_for,
            TaskStatus::someday,
            TaskStatus::later,
            TaskStatus::done,
            TaskStatus::trash,
            TaskStatus::calendar,
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

        // Filter by Inbox
        assert_eq!(data.inbox.len(), 1);

        // Filter by Done
        assert_eq!(data.done.len(), 1);

        // Verify all statuses have exactly one task
        assert_eq!(data.task_count(), 8);
    }

    // プロジェクトによるタスクフィルタリングテスト
    // 特定のプロジェクトに紐づくタスクのみをフィルタリングできることを確認
    #[test]
    fn test_task_filter_by_project() {
        let mut data = GtdData::new();

        for i in 1..=5 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: TaskStatus::inbox,
                project: if i % 2 == 0 {
                    Some("project-1".to_string())
                } else {
                    None
                },
                context: None,
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            };
            data.add_task(task);
        }

        let all_tasks = data.all_tasks();
        let project_tasks: Vec<_> = all_tasks
            .iter()
            .filter(|t| t.project.as_ref().is_some_and(|p| p == "project-1"))
            .collect();
        assert_eq!(project_tasks.len(), 2);
    }

    // コンテキストによるタスクフィルタリングテスト
    // 特定のコンテキストに紐づくタスクのみをフィルタリングできることを確認
    #[test]
    fn test_task_filter_by_context() {
        let mut data = GtdData::new();

        for i in 1..=5 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: TaskStatus::inbox,
                project: None,
                context: if i % 2 == 0 {
                    Some("context-1".to_string())
                } else {
                    None
                },
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            };
            data.add_task(task);
        }

        let all_tasks = data.all_tasks();
        let context_tasks: Vec<_> = all_tasks
            .iter()
            .filter(|t| t.context.as_ref().is_some_and(|c| c == "context-1"))
            .collect();
        assert_eq!(context_tasks.len(), 2);
    }

    // 日付パースのテスト
    // 文字列形式の日付を正しくパースし、年月日が正確に取得できることを確認
    #[test]
    fn test_date_parsing() {
        let date_str = "2024-12-25";
        let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
        assert!(parsed.is_ok());

        let date = parsed.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 25);
    }

    // 不正な日付パースのテスト
    // 無効な月と日を含む日付文字列のパースがエラーになることを確認
    #[test]
    fn test_invalid_date_parsing() {
        let date_str = "2024-13-45"; // Invalid month and day
        let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
        assert!(parsed.is_err());
    }

    // タスクのクローンテスト
    // タスクをクローンし、元のタスクと同じ内容を持つことを確認
    #[test]
    fn test_task_clone() {
        let task1 = Task {
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

        let task2 = task1.clone();
        assert_eq!(task1.id, task2.id);
        assert_eq!(task1.title, task2.title);
        assert_eq!(task1.project, task2.project);
    }

    // TOML serialization verification test
    // Verify that enum variants are serialized as snake_case in TOML format
    #[test]
    fn test_enum_snake_case_serialization() {
        let mut data = GtdData::new();

        // Add a task to next_action to verify the field name is snake_case
        data.add_task(Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        });

        let serialized = toml::to_string(&data).unwrap();
        assert!(
            serialized.contains("[[next_action]]"),
            "Expected '[[next_action]]' in TOML output"
        );

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::on_hold,
            context: None,
        };

        let serialized = toml::to_string(&project).unwrap();
        assert!(
            serialized.contains("on_hold"),
            "Expected 'on_hold' in TOML output"
        );
    }

    // Insertion order preservation test
    // Verify that tasks maintain their insertion order (Vec-based instead of HashMap)
    #[test]
    fn test_gtd_data_insertion_order() {
        let mut data = GtdData::new();

        // 特定の順序でタスクを追加
        for i in 1..=5 {
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

        // Verify that tasks maintain insertion order
        assert_eq!(data.inbox.len(), 5);
        for (i, task) in data.inbox.iter().enumerate() {
            assert_eq!(task.id, format!("task-{}", i + 1));
            assert_eq!(task.title, format!("Task {}", i + 1));
        }
    }

    // TOML serialization order preservation test
    // Verify that TOML serialization maintains insertion order
    #[test]
    fn test_toml_serialization_order() {
        let mut data = GtdData::new();

        // 特定の順序でアイテムを追加
        for i in 1..=3 {
            data.add_task(Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: TaskStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            });
        }

        for i in 1..=2 {
            data.add_project(Project {
                id: format!("project-{}", i),
                name: format!("Project {}", i),
                description: None,
                status: ProjectStatus::active,
                context: None,
            });
        }

        let toml_str = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&toml_str).unwrap();

        // Verify deserialized data maintains insertion order for tasks
        assert_eq!(deserialized.inbox.len(), 3);
        for (i, task) in deserialized.inbox.iter().enumerate() {
            assert_eq!(task.id, format!("task-{}", i + 1));
        }

        // Verify all projects are present (HashMap doesn't guarantee order)
        assert_eq!(deserialized.projects.len(), 2);
        assert!(deserialized.projects.contains_key("project-1"));
        assert!(deserialized.projects.contains_key("project-2"));
    }

    // 完全なTOML出力テスト（全フィールド設定）
    // 全フィールドを設定した状態でTOML出力を検証し、意図したテキスト形式で出力されることを確認する
    // このテストは出力形式の変更を検出するため、期待されるTOMLテキストとの完全一致を検証する
    #[test]
    fn test_complete_toml_output() {
        let mut data = GtdData::new();

        // 全フィールドを設定したタスクを追加
        data.add_task(Task {
            id: "task-001".to_string(),
            title: "Complete project documentation".to_string(),
            status: TaskStatus::next_action,
            project: Some("project-001".to_string()),
            context: Some("Office".to_string()),
            notes: Some("Review all sections and update examples".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 3, 15),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        });

        // 最小限のフィールドを設定したタスクを追加（比較用）
        data.add_task(Task {
            id: "task-002".to_string(),
            title: "Quick task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        });

        // 全フィールドを設定したプロジェクトを追加
        data.add_project(Project {
            id: "project-001".to_string(),
            name: "Documentation Project".to_string(),
            description: Some("Comprehensive project documentation update".to_string()),
            status: ProjectStatus::active,
            context: None,
        });

        // 説明付きコンテキストを追加
        data.add_context(Context {
            name: "Office".to_string(),
            description: Some("Work environment with desk and computer".to_string()),
        });

        // TOML出力を生成
        let toml_output = toml::to_string_pretty(&data).unwrap();

        // TOML構造と可読性を確認
        println!(
            "\n=== TOML Output ===\n{}\n===================\n",
            toml_output
        );

        // 期待されるTOML構造（テキスト完全一致）
        let expected_toml = r#"format_version = 2

[[inbox]]
id = "task-002"
title = "Quick task"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[next_action]]
id = "task-001"
title = "Complete project documentation"
project = "project-001"
context = "Office"
notes = "Review all sections and update examples"
start_date = "2024-03-15"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[projects.project-001]
name = "Documentation Project"
description = "Comprehensive project documentation update"
status = "active"

[contexts.Office]
description = "Work environment with desk and computer"
"#;

        // TOML出力が期待される形式と完全一致することを確認
        assert_eq!(
            toml_output, expected_toml,
            "TOML output should match expected format"
        );

        // デシリアライゼーションが正しく動作することを確認
        let deserialized: GtdData = toml::from_str(&toml_output).unwrap();

        // 全タスクフィールドを検証
        assert_eq!(deserialized.inbox.len(), 1);
        assert_eq!(deserialized.next_action.len(), 1);

        let task_inbox = &deserialized.inbox[0];
        assert_eq!(task_inbox.id, "task-002");
        assert_eq!(task_inbox.title, "Quick task");
        assert!(matches!(task_inbox.status, TaskStatus::inbox));

        let task1 = &deserialized.next_action[0];
        assert_eq!(task1.id, "task-001");
        assert_eq!(task1.title, "Complete project documentation");
        assert!(matches!(task1.status, TaskStatus::next_action));
        assert_eq!(task1.project, Some("project-001".to_string()));
        assert_eq!(task1.context, Some("Office".to_string()));
        assert_eq!(
            task1.notes,
            Some("Review all sections and update examples".to_string())
        );
        assert_eq!(task1.start_date, NaiveDate::from_ymd_opt(2024, 3, 15));

        // プロジェクトフィールドを検証
        assert_eq!(deserialized.projects.len(), 1);
        let project1 = deserialized.projects.get("project-001").unwrap();
        assert_eq!(project1.id, "project-001");
        assert_eq!(project1.name, "Documentation Project");
        assert_eq!(
            project1.description,
            Some("Comprehensive project documentation update".to_string())
        );
        assert!(matches!(project1.status, ProjectStatus::active));

        // コンテキストフィールドを検証
        assert_eq!(deserialized.contexts.len(), 1);

        let context_office = deserialized.contexts.get("Office").unwrap();
        assert_eq!(context_office.name, "Office");
        assert_eq!(
            context_office.description,
            Some("Work environment with desk and computer".to_string())
        );
    }

    // 後方互換性テスト: 旧形式（nameフィールド付き）のTOMLも正しく読み込めることを確認
    #[test]
    fn test_backward_compatibility_with_name_field() {
        // 旧形式のTOML（nameフィールドが含まれている）
        let old_format_toml = r#"
[[tasks]]
id = "task-001"
title = "Test task"
status = "inbox"

[contexts.Office]
name = "Office"
description = "Work environment with desk and computer"

[contexts.Home]
name = "Home"
"#;

        // 旧形式のTOMLを読み込めることを確認
        let deserialized: GtdData = toml::from_str(old_format_toml).unwrap();

        assert_eq!(deserialized.contexts.len(), 2);

        // Officeコンテキストを検証
        let office = deserialized.contexts.get("Office").unwrap();
        assert_eq!(office.name, "Office");
        assert_eq!(
            office.description,
            Some("Work environment with desk and computer".to_string())
        );

        // Homeコンテキストを検証
        let home = deserialized.contexts.get("Home").unwrap();
        assert_eq!(home.name, "Home");
        assert_eq!(home.description, None);

        // 再シリアライズすると新形式（nameフィールドなし）になることを確認
        let reserialized = toml::to_string_pretty(&deserialized).unwrap();
        assert!(
            !reserialized.contains("name = \"Office\""),
            "Reserialized TOML should not contain redundant name field"
        );
        assert!(
            !reserialized.contains("name = \"Home\""),
            "Reserialized TOML should not contain redundant name field"
        );
    }

    // 参照整合性検証テスト - プロジェクト参照が有効
    // タスクのプロジェクト参照が存在するプロジェクトを指している場合、検証が成功することを確認
    #[test]
    fn test_validate_task_project_valid() {
        let mut data = GtdData::new();

        data.add_project(Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(data.validate_task_project(&task));
    }

    // 参照整合性検証テスト - プロジェクト参照が無効
    // タスクのプロジェクト参照が存在しないプロジェクトを指している場合、検証が失敗することを確認
    #[test]
    fn test_validate_task_project_invalid() {
        let data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("non-existent-project".to_string()),
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(!data.validate_task_project(&task));
    }

    // 参照整合性検証テスト - プロジェクト参照がNone
    // タスクのプロジェクト参照がNoneの場合、検証が成功することを確認
    #[test]
    fn test_validate_task_project_none() {
        let data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(data.validate_task_project(&task));
    }

    // 参照整合性検証テスト - コンテキスト参照が有効
    // タスクのコンテキスト参照が存在するコンテキストを指している場合、検証が成功することを確認
    #[test]
    fn test_validate_task_context_valid() {
        let mut data = GtdData::new();

        data.add_context(Context {
            name: "Office".to_string(),
            description: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: Some("Office".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(data.validate_task_context(&task));
    }

    // 参照整合性検証テスト - コンテキスト参照が無効
    // タスクのコンテキスト参照が存在しないコンテキストを指している場合、検証が失敗することを確認
    #[test]
    fn test_validate_task_context_invalid() {
        let data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: Some("NonExistent".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(!data.validate_task_context(&task));
    }

    // 参照整合性検証テスト - コンテキスト参照がNone
    // タスクのコンテキスト参照がNoneの場合、検証が成功することを確認
    #[test]
    fn test_validate_task_context_none() {
        let data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(data.validate_task_context(&task));
    }

    // 参照整合性検証テスト - 全ての参照が有効
    // タスクのプロジェクトとコンテキストの両方の参照が有効な場合、検証が成功することを確認
    #[test]
    fn test_validate_task_references_all_valid() {
        let mut data = GtdData::new();

        data.add_project(Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        });

        data.add_context(Context {
            name: "Office".to_string(),
            description: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("Office".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(data.validate_task_references(&task));
    }

    // 参照整合性検証テスト - プロジェクト参照のみ無効
    // プロジェクト参照が無効でコンテキスト参照が有効な場合、検証が失敗することを確認
    #[test]
    fn test_validate_task_references_invalid_project() {
        let mut data = GtdData::new();

        data.add_context(Context {
            name: "Office".to_string(),
            description: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("non-existent-project".to_string()),
            context: Some("Office".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(!data.validate_task_references(&task));
    }

    // 参照整合性検証テスト - コンテキスト参照のみ無効
    // コンテキスト参照が無効でプロジェクト参照が有効な場合、検証が失敗することを確認
    #[test]
    fn test_validate_task_references_invalid_context() {
        let mut data = GtdData::new();

        data.add_project(Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("NonExistent".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(!data.validate_task_references(&task));
    }

    // 参照整合性検証テスト - 両方の参照が無効
    // プロジェクトとコンテキストの両方の参照が無効な場合、検証が失敗することを確認
    #[test]
    fn test_validate_task_references_both_invalid() {
        let data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("non-existent-project".to_string()),
            context: Some("NonExistent".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(!data.validate_task_references(&task));
    }

    // 作成日と更新日のテスト
    // タスクが作成されたとき、created_atとupdated_atが同じ日付に設定されることを確認
    #[test]
    fn test_task_created_at_and_updated_at() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: date,
            updated_at: date,
        };

        assert_eq!(task.created_at, date);
        assert_eq!(task.updated_at, date);
        assert_eq!(task.created_at, task.updated_at);
    }

    // 更新日の変更テスト
    // タスクが更新されたとき、updated_atが変更されることを確認
    #[test]
    fn test_task_updated_at_changes() {
        let created_date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let updated_date = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();

        let mut task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: created_date,
            updated_at: created_date,
        };

        // タスクを更新
        task.status = TaskStatus::next_action;
        task.updated_at = updated_date;

        assert_eq!(task.created_at, created_date);
        assert_eq!(task.updated_at, updated_date);
        assert_ne!(task.created_at, task.updated_at);
    }

    // 作成日は変更されないことを確認するテスト
    // タスクのステータスが変更されても、created_atは変更されないことを確認
    #[test]
    fn test_task_created_at_immutable() {
        let mut data = GtdData::new();
        let created_date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let task_id = "task-1".to_string();

        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: created_date,
            updated_at: created_date,
        };

        data.add_task(task);

        // タスクのステータスを更新
        if let Some(task) = data.find_task_by_id_mut(&task_id) {
            task.status = TaskStatus::next_action;
            task.updated_at = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
        }

        // created_atは変更されていないことを確認
        let task = data.find_task_by_id(&task_id).unwrap();
        assert_eq!(task.created_at, created_date);
        assert_ne!(task.updated_at, created_date);
    }

    // TOML シリアライゼーションに作成日と更新日が含まれることを確認
    #[test]
    fn test_task_dates_serialization() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: date,
            updated_at: date,
        };

        let serialized = toml::to_string(&task).unwrap();
        assert!(serialized.contains("created_at = \"2024-03-15\""));
        assert!(serialized.contains("updated_at = \"2024-03-15\""));

        let deserialized: Task = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.created_at, date);
        assert_eq!(deserialized.updated_at, date);
    }

    // ID生成テスト - タスクIDが連番で生成されることを確認
    #[test]
    fn test_generate_task_id() {
        let mut data = GtdData::new();

        let id1 = data.generate_task_id();
        let id2 = data.generate_task_id();
        let id3 = data.generate_task_id();

        assert_eq!(id1, "#1");
        assert_eq!(id2, "#2");
        assert_eq!(id3, "#3");
        assert_eq!(data.task_counter, 3);
    }

    // ID生成テスト - カウンターの永続化を確認
    #[test]
    fn test_counter_serialization() {
        let mut data = GtdData::new();

        // Generate some IDs
        data.generate_task_id();
        data.generate_task_id();

        // Serialize
        let serialized = toml::to_string_pretty(&data).unwrap();

        // Check that counter is in the serialized output
        assert!(
            serialized.contains("task_counter = 2"),
            "task_counter should be serialized"
        );

        // Deserialize
        let deserialized: GtdData = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.task_counter, 2);

        // Next ID should continue from where we left off
        let mut data = deserialized;
        assert_eq!(data.generate_task_id(), "#3");
    }

    // ID生成テスト - カウンターが0の場合はTOMLに含まれないことを確認
    #[test]
    fn test_counter_skip_serializing_if_zero() {
        let data = GtdData::new();

        let serialized = toml::to_string_pretty(&data).unwrap();

        // Counters should not appear in serialized output when they are 0
        assert!(
            !serialized.contains("task_counter"),
            "task_counter should not be serialized when 0"
        );
        assert!(
            !serialized.contains("project_counter"),
            "project_counter should not be serialized when 0"
        );
    }

    // プロジェクトのコンテキスト参照検証テスト - 有効な参照
    // プロジェクトのコンテキスト参照が存在するコンテキストを指している場合、検証が成功することを確認
    #[test]
    fn test_validate_project_context_valid() {
        let mut data = GtdData::new();

        data.add_context(Context {
            name: "Office".to_string(),
            description: None,
        });

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: Some("Office".to_string()),
        };

        assert!(data.validate_project_context(&project));
    }

    // プロジェクトのコンテキスト参照検証テスト - 無効な参照
    // プロジェクトのコンテキスト参照が存在しないコンテキストを指している場合、検証が失敗することを確認
    #[test]
    fn test_validate_project_context_invalid() {
        let data = GtdData::new();

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: Some("NonExistent".to_string()),
        };

        assert!(!data.validate_project_context(&project));
    }

    // プロジェクトのコンテキスト参照検証テスト - コンテキスト参照がNone
    // プロジェクトのコンテキスト参照がNoneの場合、検証が成功することを確認
    #[test]
    fn test_validate_project_context_none() {
        let data = GtdData::new();

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: None,
        };

        assert!(data.validate_project_context(&project));
    }

    // プロジェクトとタスクの両方にコンテキストを設定するテスト
    // プロジェクトとタスクの両方が同じコンテキストを参照できることを確認
    #[test]
    fn test_project_and_task_with_same_context() {
        let mut data = GtdData::new();

        data.add_context(Context {
            name: "Office".to_string(),
            description: Some("Work environment".to_string()),
        });

        let project = Project {
            id: "project-1".to_string(),
            name: "Office Project".to_string(),
            description: None,
            status: ProjectStatus::active,
            context: Some("Office".to_string()),
        };
        data.add_project(project.clone());

        let task = Task {
            id: "task-1".to_string(),
            title: "Office Task".to_string(),
            status: TaskStatus::next_action,
            project: Some("project-1".to_string()),
            context: Some("Office".to_string()),
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(data.validate_project_context(&project));
        assert!(data.validate_task_context(&task));
        assert_eq!(project.context, task.context);
    }

    // 後方互換性テスト - コンテキストフィールドなしのプロジェクト
    // 旧バージョンのTOMLファイル（コンテキストフィールドなし）を正しく読み込めることを確認
    #[test]
    fn test_backward_compatibility_project_without_context() {
        // TOML from old version without context field
        let toml_str = r#"
[[projects]]
id = "project-1"
name = "Old Project"
description = "Project without context field"
status = "active"
"#;

        let data: GtdData = toml::from_str(toml_str).unwrap();
        assert_eq!(data.projects.len(), 1);

        let project = data.projects.get("project-1").unwrap();
        assert_eq!(project.id, "project-1");
        assert_eq!(project.name, "Old Project");
        assert_eq!(project.context, None);
    }

    // フォーマットバージョン1からバージョン2への自動マイグレーションテスト
    // 旧形式（Vec<Project>）のTOMLを読み込み、新形式（HashMap）に自動変換されることを確認
    #[test]
    fn test_format_migration_v1_to_v2() {
        // Format version 1: projects as array ([[projects]])
        let old_format_toml = r#"
[[projects]]
id = "project-1"
name = "First Project"
description = "Original format"
status = "active"

[[projects]]
id = "project-2"
name = "Second Project"
status = "on_hold"

[[inbox]]
id = "task-1"
title = "Test task"
project = "project-1"
created_at = "2024-01-01"
updated_at = "2024-01-01"
"#;

        // Load old format
        let data: GtdData = toml::from_str(old_format_toml).unwrap();

        // Verify it's automatically migrated to version 2
        assert_eq!(data.format_version, 2);
        assert_eq!(data.projects.len(), 2);

        // Verify projects are in HashMap
        let project1 = data.projects.get("project-1").unwrap();
        assert_eq!(project1.id, "project-1");
        assert_eq!(project1.name, "First Project");
        assert_eq!(project1.description, Some("Original format".to_string()));

        let project2 = data.projects.get("project-2").unwrap();
        assert_eq!(project2.id, "project-2");
        assert_eq!(project2.name, "Second Project");

        // Verify task references still work
        assert_eq!(data.inbox.len(), 1);
        assert_eq!(data.inbox[0].project, Some("project-1".to_string()));

        // Save to new format
        let new_format_toml = toml::to_string_pretty(&data).unwrap();

        // Verify new format has HashMap syntax
        assert!(new_format_toml.contains("format_version = 2"));
        assert!(new_format_toml.contains("[projects.project-1]"));
        assert!(new_format_toml.contains("[projects.project-2]"));
        assert!(!new_format_toml.contains("[[projects]]"));

        // Verify round-trip works
        let reloaded: GtdData = toml::from_str(&new_format_toml).unwrap();
        assert_eq!(reloaded.format_version, 2);
        assert_eq!(reloaded.projects.len(), 2);
        assert!(reloaded.projects.contains_key("project-1"));
        assert!(reloaded.projects.contains_key("project-2"));
    }

    // タスクステータスの順序がTOMLシリアライズに反映されることを確認
    // TaskStatus enumの順序とGtdDataフィールドの順序が一致し、TOML出力もその順序になることを検証
    #[test]
    fn test_task_status_order_in_toml_serialization() {
        let mut data = GtdData::new();

        // Add one task for each status in enum order
        let statuses = [
            TaskStatus::inbox,
            TaskStatus::next_action,
            TaskStatus::waiting_for,
            TaskStatus::later,
            TaskStatus::calendar,
            TaskStatus::someday,
            TaskStatus::done,
            TaskStatus::trash,
        ];

        for (i, status) in statuses.iter().enumerate() {
            data.add_task(Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: status.clone(),
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            });
        }

        let toml_str = toml::to_string(&data).unwrap();

        // Extract section headers in order they appear in TOML
        let sections: Vec<&str> = toml_str
            .lines()
            .filter(|line| line.starts_with("[["))
            .collect();

        // Verify the order matches TaskStatus enum order
        let expected_sections = [
            "[[inbox]]",
            "[[next_action]]",
            "[[waiting_for]]",
            "[[later]]",
            "[[calendar]]",
            "[[someday]]",
            "[[done]]",
            "[[trash]]",
        ];

        assert_eq!(
            sections.len(),
            expected_sections.len(),
            "Expected {} sections but found {}",
            expected_sections.len(),
            sections.len()
        );

        for (i, expected) in expected_sections.iter().enumerate() {
            assert_eq!(
                sections[i], *expected,
                "Section at position {} should be {}, but got {}",
                i, expected, sections[i]
            );
        }
    }
}
