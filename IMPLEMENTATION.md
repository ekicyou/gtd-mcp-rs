# GTD MCP Server Implementation

## Overview

This is a basic implementation of a GTD (Getting Things Done) MCP (Model Context Protocol) server in Rust. The server provides task and project management capabilities through MCP tools.

**Version 0.3.0 - Cross-Platform Compatible**

This version uses `mcp-attr` v0.0.7 for declarative server building:

- ✅ Works on Windows, Linux, and macOS
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

## MCP Tools

The server exposes the following tools:

### add_task
Adds a new task to the inbox.

**Parameters:**
- `title` (required): Task title
- `project` (optional): Project ID
- `context` (optional): Context ID
- `notes` (optional): Additional notes
- `start_date` (optional): Start date in YYYY-MM-DD format for GTD tickler file workflow

### list_tasks
Lists all tasks with optional status filtering.

**Parameters:**
- `status` (optional): Filter by status (inbox, next_action, waiting_for, someday, done, trash)

### trash_task
Moves a task to trash.

**Parameters:**
- `task_id` (required): Task ID to move to trash

### empty_trash
Permanently deletes all trashed tasks.

**Parameters:** None

### add_project
Creates a new project.

**Parameters:**
- `name` (required): Project name
- `description` (optional): Project description

### list_projects
Lists all projects.

**Parameters:** None

### update_project
Updates an existing project.

**Parameters:**
- `project_id` (required): Project ID to update
- `name` (optional): New project name
- `description` (optional): New description (empty string to remove)
- `status` (optional): New status (active, on_hold, completed)

### add_context
Creates a new context.

**Parameters:**
- `name` (required): Context name
- `description` (optional): Context description

### list_contexts
Lists all contexts alphabetically.

**Parameters:** None

### update_context
Updates an existing context's description.

**Parameters:**
- `name` (required): Context name
- `description` (optional): New description (empty string to remove)

### delete_context
Deletes a context from the system.

**Parameters:**
- `name` (required): Context name to delete

## Data Storage Format

Data is stored in TOML format in `gtd.toml`:

```toml
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

[[projects]]
id = "project-1"
name = "Q1 Marketing Campaign"
description = "Launch new product marketing campaign"
status = "active"

[contexts.Office]
description = "Work environment with desk and computer"

task_counter = 2
project_counter = 1
```

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
- `git2` (0.19): Git operations for automatic version control
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
