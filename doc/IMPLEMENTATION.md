# GTD MCP Server Implementation

## Overview

This is a basic implementation of a GTD (Getting Things Done) MCP (Model Context Protocol) server in Rust. The server provides task and project management capabilities through MCP tools.

**Version 0.7.1**

This version uses `mcp-attr` v0.0.7 for declarative server building:

- ✅ Uses declarative server building with `#[mcp_server]` and `#[tool]` macros
- ✅ Automatic JSON Schema generation from function signatures
- ✅ Full MCP 2025-03-26 protocol support

## Architecture

### Components

1. **Data Structures** (`src/gtd.rs`)
   - `Task`: Represents a GTD task with status (inbox, next_action, waiting_for, someday, done, trash) and optional start date for calendar management
   - `Project`: Represents a project with status (active, on_hold, completed)
   - `Context`: Represents a context (e.g., @office, @home)
   - `GtdData`: Container for all tasks, projects, and contexts

2. **Storage** (`src/storage.rs`)
   - TOML-based serialization and deserialization
   - Saves data to `gtd.toml` file
   - Git-friendly format for version control
   - Integrates with git operations for automatic synchronization
   - **Format Versioning**: Automatic migration from old formats

3. **Git Operations** (`src/git_ops.rs`)
   - Automatic git repository detection
   - Pull, commit, and push operations using git2 crate
   - Thread-safe implementation with `Arc<Mutex<Repository>>`
   - Graceful degradation when git operations fail

4. **MCP Server** (`src/main.rs`)
   - Uses `mcp-attr` v0.0.7 with declarative server building
   - Implements `McpServer` trait using `#[mcp_server]` macro
   - Provides stdio transport for MCP communication
   - Uses `#[tool]` attributes for tool registration

### TOML Format Versions

The server uses a format version system to enable backwards-compatible changes to the data structure:

- **Version 1** (Legacy): Projects stored as array `[[projects]]`
  ```toml
  [[projects]]
  id = "project-1"
  name = "My Project"
  ```

- **Version 2** (Current): Projects stored as HashMap `[projects.id]`
  ```toml
  format_version = 2
  
  [projects.project-1]
  name = "My Project"
  ```

**Automatic Migration**: When loading a version 1 file, the server automatically migrates it to version 2 format. On the next save, the file will be written in version 2 format. This ensures backwards compatibility while allowing the data structure to evolve.

## MCP Tools

The server exposes the following tools:

### Task Management

#### add_task
Adds a new task to the inbox.

**Parameters:**
- `title` (required): Task title
- `project` (optional): Project ID (must exist if specified)
- `context` (optional): Context name (must exist if specified)
- `notes` (optional): Additional notes
- `start_date` (optional): Start date in YYYY-MM-DD format for GTD tickler file workflow

**Automatic Fields:**
- `created_at` (date): Automatically set to current local date when task is created
- `updated_at` (date): Automatically set to current local date when task is created or modified

**Referential Integrity:** If a project or context is specified, the server validates that it exists before creating the task.

#### list_tasks
Lists all tasks with optional status filtering.

**Parameters:**
- `status` (optional): Filter by status (inbox, next_action, waiting_for, someday, later, done, trash, calendar)
- `date` (optional): Filter by date in YYYY-MM-DD format. Tasks with start_date in the future (later than the specified date) are excluded.
- `exclude_notes` (optional): Set to `true` to exclude notes from output and reduce token usage. Default is `false`.

**Output Format:** Each task is displayed with:
- Task ID
- Task title
- Status
- Start date (if set)
- Project reference (if set)
- Context reference (if set)
- Notes (if set and not excluded)
- Creation date
- Last update date

#### update_task
Updates an existing task. All parameters are optional except the task_id. Only provided fields will be updated.

**Parameters:**
- `task_id` (required): Task ID to update
- `title` (optional): New task title
- `project` (optional): New project ID (use empty string to remove)
- `context` (optional): New context name (use empty string to remove)
- `notes` (optional): New notes (use empty string to remove)
- `start_date` (optional): New start date in YYYY-MM-DD format (use empty string to remove)

**Automatic Updates:**
- `updated_at` (date): Automatically updated to current local date when task is modified

**Note:** 
- Project and context references are validated to ensure referential integrity.
- To change task status, use the specialized status movement methods instead of this method.

### Status Movement Methods

All status movement methods support batch operations (moving multiple tasks at once) and automatically update the `updated_at` timestamp.

#### trash_tasks
Moves one or more tasks to trash.

**Parameters:**
- `task_ids` (required): Array of task IDs to move to trash

**Example:**
```json
{
  "task_ids": ["#1", "#3", "#5"]
}
```

#### inbox_tasks
Move one or more tasks to inbox.

**Parameters:**
- `task_ids` (required): Array of task IDs to move to inbox

#### next_action_tasks
Move one or more tasks to next action.

**Parameters:**
- `task_ids` (required): Array of task IDs to move to next action

#### waiting_for_tasks
Move one or more tasks to waiting for.

**Parameters:**
- `task_ids` (required): Array of task IDs to move to waiting for

