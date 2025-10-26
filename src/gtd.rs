use chrono::{Local, NaiveDate};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::str::FromStr;

// Import legacy types from migration module (for backward compatibility only)
use crate::migration::{Context, Project, Task};

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
pub enum NotaStatus {
    /// Unprocessed items
    inbox,
    /// Tasks to do now
    next_action,
    /// Tasks waiting for someone else or an external event
    waiting_for,
    /// Tasks to do later (not immediately actionable)
    later,
    /// Tasks scheduled for a specific date
    calendar,
    /// Tasks that might be done someday but not now
    someday,
    /// Completed tasks
    done,
    /// Context nota (represents a location, tool, or situation)
    context,
    /// Project nota (represents a multi-step outcome)
    project,
    /// Deleted or discarded items
    trash,
}

impl FromStr for NotaStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(NotaStatus::inbox),
            "next_action" => Ok(NotaStatus::next_action),
            "waiting_for" => Ok(NotaStatus::waiting_for),
            "someday" => Ok(NotaStatus::someday),
            "later" => Ok(NotaStatus::later),
            "calendar" => Ok(NotaStatus::calendar),
            "done" => Ok(NotaStatus::done),
            "trash" => Ok(NotaStatus::trash),
            "context" => Ok(NotaStatus::context),
            "project" => Ok(NotaStatus::project),
            _ => Err(format!(
                "Invalid status '{}'. Valid options are: inbox, next_action, waiting_for, someday, later, calendar, done, trash, context, project",
                s
            )),
        }
    }
}

/// A unified nota (note) in the GTD system
///
/// Nota unifies Task, Project, and Context into a single structure.
/// The `status` field determines what type of nota it is:
/// - status = "context": represents a Context
/// - status = "project": represents a Project
/// - other statuses (inbox, next_action, etc.): represents a Task
///
/// This design is inspired by TiddlyWiki's tiddler concept.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nota {
    /// Unique identifier (e.g., "meeting-prep", "website-redesign", "Office")
    pub id: String,
    /// Title describing the nota
    pub title: String,
    /// Current status (inbox, next_action, waiting_for, later, calendar, someday, done, trash, context, project)
    pub status: NotaStatus,
    /// Optional parent project ID
    pub project: Option<String>,
    /// Optional context where this nota applies
    pub context: Option<String>,
    /// Optional additional notes in Markdown format
    pub notes: Option<String>,
    /// Optional start date (format: YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,
    /// Date when the nota was created
    pub created_at: NaiveDate,
    /// Date when the nota was last updated
    pub updated_at: NaiveDate,
}

#[allow(dead_code)]
impl Nota {
    /// Create a Nota from a Task
    pub fn from_task(task: Task) -> Self {
        Self {
            id: task.id,
            title: task.title,
            status: task.status,
            project: task.project,
            context: task.context,
            notes: task.notes,
            start_date: task.start_date,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }

    /// Create a Nota from a Project
    pub fn from_project(project: Project) -> Self {
        Self {
            id: project.id,
            title: project.title,
            status: NotaStatus::project,
            project: project.project,
            context: project.context,
            notes: project.notes,
            start_date: project.start_date,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }

    /// Create a Nota from a Context
    pub fn from_context(context: Context) -> Self {
        Self {
            id: context.name.clone(),
            title: context.title.unwrap_or(context.name),
            status: NotaStatus::context,
            project: context.project,
            context: context.context,
            notes: context.notes,
            start_date: context.start_date,
            created_at: context.created_at.unwrap_or_else(local_date_today),
            updated_at: context.updated_at.unwrap_or_else(local_date_today),
        }
    }

    /// Convert this Nota to a Task (if status is task-related)
    pub fn to_task(&self) -> Option<Task> {
        match self.status {
            NotaStatus::context | NotaStatus::project => None,
            _ => Some(Task {
                id: self.id.clone(),
                title: self.title.clone(),
                status: self.status.clone(),
                project: self.project.clone(),
                context: self.context.clone(),
                notes: self.notes.clone(),
                start_date: self.start_date,
                created_at: self.created_at,
                updated_at: self.updated_at,
            }),
        }
    }

    /// Convert this Nota to a Project (if status is project)
    pub fn to_project(&self) -> Option<Project> {
        if self.status == NotaStatus::project {
            Some(Project {
                id: self.id.clone(),
                title: self.title.clone(),
                notes: self.notes.clone(),
                project: self.project.clone(),
                context: self.context.clone(),
                start_date: self.start_date,
                created_at: self.created_at,
                updated_at: self.updated_at,
            })
        } else {
            None
        }
    }

    /// Convert this Nota to a Context (if status is context)
    pub fn to_context(&self) -> Option<Context> {
        if self.status == NotaStatus::context {
            Some(Context {
                name: self.id.clone(),
                title: Some(self.title.clone()),
                notes: self.notes.clone(),
                status: NotaStatus::context,
                project: self.project.clone(),
                context: self.context.clone(),
                start_date: self.start_date,
                created_at: Some(self.created_at),
                updated_at: Some(self.updated_at),
            })
        } else {
            None
        }
    }

    /// Check if this nota is a task
    pub fn is_task(&self) -> bool {
        !matches!(self.status, NotaStatus::context | NotaStatus::project)
    }

    /// Check if this nota is a project
    pub fn is_project(&self) -> bool {
        self.status == NotaStatus::project
    }

    /// Check if this nota is a context
    pub fn is_context(&self) -> bool {
        self.status == NotaStatus::context
    }
}

/// The main GTD data structure
///
/// This struct holds all GTD items (tasks, projects, contexts) as unified Nota objects.
/// The Nota structure is the fundamental data type that can represent any GTD item.
///
/// The data is designed to be serialized to/from TOML format for persistent storage.
///
/// ## Format Versions
///
/// - Version 1 (legacy): Projects stored as `Vec<Project>` (TOML: `[[projects]]`)
/// - Version 2 (legacy): Projects stored as `HashMap<String, Project>` (TOML: `[projects.id]`), separate arrays for each status
/// - Version 3 (legacy): Projects and contexts stored as Vec (TOML: `[[project]]`, `[[context]]`)
/// - Version 4 (legacy): All items stored as unified Nota array (TOML: `[[notas]]`)
/// - Version 5 (current): Items stored in separate arrays by status (TOML: `[[inbox]]`, `[[next_action]]`, etc.)
///
/// The deserializer automatically migrates from older versions to the current internal format.
#[derive(Debug)]
pub struct GtdData {
    /// Format version for the TOML file (current: 5)
    pub format_version: u32,
    /// All GTD items stored as Nota objects
    pub(crate) notas: Vec<Nota>,
    /// Internal map of all nota IDs for duplicate checking (not serialized)
    pub(crate) nota_map: HashMap<String, NotaStatus>,
    /// Counter for generating unique task IDs
    pub task_counter: u32,
    /// Counter for generating unique project IDs
    pub project_counter: u32,
}

impl Default for GtdData {
    fn default() -> Self {
        Self {
            format_version: 5,
            notas: Vec::new(),
            nota_map: HashMap::new(),
            task_counter: 0,
            project_counter: 0,
        }
    }
}

/// Default format version for new files
#[allow(dead_code)] // Used by serde
fn default_format_version() -> u32 {
    5
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

        let helper = GtdDataMigrationHelper::deserialize(deserializer)?;

        // Start with notas from Version 4/5 format if available
        let mut notas = helper.notas;

        // If notas is empty, we need to migrate from older formats or Version 5 status-based arrays
        if notas.is_empty() {
            // Initialize collections for migration
            let mut inbox = helper.inbox;
            let mut next_action = helper.next_action;
            let mut waiting_for = helper.waiting_for;
            let mut later = helper.later;
            let mut calendar = helper.calendar;
            let mut someday = helper.someday;
            let mut done = helper.done;
            let mut trash = helper.trash;
            let mut projects = migrate_projects_to_latest(helper.projects);
            let mut contexts = helper.contexts;

            // If this is Version 3 format with Vec arrays for projects/contexts, convert to HashMap
            if !helper.project.is_empty() {
                for project in helper.project {
                    projects.insert(project.id.clone(), project);
                }
            }
            if !helper.context.is_empty() {
                for context in helper.context {
                    contexts.insert(context.name.clone(), context);
                }
            }

            // Populate the name/id fields
            populate_context_names(&mut contexts);
            populate_project_ids(&mut projects);

            // Normalize line endings in all string fields
            normalize_task_line_endings(&mut inbox);
            normalize_task_line_endings(&mut next_action);
            normalize_task_line_endings(&mut waiting_for);
            normalize_task_line_endings(&mut later);
            normalize_task_line_endings(&mut calendar);
            normalize_task_line_endings(&mut someday);
            normalize_task_line_endings(&mut done);
            normalize_task_line_endings(&mut trash);
            normalize_project_line_endings(&mut projects);
            normalize_context_line_endings(&mut contexts);

            // Set the status field for each task based on which collection it's in
            for task in &mut inbox {
                task.status = NotaStatus::inbox;
            }
            for task in &mut next_action {
                task.status = NotaStatus::next_action;
            }
            for task in &mut waiting_for {
                task.status = NotaStatus::waiting_for;
            }
            for task in &mut later {
                task.status = NotaStatus::later;
            }
            for task in &mut calendar {
                task.status = NotaStatus::calendar;
            }
            for task in &mut someday {
                task.status = NotaStatus::someday;
            }
            for task in &mut done {
                task.status = NotaStatus::done;
            }
            for task in &mut trash {
                task.status = NotaStatus::trash;
            }

            // Convert all old structures to Nota
            for task in inbox {
                notas.push(Nota::from_task(task));
            }
            for task in next_action {
                notas.push(Nota::from_task(task));
            }
            for task in waiting_for {
                notas.push(Nota::from_task(task));
            }
            for task in later {
                notas.push(Nota::from_task(task));
            }
            for task in calendar {
                notas.push(Nota::from_task(task));
            }
            for task in someday {
                notas.push(Nota::from_task(task));
            }
            for task in done {
                notas.push(Nota::from_task(task));
            }
            for task in trash {
                notas.push(Nota::from_task(task));
            }
            for project in projects.into_values() {
                notas.push(Nota::from_project(project));
            }
            for context in contexts.into_values() {
                notas.push(Nota::from_context(context));
            }
        }

        // Build nota_map from all notas for duplicate checking
        let mut nota_map = HashMap::new();
        for nota in &notas {
            nota_map.insert(nota.id.clone(), nota.status.clone());
        }

        Ok(GtdData {
            format_version: 5, // Use version 5 for in-memory representation
            notas,
            nota_map,
            task_counter: helper.task_counter,
            project_counter: helper.project_counter,
        })
    }
}

impl Serialize for GtdData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("GtdData", 13)?;
        state.serialize_field("format_version", &self.format_version)?;