#### someday_tasks
Move one or more tasks to someday/maybe.

**Parameters:**
- `task_ids` (required): Array of task IDs to move to someday

#### later_tasks
Move one or more tasks to later (deferred but not someday).

**Parameters:**
- `task_ids` (required): Array of task IDs to move to later

#### done_tasks
Move one or more tasks to done.

**Parameters:**
- `task_ids` (required): Array of task IDs to move to done

#### calendar_tasks
Move one or more tasks to calendar (GTD tickler file).

**Parameters:**
- `task_ids` (required): Array of task IDs to move
- `start_date` (optional): Start date in YYYY-MM-DD format. If provided, all tasks will have their start_date set to this value. If not provided, each task must already have a `start_date`.

**Validation:**
- Each task must have a `start_date` to be moved to calendar status
- If a task doesn't have a `start_date` and you don't provide one, that task will fail to move (but others may succeed)
- If you provide a `start_date`, it will be applied to all tasks

**Example (setting new start date for all tasks):**
```json
{
  "task_ids": ["#1", "#2"],
  "start_date": "2024-12-25"
}
```

**Example (using existing start dates):**
```json
{
  "task_ids": ["#1", "#2"]
}
```

### empty_trash
Permanently deletes all trashed tasks.

**Parameters:** None

### Project Management

#### add_project
Creates a new project.

**Parameters:**
- `name` (required): Project name
- `description` (optional): Project description
- `context` (optional): Context name (must exist if specified)
- `id` (optional): Custom project ID (auto-generated if not specified)

**Referential Integrity:** 
- If a context is specified, the server validates that it exists before creating the project.
- If a custom ID is specified, the server validates that it doesn't already exist.

**Auto-generated IDs:** If no custom ID is provided, the server generates sequential IDs in the format `project-1`, `project-2`, etc.

#### list_projects
Lists all projects with their status, description, and context information.

**Parameters:** None

**Output Format:** Each project is displayed with:
- Project ID
- Project name
- Status (active, on_hold, completed)
- Description (if set)
- Context (if set)

#### update_project
Updates an existing project. All parameters are optional except the project_id. Only provided fields will be updated.

**Parameters:**
- `project_id` (required): Project ID to update
- `id` (optional): New project ID
- `name` (optional): New project name
- `description` (optional): New description (empty string to remove)
- `status` (optional): New status (active, on_hold, completed)
- `context` (optional): New context name (empty string to remove)

**Note:** 
- Context references are validated to ensure referential integrity.
- If a new project ID is specified, the server validates that it doesn't conflict with existing projects.
- When a project ID is changed, all task references to the old project ID are automatically updated to the new ID.

### Context Management

#### add_context
Creates a new context.

**Parameters:**
- `name` (required): Context name
- `description` (optional): Context description

#### list_contexts
Lists all contexts alphabetically.

**Parameters:** None

#### update_context
Updates an existing context's description.

**Parameters:**
- `name` (required): Context name
- `description` (optional): New description (empty string to remove)

#### delete_context
Deletes a context from the system.

**Parameters:**
- `name` (required): Context name to delete

## MCP Prompts

The server provides several prompts to guide LLMs in using the GTD system effectively:

### gtd_overview
Comprehensive overview of the GTD system, including:
- Core concepts (task statuses, projects, contexts)
- Task ID format (#1, #2, project-1, project-2)
- Common workflows (Capture, Process, Review, Do)
- Available tools summary

### process_inbox
Step-by-step guide for processing inbox items following GTD methodology:
- Is it actionable? (no → someday/trash)
- Less than 2 minutes? (yes → do it now)
- Can you do it yourself? (no → waiting_for)
- Specific date? (yes → calendar)
- Should this be done later? (yes → later)
- Part of project? (assign project)
- Add context and move to next_action

Goal: Process inbox to zero with every item clarified and organized.

### weekly_review
Complete GTD weekly review process:
- **Get Clear**: Process inbox, empty your head
- **Get Current**: Review calendar, next actions, waiting for, later, someday tasks
- **Review Projects**: Ensure each has next action, update status
- **Get Creative**: Brainstorm new possibilities

### next_actions
Guide for identifying and managing next actions:
- Characteristics of good next actions (specific, physical, doable, single-step)
- Context-based work (@office, @computer, @phone, @home, @errands)
- Choosing what to do (consider context, time, energy, priority)
- Post-completion steps

### add_task_guide
Best practices for creating well-formed tasks:
- Good vs. poor task title examples
- When to use optional fields (project, context, notes, start_date)
- Recommended workflow (quick capture → process → add details)

## Data Storage Format

Data is stored in TOML format in `gtd.toml`:

```toml
format_version = 2

[[inbox]]
id = "#1"
title = "Review project proposal"
project = "project-1"
context = "Office"
start_date = "2024-12-25"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[next_action]]
id = "#2"
title = "Complete documentation"
notes = "Review all sections and update examples"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[projects.project-1]
name = "Q1 Marketing Campaign"
description = "Launch new product marketing campaign"
status = "active"

[contexts.Office]
description = "Work environment with desk and computer"

task_counter = 2
project_counter = 1
```

### Line Ending Handling

The server handles line endings consistently:

- **Serialization (saving to file)**: TOML files are written with OS-native line endings
  - Windows: CRLF (`\r\n`)
  - Linux/macOS: LF (`\n`)
  - This ensures files are readable in standard text editors on each platform

- **Deserialization (loading from file)**: Line endings are normalized to LF (`\n`) internally
  - All line ending styles (CRLF, CR, LF) are accepted when reading files
  - This ensures consistent behavior
  - Allows files created with different line endings to be read correctly

- **MCP Communication**: JSON-RPC protocol uses LF (`\n`) for newlines in string fields
  - Task notes and other multi-line fields use `\n` in MCP tool calls
  - The server automatically handles conversion to/from OS-native format

This design ensures that:
- Files are readable and Git-friendly
- Multi-line content (like task notes) is handled consistently

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Running

The server uses stdio transport and communicates via standard input/output:

```bash
cargo run
```

Or with the release build:

```bash
./target/release/gtd-mcp
```

## Dependencies

- `mcp-attr` (0.0.7): MCP protocol implementation with declarative server building (provides `schemars` 0.8 for JSON Schema generation)
- `tokio` (1.x): Async runtime
- `serde` (1.x): Serialization framework
- `toml` (0.9): TOML parsing and generation
- `anyhow` (1.x): Error handling
- `chrono` (0.4): Date and time handling for task start dates
- `git2` (0.20): Git operations for automatic version control
- `clap` (4.x): Command-line argument parsing

## LLM-Friendly ID Generation

The server uses sequential counter-based IDs instead of UUIDs for better LLM interaction:

- **Task IDs**: `#1`, `#2`, `#3`, ... (2-3 characters, GitHub issue tracker style)
- **Project IDs**: `project-1`, `project-2`, `project-3`, ... (9-11 characters, descriptive format)

### Benefits:
- ✅ **94%+ reduction** in character count for task IDs compared to UUIDs (36 chars → 2-3 chars)
- ✅ **GitHub issue tracker style** for tasks (`#1`, `#2`, `#42`) - instantly familiar
- ✅ **Descriptive project IDs** (`project-1`) - more readable as projects are fewer
- ✅ **Human-readable** and easy to remember (`#42` vs `d8f5f3c1-7e4d-4b2a-9f8e-1c2d3e4f5a6b`)
- ✅ **LLM-friendly** - easier for language models to reference and recall
- ✅ **Lower token cost** when transmitting task lists to LLMs
- ✅ **Persistent counters** stored in `gtd.toml` ensure uniqueness across sessions

Example output:
```
Old: - [d8f5f3c1-7e4d-4b2a-9f8e-1c2d3e4f5a6b] Complete documentation
New: - [#1] Complete documentation
```

## Future Enhancements

This is a basic implementation. Potential enhancements include:

1. ~~Context management tools (add_context, list_contexts)~~ ✅ **COMPLETED** - Full context CRUD operations implemented
2. ~~Task update and deletion~~ ✅ Trash management implemented (move to trash, empty trash)
3. Project completion tracking
4. Task dependencies
5. ~~Due dates and reminders~~ ✅ Start dates implemented for GTD tickler file workflow (Due dates - see GTD_ASSESSMENT.md)
6. Tags and labels
7. Search and filtering capabilities (basic status filtering exists)
8. Backup and restore functionality
9. Multiple GTD workflow support

**For a comprehensive feature assessment and roadmap, see [GTD_ASSESSMENT.md](GTD_ASSESSMENT.md)**

## Git Integration

The `gtd.toml` file features automatic git synchronization using the git2 crate:

### Automatic Version Control

When `gtd.toml` is in a git-managed directory and `--sync-git` flag is enabled, the server automatically:
1. **Pulls** latest changes from remote before loading data
2. **Commits** changes with descriptive messages based on the operation (e.g., "Add task to inbox: Task title", "Mark task #1 as done", "Add project: Project name")
3. **Pushes** to remote repository after successful save

This provides:
- Automatic version control for all task and project changes
- Cross-device synchronization without manual intervention
- Complete history of all GTD data modifications with meaningful commit messages
- Safe concurrent access from multiple devices

### Implementation Details

The git integration is implemented in `src/git_ops.rs`:
- Thread-safe using `Arc<Mutex<Repository>>` for async compatibility
- **Error propagation**: Git operation failures are now properly returned to the MCP client instead of being silently ignored
- Only activates when storage file is in a git repository and `--sync-git` is enabled
- Uses configured `user.name` and `user.email`, falls back to defaults
- File is added to git index automatically via `commit()` method

### Setup

Simply place `gtd.toml` in a git repository with a configured remote:

```bash
git init
git config user.name "Your Name"
git config user.email "your.email@example.com"
git remote add origin https://github.com/yourusername/gtd-data.git
git add gtd.toml
git commit -m "Initial GTD data"
git push -u origin main
```

After setup, all updates are automatically synchronized.