        // Separate notas by status (Version 5 format)
        let inbox: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::inbox).collect();
        let next_action: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::next_action).collect();
        let waiting_for: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::waiting_for).collect();
        let later: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::later).collect();
        let calendar: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::calendar).collect();
        let someday: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::someday).collect();
        let done: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::done).collect();
        let context: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::context).collect();
        let project: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::project).collect();
        let trash: Vec<&Nota> = self.notas.iter().filter(|n| n.status == NotaStatus::trash).collect();

        // Serialize each status array (only if non-empty)
        if !inbox.is_empty() {
            state.serialize_field("inbox", &inbox)?;
        }
        if !next_action.is_empty() {
            state.serialize_field("next_action", &next_action)?;
        }
        if !waiting_for.is_empty() {
            state.serialize_field("waiting_for", &waiting_for)?;
        }
        if !later.is_empty() {
            state.serialize_field("later", &later)?;
        }
        if !calendar.is_empty() {
            state.serialize_field("calendar", &calendar)?;
        }
        if !someday.is_empty() {
            state.serialize_field("someday", &someday)?;
        }
        if !done.is_empty() {
            state.serialize_field("done", &done)?;
        }
        if !context.is_empty() {
            state.serialize_field("context", &context)?;
        }
        if !project.is_empty() {
            state.serialize_field("project", &project)?;
        }
        if !trash.is_empty() {
            state.serialize_field("trash", &trash)?;
        }

        if self.task_counter != 0 {
            state.serialize_field("task_counter", &self.task_counter)?;
        }
        if self.project_counter != 0 {
            state.serialize_field("project_counter", &self.project_counter)?;
        }

        state.end()
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

    /// Count total number of notas (all types: tasks, projects, contexts)
    #[allow(dead_code)]
    pub fn nota_count(&self) -> usize {
        self.notas.len()
    }

    /// Count total number of task notas across all task statuses
    #[allow(dead_code)]
    pub fn task_count(&self) -> usize {
        self.notas.iter().filter(|n| n.is_task()).count()
    }

    /// Find a nota by its ID
    ///
    /// # Arguments
    /// * `id` - The nota ID to search for
    ///
    /// # Returns
    /// An optional reference to the nota if found
    #[allow(dead_code)]
    fn find_nota_by_id(&self, id: &str) -> Option<&Nota> {
        self.notas.iter().find(|n| n.id == id)
    }

    /// Find a nota by its ID and return a mutable reference
    ///
    /// # Arguments
    /// * `id` - The nota ID to search for
    ///
    /// # Returns
    /// An optional mutable reference to the nota if found
    fn find_nota_by_id_mut(&mut self, id: &str) -> Option<&mut Nota> {
        self.notas.iter_mut().find(|n| n.id == id)
    }

    /// Find a task by its ID (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The task ID to search for (e.g., "#1")
    ///
    /// # Returns
    /// An optional Nota reference if found and it's a task
    #[allow(dead_code)]
    pub fn find_task_by_id(&self, id: &str) -> Option<&Nota> {
        self.notas.iter().find(|n| n.id == id && n.is_task())
    }

    /// Find a task by its ID and return a mutable reference (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The task ID to search for (e.g., "#1")
    ///
    /// # Returns
    /// An optional mutable Nota reference if found and it's a task
    pub fn find_task_by_id_mut(&mut self, id: &str) -> Option<&mut Nota> {
        self.notas.iter_mut().find(|n| n.id == id && n.is_task())
    }

    /// Add a nota to the collection
    ///
    /// # Arguments
    /// * `nota` - The nota to add
    pub fn add_nota(&mut self, nota: Nota) {
        let id = nota.id.clone();
        let status = nota.status.clone();

        // Add to nota_map for duplicate checking
        self.nota_map.insert(id, status);

        // Add to notas vector
        self.notas.push(nota);
    }

    /// Remove a nota from the collection and return it
    ///
    /// # Arguments
    /// * `id` - The nota ID to remove
    ///
    /// # Returns
    /// The removed nota if found
    #[allow(dead_code)]
    pub fn remove_nota(&mut self, id: &str) -> Option<Nota> {
        // Find and remove nota
        if let Some(pos) = self.notas.iter().position(|n| n.id == id) {
            let nota = self.notas.remove(pos);
            self.nota_map.remove(id);
            Some(nota)
        } else {
            None
        }
    }

    /// Move a nota to a different status
    ///
    /// This method updates the status of the nota.
    ///
    /// # Arguments
    /// * `id` - The nota ID to move
    /// * `new_status` - The target status
    ///
    /// # Returns
    /// `Some(())` if the nota was found and moved, `None` otherwise
    pub fn move_status(&mut self, id: &str, new_status: NotaStatus) -> Option<()> {
        if let Some(nota) = self.find_nota_by_id_mut(id) {
            nota.status = new_status.clone();
            nota.updated_at = local_date_today();
            self.nota_map.insert(id.to_string(), new_status);
            Some(())
        } else {
            None
        }
    }

    /// Find a project by its ID (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The project ID to search for (e.g., "project-1")
    ///
    /// # Returns
    /// An optional reference to the nota if found and it's a project
    #[allow(dead_code)]
    pub fn find_project_by_id(&self, id: &str) -> Option<&Nota> {
        self.notas
            .iter()
            .find(|n| n.id == id && n.status == NotaStatus::project)
    }

    /// Find a project by its ID and return a mutable reference (for compatibility)
    ///
    /// # Arguments
    /// * `id` - The project ID to search for (e.g., "project-1")
    ///
    /// # Returns
    /// An optional mutable reference to the nota if found and it's a project
    #[allow(dead_code)]
    pub fn find_project_by_id_mut(&mut self, id: &str) -> Option<&mut Nota> {
        self.notas
            .iter_mut()
            .find(|n| n.id == id && n.status == NotaStatus::project)
    }

    /// Find a context by its name (for compatibility)
    ///
    /// # Arguments
    /// * `name` - The context name to search for (e.g., "Office")
    ///
    /// # Returns
    /// An optional reference to the nota if found and it's a context
    #[allow(dead_code)]
    pub fn find_context_by_name(&self, name: &str) -> Option<&Nota> {
        self.notas
            .iter()
            .find(|n| n.id == name && n.status == NotaStatus::context)
    }

    /// Validate that a nota's project reference exists (if specified)
    /// Returns true if the nota has no project reference or if the reference is valid
    pub fn validate_nota_project(&self, nota: &Nota) -> bool {
        match &nota.project {
            None => true,
            Some(project_id) => self.find_project_by_id(project_id).is_some(),
        }
    }

    /// Validate that a nota's context reference exists (if specified)
    /// Returns true if the nota has no context reference or if the reference is valid
    pub fn validate_nota_context(&self, nota: &Nota) -> bool {
        match &nota.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Validate that a nota's references (project and context) exist
    /// Returns true if all references are valid or not specified
    pub fn validate_nota_references(&self, nota: &Nota) -> bool {
        self.validate_nota_project(nota) && self.validate_nota_context(nota)
    }

    /// Update project ID references in all notas
    ///
    /// When a project ID changes, this method updates all nota references
    /// from the old ID to the new ID.
    ///
    /// # Arguments
    /// * `old_id` - The old project ID
    /// * `new_id` - The new project ID
    pub fn update_project_id_in_notas(&mut self, old_id: &str, new_id: &str) {
        for nota in self.notas.iter_mut() {
            if let Some(ref project_id) = nota.project
                && project_id == old_id
            {
                nota.project = Some(new_id.to_string());
            }
        }
    }

    /// Add a nota (unified task/project/context)
    ///
    /// # Arguments
    /// * `nota` - The nota to add
    #[allow(dead_code)]
    pub fn add(&mut self, nota: Nota) {
        self.add_nota(nota);
    }

    /// Find a nota by its ID
    ///
    /// Searches across all notas.
    ///
    /// # Arguments
    /// * `id` - The nota ID to search for
    ///
    /// # Returns
    /// An optional Nota if found
    #[allow(dead_code)]
    pub fn find_by_id(&self, id: &str) -> Option<Nota> {
        self.find_nota_by_id(id).cloned()
    }

    /// Update a nota by its ID
    ///
    /// # Arguments
    /// * `id` - The nota ID to update
    /// * `nota` - The new nota data
    ///
    /// # Returns
    /// The old nota if found and replaced
    pub fn update(&mut self, id: &str, nota: Nota) -> Option<Nota> {
        if let Some(pos) = self.notas.iter().position(|n| n.id == id) {
            let old_nota = self.notas.remove(pos);
            self.notas.push(nota.clone());
            self.nota_map.insert(nota.id.clone(), nota.status.clone());
            Some(old_nota)
        } else {
            None
        }
    }

    /// Remove a nota by its ID
    ///
    /// Searches across all notas and removes if found.
    ///
    /// # Arguments
    /// * `id` - The nota ID to remove
    ///
    /// # Returns
    /// The removed Nota if found
    #[allow(dead_code)]
    pub fn remove(&mut self, id: &str) -> Option<Nota> {
        self.remove_nota(id)
    }

    /// List all notas with optional status filter
    ///
    /// # Arguments
    /// * `status_filter` - Optional status to filter by
    ///
    /// # Returns
    /// Vector of Nota objects matching the filter
    #[allow(dead_code)]
    pub fn list_all(&self, status_filter: Option<NotaStatus>) -> Vec<Nota> {
        if let Some(status) = status_filter {
            self.notas
                .iter()
                .filter(|n| n.status == status)
                .cloned()
                .collect()
        } else {
            self.notas.clone()
        }
    }

    /// Check if a nota ID is referenced by other notas
    ///
    /// Returns true if the ID is used in any nota's project or context fields.
    ///
    /// # Arguments
    /// * `id` - The nota ID to check
    ///
    /// # Returns
    /// True if the ID is referenced by other notas
    #[allow(dead_code)]
    pub fn is_referenced(&self, id: &str) -> bool {
        self.notas
            .iter()
            .any(|nota| nota.project.as_deref() == Some(id) || nota.context.as_deref() == Some(id))
    }

    // Compatibility properties for tests
    /// Get inbox notas (for compatibility)
    #[allow(dead_code)]
    pub fn inbox(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::inbox)
            .collect()
    }

    /// Get next_action notas (for compatibility)
    #[allow(dead_code)]
    pub fn next_action(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::next_action)
            .collect()
    }

    /// Get waiting_for notas (for compatibility)
    #[allow(dead_code)]
    pub fn waiting_for(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::waiting_for)
            .collect()
    }

    /// Get later notas (for compatibility)
    #[allow(dead_code)]
    pub fn later(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::later)
            .collect()
    }

    /// Get calendar notas (for compatibility)
    #[allow(dead_code)]
    pub fn calendar(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::calendar)
            .collect()
    }

    /// Get someday notas (for compatibility)
    #[allow(dead_code)]
    pub fn someday(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::someday)
            .collect()
    }

    /// Get done notas (for compatibility)
    #[allow(dead_code)]
    pub fn done(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::done)
            .collect()
    }

    /// Get trash notas (for compatibility)
    #[allow(dead_code)]
    pub fn trash(&self) -> Vec<&Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::trash)
            .collect()
    }

    /// Get projects map (for compatibility)
    #[allow(dead_code)]
    pub fn projects(&self) -> HashMap<String, &Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::project)
            .map(|n| (n.id.clone(), n))
            .collect()
    }

    /// Get contexts map (for compatibility)
    #[allow(dead_code)]
    pub fn contexts(&self) -> HashMap<String, &Nota> {
        self.notas
            .iter()
            .filter(|n| n.status == NotaStatus::context)
            .map(|n| (n.id.clone(), n))
            .collect()
    }

    /// Add a task (for compatibility with tests)
    #[allow(dead_code)]
    pub fn add_task(&mut self, task: Task) {
        self.add(Nota::from_task(task));
    }

    /// Remove a task (for compatibility with tests)
    #[allow(dead_code)]
    pub fn remove_task(&mut self, id: &str) -> Option<Task> {
        self.remove_nota(id).and_then(|n| n.to_task())
    }

    /// Add a project (for compatibility with tests)
    #[allow(dead_code)]
    pub fn add_project(&mut self, project: Project) {
        self.add(Nota::from_project(project));
    }

    /// Add a context (for compatibility with tests)
    #[allow(dead_code)]
    pub fn add_context(&mut self, context: Context) {
        self.add(Nota::from_context(context));
    }

    /// Validate task project (for compatibility)
    pub fn validate_task_project(&self, task: &Task) -> bool {
        match &task.project {
            None => true,
            Some(project_id) => self.find_project_by_id(project_id).is_some(),
        }
    }

    /// Validate task context (for compatibility)
    pub fn validate_task_context(&self, task: &Task) -> bool {
        match &task.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Validate task references (for compatibility)
    pub fn validate_task_references(&self, task: &Task) -> bool {
        self.validate_task_project(task) && self.validate_task_context(task)
    }

    /// Validate project context (for compatibility)
    pub fn validate_project_context(&self, project: &Project) -> bool {
        match &project.context {
            None => true,
            Some(context_name) => self.find_context_by_name(context_name).is_some(),
        }
    }

    /// Update project ID in tasks (for compatibility)
    pub fn update_project_id_in_tasks(&mut self, old_id: &str, new_id: &str) {
        self.update_project_id_in_notas(old_id, new_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate};

    // GtdDataの新規作成テスト
    // 空のnotasが初期化されることを確認
    #[test]
    fn test_gtd_data_new() {
        let data = GtdData::new();
        assert!(data.inbox().is_empty());
        assert!(data.next_action().is_empty());
        assert!(data.waiting_for().is_empty());
        assert!(data.someday().is_empty());
        assert!(data.later().is_empty());
        assert!(data.done().is_empty());
        assert!(data.trash().is_empty());
        assert!(data.projects().is_empty());
        assert!(data.contexts().is_empty());
    }

    // GtdDataへのNota挿入テスト
    // Notaを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_nota() {
        let mut data = GtdData::new();
        let nota = Nota {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add(nota.clone());
        assert_eq!(data.task_count(), 1);
        assert_eq!(data.inbox().len(), 1);
        assert_eq!(data.find_task_by_id("task-1").unwrap().title, "Test Task");
    }

    // 複数Notaの挿入テスト
    // 5つのNotaを追加し、すべて正しく格納されることを確認
    #[test]
    fn test_gtd_data_insert_multiple_notas() {
        let mut data = GtdData::new();

        for i in 1..=5 {
            let nota = Nota {
                id: format!("task-{}", i),
                title: format!("Test Task {}", i),
                status: NotaStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
                created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            };
            data.add(nota);
        }

        assert_eq!(data.task_count(), 5);
        assert_eq!(data.inbox().len(), 5);
    }

    // Notaステータスの更新テスト
    // NotaのステータスをInboxからNextActionに更新し、正しく反映されることを確認
    #[test]
    fn test_gtd_data_update_nota_status() {
        let mut data = GtdData::new();
        let nota_id = "task-1".to_string();
        let nota = Nota {
            id: nota_id.clone(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add(nota);

        // Update status
        data.move_status(&nota_id, NotaStatus::next_action);

        assert!(matches!(
            data.find_task_by_id(&nota_id).unwrap().status,
            NotaStatus::next_action
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
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.task_count(), 1);
        assert_eq!(data.inbox().len(), 1);

        data.remove_task(&task_id);
        assert_eq!(data.task_count(), 0);
        assert_eq!(data.inbox().len(), 0);
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
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.inbox().len(), 1);
        assert_eq!(data.trash().len(), 0);

        // Move task to trash
        let result = data.move_status(&task_id, NotaStatus::trash);
        assert!(result.is_some());

        // Verify task was moved
        assert_eq!(data.inbox().len(), 0);
        assert_eq!(data.trash().len(), 1);
        assert_eq!(data.task_count(), 1);

        // Verify task status was updated
        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(moved_task.status, NotaStatus::trash));
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
            status: NotaStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.next_action().len(), 1);
        assert_eq!(data.done().len(), 0);

        // Move task to done
        let result = data.move_status(&task_id, NotaStatus::done);
        assert!(result.is_some());

        // Verify task was moved
        assert_eq!(data.next_action().len(), 0);
        assert_eq!(data.done().len(), 1);
        assert_eq!(data.task_count(), 1);

        // Verify task status was updated
        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(moved_task.status, NotaStatus::done));
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
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);

        // inbox -> next_action
        data.move_status(&task_id, NotaStatus::next_action);
        assert_eq!(data.inbox().len(), 0);
        assert_eq!(data.next_action().len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            NotaStatus::next_action
        ));

        // next_action -> waiting_for
        data.move_status(&task_id, NotaStatus::waiting_for);
        assert_eq!(data.next_action().len(), 0);
        assert_eq!(data.waiting_for().len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            NotaStatus::waiting_for
        ));

        // waiting_for -> done
        data.move_status(&task_id, NotaStatus::done);
        assert_eq!(data.waiting_for().len(), 0);
        assert_eq!(data.done().len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            NotaStatus::done
        ));

        // done -> trash
        data.move_status(&task_id, NotaStatus::trash);
        assert_eq!(data.done().len(), 0);
        assert_eq!(data.trash().len(), 1);
        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
            NotaStatus::trash
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
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: Some(date),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);
        assert_eq!(data.inbox().len(), 1);

        // inbox -> calendar
        let result = data.move_status(&task_id, NotaStatus::calendar);
        assert!(result.is_some());
        assert_eq!(data.inbox().len(), 0);
        assert_eq!(data.calendar().len(), 1);

        let moved_task = data.find_task_by_id(&task_id).unwrap();
        assert!(matches!(moved_task.status, NotaStatus::calendar));
        assert_eq!(moved_task.start_date.unwrap(), date);
    }

    // ステータス移動テスト - 存在しないタスク
    // 存在しないタスクの移動がNoneを返すことを確認
    #[test]
    fn test_gtd_data_move_status_nonexistent_task() {
        let mut data = GtdData::new();
        let result = data.move_status("nonexistent-id", NotaStatus::trash);
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
            status: NotaStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("Office".to_string()),
            notes: Some("Important notes".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        data.add_task(task);

        // Move task to next_action
        data.move_status(&task_id, NotaStatus::next_action);

        // Verify all properties are preserved (except updated_at which should be updated)
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
        // Note: updated_at is automatically updated by move_status to reflect the change
        assert!(matches!(moved_task.status, NotaStatus::next_action));
    }

    // プロジェクトとコンテキスト付きタスクのテスト
    // プロジェクト、コンテキスト、ノートが正しく設定されることを確認
    #[test]
    fn test_task_with_project_and_context() {
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            status: NotaStatus::calendar,
            project: None,
            context: None,
            notes: None,
            start_date: Some(date),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        assert!(matches!(task.status, NotaStatus::calendar));
        assert_eq!(task.start_date.unwrap(), date);
    }

    // タスクステータスの全バリアントテスト
    // 8種類のタスクステータス（Inbox、NextAction、WaitingFor、Someday、Later、Done、Trash、Calendar）がすべて正しく動作することを確認
    #[test]
    fn test_task_status_variants() {
        let statuses = vec![
            NotaStatus::inbox,
            NotaStatus::next_action,
            NotaStatus::waiting_for,
            NotaStatus::someday,
            NotaStatus::later,
            NotaStatus::done,
            NotaStatus::trash,
            NotaStatus::calendar,
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
                NotaStatus::inbox => assert!(matches!(task.status, NotaStatus::inbox)),
                NotaStatus::next_action => assert!(matches!(task.status, NotaStatus::next_action)),
                NotaStatus::waiting_for => assert!(matches!(task.status, NotaStatus::waiting_for)),
                NotaStatus::someday => assert!(matches!(task.status, NotaStatus::someday)),
                NotaStatus::later => assert!(matches!(task.status, NotaStatus::later)),
                NotaStatus::done => assert!(matches!(task.status, NotaStatus::done)),
                NotaStatus::trash => assert!(matches!(task.status, NotaStatus::trash)),
                NotaStatus::calendar => assert!(matches!(task.status, NotaStatus::calendar)),
                NotaStatus::context | NotaStatus::project => {
                    panic!("context and project are not task statuses")
                }
            }
        }
    }

    // プロジェクトの作成テスト
    // プロジェクトを作成し、ID、名前、説明、ステータスが正しく設定されることを確認
    #[test]
    fn test_project_creation() {
        let project = Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: Some("Test description".to_string()),
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        };

        assert_eq!(project.id, "project-1");
        assert_eq!(project.title, "Test Project");
        assert_eq!(project.notes.as_ref().unwrap(), "Test description");
    }

    // 説明なしプロジェクトのテスト
    // 説明を持たないプロジェクトが正しく作成されることを確認
    #[test]
    fn test_project_without_description() {
        let project = Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        };

        assert!(project.notes.is_none());
    }

    // プロジェクトステータスの全バリアントテスト
    // 3種類のプロジェクトステータス（Active、OnHold、Completed）がすべて正しく動作することを確認
    // GtdDataへのプロジェクト挿入テスト
    // プロジェクトを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_project() {
        let mut data = GtdData::new();
        let project = Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        };

        data.add_project(project.clone());
        assert_eq!(data.projects().len(), 1);
        assert_eq!(
            data.find_project_by_id("project-1").unwrap().title,
            "Test Project"
        );
    }

    // プロジェクトステータスの更新テスト
    // コンテキストの作成テスト
    // コンテキストを作成し、IDと名前が正しく設定されることを確認
    #[test]
    fn test_context_creation() {
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

        assert_eq!(context.name, "Office");
        assert_eq!(context.notes, None);
    }

    // コンテキストの説明付き作成テスト
    // 説明フィールドを持つコンテキストが正しく作成されることを確認
    #[test]
    fn test_context_with_description() {
        let context = Context {
            name: "Office".to_string(),
            notes: Some("Work environment with desk and computer".to_string()),
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        };

        assert_eq!(context.name, "Office");
        assert_eq!(
            context.notes,
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
        assert_eq!(data.contexts().len(), 1);
        assert_eq!(data.find_context_by_name("Office").unwrap().id, "Office");
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

        assert_eq!(data.contexts().len(), 4);
    }

    // タスクのシリアライゼーションテスト
    // タスクをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    #[test]
    fn test_task_serialization() {
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
            title: "Test Project".to_string(),
            notes: Some("Test description".to_string()),
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        };

        data.add_project(project.clone());

        let serialized = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&serialized).unwrap();

        let deserialized_projects = deserialized.projects();
        let deserialized_project = deserialized_projects.get("project-1").unwrap();
        assert_eq!(project.id, deserialized_project.id);
        assert_eq!(project.title, deserialized_project.title);
        assert_eq!(project.notes, deserialized_project.notes);
    }

    // コンテキストのシリアライゼーションテスト
    // コンテキストをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    // Note: name フィールドは skip_serializing されるため、TOML には含まれない
    // Context serialization test for Version 3
    // In V3 format, contexts are stored in [[context]] arrays, so name must be serialized
    #[test]
    fn test_context_serialization() {
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

        let serialized = toml::to_string(&context).unwrap();
        // In Version 3, name field is serialized as part of the [[context]] array
        assert!(
            serialized.contains("name"),
            "name field should be serialized in Version 3"
        );

        let deserialized: Context = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "Office");
        assert_eq!(deserialized.notes, None);
    }

    // GtdData全体のシリアライゼーションテスト
    // タスク、プロジェクト、コンテキストを含むGtdDataをTOML形式にシリアライズし、デシリアライズして各要素数が一致することを確認
    #[test]
    fn test_gtd_data_serialization() {
        let mut data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
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

        let project = Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        };
        data.add_project(project);

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
        data.add_context(context);

        let serialized = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&serialized).unwrap();

        assert_eq!(data.task_count(), deserialized.task_count());
        assert_eq!(data.projects().len(), deserialized.projects().len());
        assert_eq!(data.contexts().len(), deserialized.contexts().len());
    }

    // ステータスによるタスクフィルタリングテスト
    // 複数のステータスを持つタスクを追加し、特定のステータスでフィルタリングできることを確認
    #[test]
    fn test_task_filter_by_status() {
        let mut data = GtdData::new();

        let statuses = [
            NotaStatus::inbox,
            NotaStatus::next_action,
            NotaStatus::waiting_for,
            NotaStatus::someday,
            NotaStatus::later,
            NotaStatus::done,
            NotaStatus::trash,
            NotaStatus::calendar,
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
        assert_eq!(data.inbox().len(), 1);

        // Filter by Done
        assert_eq!(data.done().len(), 1);

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
                status: NotaStatus::inbox,
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

        let all_tasks = data.list_all(None);
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
                status: NotaStatus::inbox,
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

        let all_tasks = data.list_all(None);
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
            status: NotaStatus::inbox,
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

        // Add a task to next_action to verify the status field is snake_case
        data.add_task(Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        });

        let serialized = toml::to_string(&data).unwrap();
        // V5 format uses [[next_action]] with status field
        assert!(
            serialized.contains("[[next_action]]"),
            "Expected '[[next_action]]' in TOML output"
        );
        assert!(
            serialized.contains("status = \"next_action\""),
            "Expected 'status = \"next_action\"' in TOML output"
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

        // Verify that tasks maintain insertion order
        assert_eq!(data.inbox().len(), 5);
        let data_inbox = data.inbox();
        for (i, task) in data_inbox.iter().enumerate() {
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
                status: NotaStatus::inbox,
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
                title: format!("Project {}", i),
                notes: None,
                project: None,
                context: None,
                start_date: None,
                created_at: local_date_today(),
                updated_at: local_date_today(),
            });
        }

        let toml_str = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&toml_str).unwrap();

        // Verify deserialized data maintains insertion order for tasks
        assert_eq!(deserialized.inbox().len(), 3);
        let deserialized_inbox = deserialized.inbox();
        for (i, task) in deserialized_inbox.iter().enumerate() {
            assert_eq!(task.id, format!("task-{}", i + 1));
        }

        // Verify all projects are present (HashMap doesn't guarantee order)
        assert_eq!(deserialized.projects().len(), 2);
        assert!(deserialized.projects().contains_key("project-1"));
        assert!(deserialized.projects().contains_key("project-2"));
    }

    // 完全なTOML出力テスト（全フィールド設定）
    // 全フィールドを設定した状態でTOML出力を検証し、意図したテキスト形式で出力されることを確認する
    // V4形式: 統一された[[notas]]配列を使用
    #[test]
    fn test_complete_toml_output() {
        let mut data = GtdData::new();

        // 全フィールドを設定したタスクを追加
        data.add_task(Task {
            id: "task-001".to_string(),
            title: "Complete project documentation".to_string(),
            status: NotaStatus::next_action,
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
            status: NotaStatus::inbox,
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
            title: "Documentation Project".to_string(),
            notes: Some("Comprehensive project documentation update".to_string()),
            project: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            context: None,
        });

        // 説明付きコンテキストを追加
        data.add_context(Context {
            name: "Office".to_string(),
            notes: Some("Work environment with desk and computer".to_string()),
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        });

        // TOML出力を生成
        let toml_output = toml::to_string_pretty(&data).unwrap();

        // TOML構造と可読性を確認
        println!(
            "\n=== TOML Output (V5) ===\n{}\n===================\n",
            toml_output
        );

        // V5形式の期待される構造を検証
        assert!(
            toml_output.contains("format_version = 5"),
            "Should be version 5"
        );
        assert!(
            toml_output.contains("[[inbox]]"),
            "Should have [[inbox]] section"
        );
        assert!(
            toml_output.contains("[[next_action]]"),
            "Should have [[next_action]] section"
        );
        assert!(
            toml_output.contains("[[project]]"),
            "Should have [[project]] section"
        );
        assert!(
            toml_output.contains("[[context]]"),
            "Should have [[context]] section"
        );

        // 各アイテムが含まれていることを確認
        assert!(toml_output.contains("id = \"task-001\""));
        assert!(toml_output.contains("id = \"task-002\""));
        assert!(toml_output.contains("id = \"project-001\""));
        assert!(toml_output.contains("id = \"Office\""));

        // ステータスが正しく含まれていることを確認
        assert!(toml_output.contains("status = \"next_action\""));
        assert!(toml_output.contains("status = \"inbox\""));
        assert!(toml_output.contains("status = \"project\""));
        assert!(toml_output.contains("status = \"context\""));

        // デシリアライゼーションが正しく動作することを確認
        let deserialized: GtdData = toml::from_str(&toml_output).unwrap();

        // 全タスクフィールドを検証
        assert_eq!(deserialized.inbox().len(), 1);
        assert_eq!(deserialized.next_action().len(), 1);

        let task_inbox = &deserialized.inbox()[0];
        assert_eq!(task_inbox.id, "task-002");
        assert_eq!(task_inbox.title, "Quick task");
        assert!(matches!(task_inbox.status, NotaStatus::inbox));

        let task1 = &deserialized.next_action()[0];
        assert_eq!(task1.id, "task-001");
        assert_eq!(task1.title, "Complete project documentation");
        assert!(matches!(task1.status, NotaStatus::next_action));
        assert_eq!(task1.project, Some("project-001".to_string()));
        assert_eq!(task1.context, Some("Office".to_string()));
        assert_eq!(
            task1.notes,
            Some("Review all sections and update examples".to_string())
        );
        assert_eq!(task1.start_date, NaiveDate::from_ymd_opt(2024, 3, 15));

        // プロジェクトフィールドを検証
        assert_eq!(deserialized.projects().len(), 1);
        let deserialized_projects = deserialized.projects();
        let project1 = deserialized_projects.get("project-001").unwrap();
        assert_eq!(project1.id, "project-001");
        assert_eq!(project1.title, "Documentation Project");
        assert_eq!(
            project1.notes,
            Some("Comprehensive project documentation update".to_string())
        );

        // コンテキストフィールドを検証
        assert_eq!(deserialized.contexts().len(), 1);

        let deserialized_contexts = deserialized.contexts();
        let context_office = deserialized_contexts.get("Office").unwrap();
        assert_eq!(context_office.id, "Office");
        assert_eq!(
            context_office.notes,
            Some("Work environment with desk and computer".to_string())
        );
    }

    // 後方互換性テスト: 旧形式（nameフィールド付き）のTOMLも正しく読み込めることを確認
    // Test backward compatibility with name field in contexts (Version 2 format)
    // Version 2 used HashMap format where name was the key, so name field was redundant
    // Version 3 uses Vec format where name must be included
    #[test]
    fn test_backward_compatibility_with_name_field() {
        // 旧形式のTOML（nameフィールドが含まれている）- Version 2 HashMap format
        let old_format_toml = r#"
[[tasks]]
id = "task-001"
title = "Test task"

[contexts.Office]
name = "Office"
notes = "Work environment with desk and computer"

[contexts.Home]
name = "Home"
"#;

        // 旧形式のTOMLを読み込めることを確認
        let deserialized: GtdData = toml::from_str(old_format_toml).unwrap();

        assert_eq!(deserialized.contexts().len(), 2);

        // Officeコンテキストを検証
        let deserialized_contexts = deserialized.contexts();
        let office = deserialized_contexts.get("Office").unwrap();
        assert_eq!(office.id, "Office");
        assert_eq!(
            office.notes,
            Some("Work environment with desk and computer".to_string())
        );

        // Homeコンテキストを検証
        let deserialized_contexts = deserialized.contexts();
        let home = deserialized_contexts.get("Home").unwrap();
        assert_eq!(home.id, "Home");
        assert_eq!(home.notes, None);

        // 再シリアライズするとVersion 5形式（status-based arrays）になることを確認
        let reserialized = toml::to_string_pretty(&deserialized).unwrap();
        assert!(
            reserialized.contains("[[context]]"),
            "Reserialized TOML should use [[context]] array format"
        );
        assert!(
            reserialized.contains("id = \"Office\""),
            "Reserialized TOML should contain id field"
        );
        assert!(
            reserialized.contains("id = \"Home\""),
            "Reserialized TOML should contain id field"
        );
        assert!(
            reserialized.contains("status = \"context\""),
            "Reserialized TOML should contain status = \"context\""
        );
    }

    // 参照整合性検証テスト - プロジェクト参照が有効
    // タスクのプロジェクト参照が存在するプロジェクトを指している場合、検証が成功することを確認
    #[test]
    fn test_validate_task_project_valid() {
        let mut data = GtdData::new();

        data.add_project(Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            notes: None,
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        });

        data.add_context(Context {
            name: "Office".to_string(),
            notes: None,
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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
            notes: None,
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
        });

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
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
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: created_date,
            updated_at: created_date,
        };

        // タスクを更新
        task.status = NotaStatus::next_action;
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
            status: NotaStatus::inbox,
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
            task.status = NotaStatus::next_action;
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
            status: NotaStatus::inbox,
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
            notes: None,
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        });

        let project = Project {
            id: "project-1".to_string(),
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
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
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
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
            title: "Test Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
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
            notes: Some("Work environment".to_string()),
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        });

        let project = Project {
            id: "project-1".to_string(),
            title: "Office Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: Some("Office".to_string()),
        };
        data.add_project(project.clone());

        let task = Task {
            id: "task-1".to_string(),
            title: "Office Task".to_string(),
            status: NotaStatus::next_action,
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
title = "Old Project"
notes = "Project without context field"
"#;

        let data: GtdData = toml::from_str(toml_str).unwrap();
        assert_eq!(data.projects().len(), 1);

        let projects = data.projects();
        let project = projects.get("project-1").unwrap();
        assert_eq!(project.id, "project-1");
        assert_eq!(project.title, "Old Project");
        assert_eq!(project.context, None);
    }

    // フォーマットバージョン1からバージョン3への自動マイグレーションテスト
    // 旧形式（Vec<Project>）のTOMLを読み込み、新形式（HashMap）に自動変換され、バージョン3で保存されることを確認
    #[test]
    fn test_format_migration_v1_to_v4() {
        // Format version 1: projects as array ([[projects]])
        let old_format_toml = r#"
[[projects]]
id = "project-1"
title = "First Project"
notes = "Original format"

[[projects]]
id = "project-2"
title = "Second Project"

[[inbox]]
id = "task-1"
title = "Test task"
project = "project-1"
created_at = "2024-01-01"
updated_at = "2024-01-01"
"#;

        // Load old format
        let data: GtdData = toml::from_str(old_format_toml).unwrap();

        // Verify it's automatically migrated to version 5
        assert_eq!(data.format_version, 5);
        assert_eq!(data.projects().len(), 2);

        // Verify projects are accessible
        let data_projects = data.projects();
        let project1 = data_projects.get("project-1").unwrap();
        assert_eq!(project1.id, "project-1");
        assert_eq!(project1.title, "First Project");

        let data_projects = data.projects();
        let project2 = data_projects.get("project-2").unwrap();
        assert_eq!(project2.id, "project-2");
        assert_eq!(project2.title, "Second Project");

        // Verify task references still work
        assert_eq!(data.inbox().len(), 1);
        assert_eq!(data.inbox()[0].project, Some("project-1".to_string()));

        // Save to new format
        let new_format_toml = toml::to_string_pretty(&data).unwrap();

        // Verify new format has status-based arrays and version 5
        assert!(new_format_toml.contains("format_version = 5"));
        assert!(new_format_toml.contains("[[inbox]]"));
        assert!(new_format_toml.contains("[[project]]"));
        assert!(!new_format_toml.contains("[[notas]]"));

        // Verify round-trip works
        let reloaded: GtdData = toml::from_str(&new_format_toml).unwrap();
        assert_eq!(reloaded.format_version, 5);
        assert_eq!(reloaded.projects().len(), 2);
        assert!(reloaded.projects().contains_key("project-1"));
        assert!(reloaded.projects().contains_key("project-2"));
    }

    // フォーマットバージョン2からバージョン5への自動マイグレーションテスト
    // バージョン2形式のTOMLを読み込み、バージョン5で保存されることを確認
    #[test]
    fn test_format_migration_v2_to_v4() {
        // Format version 2: projects as HashMap
        let v2_format_toml = r##"
format_version = 2

[[inbox]]
id = "#1"
title = "Test task"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[projects.project-1]
title = "Test Project"
notes = "Version 2 format"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[contexts.Office]
notes = "Office context"
"##;

        // Load version 2 format
        let data: GtdData = toml::from_str(v2_format_toml).unwrap();

        // Verify it's automatically migrated to version 5
        assert_eq!(data.format_version, 5);
        assert_eq!(data.inbox().len(), 1);
        assert_eq!(data.projects().len(), 1);
        assert_eq!(data.contexts().len(), 1);

        // Verify data integrity
        let task = &data.inbox()[0];
        assert_eq!(task.id, "#1");
        assert_eq!(task.title, "Test task");

        let projects = data.projects();
        let project = projects.get("project-1").unwrap();
        assert_eq!(project.title, "Test Project");

        let contexts = data.contexts();
        let context = contexts.get("Office").unwrap();
        assert_eq!(context.id, "Office");

        // Save to new format
        let new_format_toml = toml::to_string_pretty(&data).unwrap();

        // Verify new format has version 5 and status-based arrays
        assert!(new_format_toml.contains("format_version = 5"));
        assert!(new_format_toml.contains("[[inbox]]"));
        assert!(new_format_toml.contains("[[project]]"));
        assert!(new_format_toml.contains("[[context]]"));

        // Verify round-trip works
        let reloaded: GtdData = toml::from_str(&new_format_toml).unwrap();
        assert_eq!(reloaded.format_version, 5);
        assert_eq!(reloaded.inbox().len(), 1);
        assert_eq!(reloaded.projects().len(), 1);
        assert_eq!(reloaded.contexts().len(), 1);
    }

    // NotaStatus::from_strのテスト - 有効なステータス
    // 全ての有効なステータス文字列が正しくパースされることを確認
    #[test]
    fn test_task_status_from_str_valid() {
        assert_eq!(NotaStatus::from_str("inbox").unwrap(), NotaStatus::inbox);
        assert_eq!(
            NotaStatus::from_str("next_action").unwrap(),
            NotaStatus::next_action
        );
        assert_eq!(
            NotaStatus::from_str("waiting_for").unwrap(),
            NotaStatus::waiting_for
        );
        assert_eq!(
            NotaStatus::from_str("someday").unwrap(),
            NotaStatus::someday
        );
        assert_eq!(NotaStatus::from_str("later").unwrap(), NotaStatus::later);
        assert_eq!(
            NotaStatus::from_str("calendar").unwrap(),
            NotaStatus::calendar
        );
        assert_eq!(NotaStatus::from_str("done").unwrap(), NotaStatus::done);
        assert_eq!(NotaStatus::from_str("trash").unwrap(), NotaStatus::trash);
    }

    // NotaStatus::from_strのテスト - 無効なステータス
    // 無効なステータス文字列が適切なエラーメッセージを返すことを確認
    #[test]
    fn test_task_status_from_str_invalid() {
        let result = NotaStatus::from_str("invalid_status");
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Invalid status 'invalid_status'"));
        assert!(err_msg.contains("inbox"));
        assert!(err_msg.contains("next_action"));
        assert!(err_msg.contains("waiting_for"));
        assert!(err_msg.contains("someday"));
        assert!(err_msg.contains("later"));
        assert!(err_msg.contains("calendar"));
        assert!(err_msg.contains("done"));
        assert!(err_msg.contains("trash"));
    }

    // NotaStatus::from_strのテスト - 大文字小文字の違い
    // 大文字小文字が異なる場合はエラーになることを確認（厳密な一致が必要）
    #[test]
    fn test_task_status_from_str_case_sensitive() {
        assert!(NotaStatus::from_str("Inbox").is_err());
        assert!(NotaStatus::from_str("INBOX").is_err());
        assert!(NotaStatus::from_str("Next_Action").is_err());
        assert!(NotaStatus::from_str("NEXT_ACTION").is_err());
    }

    // NotaStatus::from_strのテスト - 存在しない一般的な名前
    // よくある誤りのステータス名がエラーになることを確認
    #[test]
    fn test_task_status_from_str_common_mistakes() {
        // 問題として報告された "in_progress" をテスト
        let result = NotaStatus::from_str("in_progress");
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Invalid status 'in_progress'"));

        // その他の一般的な誤り
        assert!(NotaStatus::from_str("complete").is_err());
        assert!(NotaStatus::from_str("completed").is_err());
        assert!(NotaStatus::from_str("pending").is_err());
        assert!(NotaStatus::from_str("todo").is_err());
        assert!(NotaStatus::from_str("in-progress").is_err());
    }

    // タスクステータスの順序がTOMLシリアライズに反映されることを確認
    // NotaStatus enumの順序とGtdDataフィールドの順序が一致し、TOML出力もその順序になることを検証
    #[test]
    fn test_task_status_order_in_toml_serialization() {
        let mut data = GtdData::new();

        // Add one task for each status in enum order
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

        // V5 format uses status-based arrays
        assert!(
            toml_str.contains("[[inbox]]"),
            "Should contain [[inbox]] section"
        );
        assert!(
            toml_str.contains("format_version = 5"),
            "Should be version 5"
        );

        // Verify all statuses are represented in their own sections
        for status in &statuses {
            let status_str = format!("{:?}", status);
            assert!(
                toml_str.contains(&format!("status = \"{}\"", status_str)),
                "Should contain status = \"{}\"",
                status_str
            );
        }
    }

    // Tests for task_map HashMap functionality
    #[test]
    fn test_task_map_prevents_duplicate_ids() {
        let mut data = GtdData::new();

        // Add a task
        let task1 = Task {
            id: "test-task".to_string(),
            title: "Test Task 1".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task1);

        // Verify task is in map
        assert!(data.nota_map.contains_key("test-task"));
        assert_eq!(data.nota_map.get("test-task"), Some(&NotaStatus::inbox));

        // Try to add another task with same ID in a different status
        let task2 = Task {
            id: "test-task".to_string(),
            title: "Test Task 2".to_string(),
            status: NotaStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        // This would add a duplicate - the application layer (lib.rs) should check
        // the task_map before calling add_task
        // Here we just verify that task_map gets updated
        data.add_task(task2);

        // The task_map should now show the new status (last one wins)
        assert_eq!(
            data.nota_map.get("test-task"),
            Some(&NotaStatus::next_action)
        );

        // But there are actually TWO tasks with same ID (one in inbox, one in next_action)
        // This demonstrates why the application layer MUST check task_map before adding
        assert_eq!(data.inbox().len(), 1);
        assert_eq!(data.next_action().len(), 1);
    }

    #[test]
    fn test_task_map_updated_on_remove() {
        let mut data = GtdData::new();

        let task = Task {
            id: "remove-test".to_string(),
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

        // Verify task is in map
        assert!(data.nota_map.contains_key("remove-test"));

        // Remove task
        let removed = data.remove_task("remove-test");
        assert!(removed.is_some());

        // Verify task is removed from map
        assert!(!data.nota_map.contains_key("remove-test"));
    }

    #[test]
    fn test_task_map_updated_on_status_change() {
        let mut data = GtdData::new();

        let task = Task {
            id: "status-test".to_string(),
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

        // Verify initial status
        assert_eq!(data.nota_map.get("status-test"), Some(&NotaStatus::inbox));

        // Move to next_action
        data.move_status("status-test", NotaStatus::next_action);

        // Verify status updated in map
        assert_eq!(
            data.nota_map.get("status-test"),
            Some(&NotaStatus::next_action)
        );
    }

    #[test]
    fn test_task_map_rebuilt_from_toml() {
        // Test that task_map is correctly rebuilt when loading from TOML (format version 2)
        let toml_str = r#"
format_version = 2

[[inbox]]
id = "task-1"
title = "First task"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[next_action]]
id = "task-2"
title = "Second task"
created_at = "2024-01-01"
updated_at = "2024-01-01"
"#;

        let data: GtdData = toml::from_str(toml_str).unwrap();

        // Verify both tasks are in task_map with correct statuses
        assert_eq!(data.nota_map.len(), 2);
        assert_eq!(data.nota_map.get("task-1"), Some(&NotaStatus::inbox));
        assert_eq!(data.nota_map.get("task-2"), Some(&NotaStatus::next_action));
    }

    // Step 4: Test HashMap serialization order
    #[test]
    fn test_hashmap_serialization_order() {
        use std::collections::HashMap;

        // Create a HashMap with tasks
        let mut tasks_map: HashMap<String, Task> = HashMap::new();

        // Add tasks in a specific order
        for i in 1..=5 {
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
            tasks_map.insert(task.id.clone(), task);
        }

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&tasks_map).unwrap();
        println!("HashMap serialization order:\n{}", toml_str);

        // HashMap in Rust does NOT guarantee order
        // This test documents that HashMap does NOT maintain insertion order
        // Therefore, we should keep Vec-based serialization for TOML readability
        assert!(toml_str.contains("task-1"));
        assert!(toml_str.contains("task-2"));
        assert!(toml_str.contains("task-3"));
        assert!(toml_str.contains("task-4"));
        assert!(toml_str.contains("task-5"));
    }

    #[test]
    fn test_vec_serialization_maintains_order() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct TestContainer {
            tasks: Vec<Task>,
        }

        // Create a Vec with tasks in order
        let mut tasks_vec: Vec<Task> = Vec::new();

        for i in 1..=5 {
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
            tasks_vec.push(task);
        }

        let container = TestContainer { tasks: tasks_vec };

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&container).unwrap();
        println!("Vec serialization order:\n{}", toml_str);

        // Vec maintains order - verify tasks appear in sequential order
        let task1_pos = toml_str.find("task-1").unwrap();
        let task2_pos = toml_str.find("task-2").unwrap();
        let task3_pos = toml_str.find("task-3").unwrap();
        let task4_pos = toml_str.find("task-4").unwrap();
        let task5_pos = toml_str.find("task-5").unwrap();

        // Verify tasks appear in order
        assert!(task1_pos < task2_pos);
        assert!(task2_pos < task3_pos);
        assert!(task3_pos < task4_pos);
        assert!(task4_pos < task5_pos);
    }

    // Tests for Nota structure (Step 6)
    #[test]
    fn test_nota_from_task() {
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("Office".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        };

        let nota = Nota::from_task(task.clone());

        assert_eq!(nota.id, task.id);
        assert_eq!(nota.title, task.title);
        assert_eq!(nota.status, task.status);
        assert_eq!(nota.project, task.project);
        assert_eq!(nota.context, task.context);
        assert_eq!(nota.notes, task.notes);
        assert!(nota.is_task());
        assert!(!nota.is_project());
        assert!(!nota.is_context());
    }

    #[test]
    fn test_nota_from_project() {
        let project = Project {
            id: "proj-1".to_string(),
            title: "Test Project".to_string(),
            notes: Some("Project notes".to_string()),
            project: None,
            context: Some("Office".to_string()),
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        let nota = Nota::from_project(project.clone());

        assert_eq!(nota.id, project.id);
        assert_eq!(nota.title, project.title);
        assert_eq!(nota.status, NotaStatus::project);
        assert_eq!(nota.context, project.context);
        assert_eq!(nota.notes, project.notes);
        assert!(!nota.is_task());
        assert!(nota.is_project());
        assert!(!nota.is_context());
    }

    #[test]
    fn test_nota_from_context() {
        let context = Context {
            name: "Office".to_string(),
            title: Some("Office Context".to_string()),
            notes: Some("Office location".to_string()),
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            updated_at: Some(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()),
        };

        let nota = Nota::from_context(context.clone());

        assert_eq!(nota.id, context.name);
        assert_eq!(nota.title, "Office Context");
        assert_eq!(nota.status, NotaStatus::context);
        assert_eq!(nota.notes, context.notes);
        assert!(!nota.is_task());
        assert!(!nota.is_project());
        assert!(nota.is_context());
    }

    #[test]
    fn test_nota_to_task() {
        let nota = Nota {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::next_action,
            project: Some("proj-1".to_string()),
            context: Some("Office".to_string()),
            notes: Some("Notes".to_string()),
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        };

        let task = nota.to_task().unwrap();

        assert_eq!(task.id, nota.id);
        assert_eq!(task.title, nota.title);
        assert_eq!(task.status, nota.status);
    }

    #[test]
    fn test_nota_to_task_fails_for_project() {
        let nota = Nota {
            id: "proj-1".to_string(),
            title: "Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        assert!(nota.to_task().is_none());
    }

    #[test]
    fn test_nota_to_project() {
        let nota = Nota {
            id: "proj-1".to_string(),
            title: "Test Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: Some("Office".to_string()),
            notes: Some("Project notes".to_string()),
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        let project = nota.to_project().unwrap();

        assert_eq!(project.id, nota.id);
        assert_eq!(project.title, nota.title);
        assert_eq!(project.context, nota.context);
    }

    #[test]
    fn test_nota_to_context() {
        let nota = Nota {
            id: "Office".to_string(),
            title: "Office Context".to_string(),
            status: NotaStatus::context,
            project: None,
            context: None,
            notes: Some("Office location".to_string()),
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        let context = nota.to_context().unwrap();

        assert_eq!(context.name, nota.id);
        assert_eq!(context.title, Some(nota.title));
        assert_eq!(context.notes, nota.notes);
    }

    // Nota追加テスト - タスクとして追加
    #[test]
    fn test_add_as_task() {
        let mut data = GtdData::new();
        let nota = Nota {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        data.add(nota.clone());
        assert_eq!(data.task_count(), 1);
        assert_eq!(data.inbox().len(), 1);

        let found = data.find_by_id("task-1").unwrap();
        assert_eq!(found.title, "Test Task");
        assert!(found.is_task());
    }

    // Nota追加テスト - プロジェクトとして追加
    #[test]
    fn test_add_as_project() {
        let mut data = GtdData::new();
        let nota = Nota {
            id: "proj-1".to_string(),
            title: "Test Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        data.add(nota.clone());
        assert_eq!(data.projects().len(), 1);

        let found = data.find_by_id("proj-1").unwrap();
        assert_eq!(found.title, "Test Project");
        assert!(found.is_project());
    }

    // Nota追加テスト - コンテキストとして追加
    #[test]
    fn test_add_as_context() {
        let mut data = GtdData::new();
        let nota = Nota {
            id: "Office".to_string(),
            title: "Office Context".to_string(),
            status: NotaStatus::context,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        data.add(nota.clone());
        assert_eq!(data.contexts().len(), 1);

        let found = data.find_by_id("Office").unwrap();
        assert_eq!(found.title, "Office Context");
        assert!(found.is_context());
    }

    // Nota削除テスト
    #[test]
    fn test_remove() {
        let mut data = GtdData::new();
        let nota = Nota {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        data.add(nota.clone());
        assert_eq!(data.task_count(), 1);

        let removed = data.remove("task-1").unwrap();
        assert_eq!(removed.title, "Test Task");
        assert_eq!(data.task_count(), 0);
    }

    // Nota一覧テスト
    #[test]
    fn test_list_all() {
        let mut data = GtdData::new();

        // Add a task
        data.add(Nota {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        // Add a project
        data.add(Nota {
            id: "proj-1".to_string(),
            title: "Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        // Add a context
        data.add(Nota {
            id: "Office".to_string(),
            title: "Office".to_string(),
            status: NotaStatus::context,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        let all_notas = data.list_all(None);
        assert_eq!(all_notas.len(), 3);

        let tasks_only = data.list_all(Some(NotaStatus::inbox));
        assert_eq!(tasks_only.len(), 1);

        let projects_only = data.list_all(Some(NotaStatus::project));
        assert_eq!(projects_only.len(), 1);

        let contexts_only = data.list_all(Some(NotaStatus::context));
        assert_eq!(contexts_only.len(), 1);
    }

    // Nota参照チェックテスト
    #[test]
    fn test_is_nota_referenced() {
        let mut data = GtdData::new();

        // Add a project
        data.add(Nota {
            id: "proj-1".to_string(),
            title: "Project".to_string(),
            status: NotaStatus::project,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        // Add a context
        data.add(Nota {
            id: "Office".to_string(),
            title: "Office".to_string(),
            status: NotaStatus::context,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        // Add a task that references both
        data.add(Nota {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            status: NotaStatus::inbox,
            project: Some("proj-1".to_string()),
            context: Some("Office".to_string()),
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        assert!(data.is_referenced("proj-1"));
        assert!(data.is_referenced("Office"));
        assert!(!data.is_referenced("task-1"));
    }

    // Nota更新テスト
    #[test]
    fn test_update() {
        let mut data = GtdData::new();

        // Add a nota
        data.add(Nota {
            id: "task-1".to_string(),
            title: "Old Title".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });

        // Update it
        let updated = Nota {
            id: "task-1".to_string(),
            title: "New Title".to_string(),
            status: NotaStatus::next_action,
            project: None,
            context: None,
            notes: Some("New notes".to_string()),
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };

        data.update("task-1", updated).unwrap();

        let found = data.find_by_id("task-1").unwrap();
        assert_eq!(found.title, "New Title");
        assert_eq!(found.status, NotaStatus::next_action);
        assert_eq!(found.notes, Some("New notes".to_string()));
        assert_eq!(data.next_action().len(), 1);
        assert_eq!(data.inbox().len(), 0);
    }
}
