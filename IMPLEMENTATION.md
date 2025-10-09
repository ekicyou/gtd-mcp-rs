# GTD MCP Server Implementation

## Overview

This is a basic implementation of a GTD (Getting Things Done) MCP (Model Context Protocol) server in Rust. The server provides task and project management capabilities through MCP tools.

**Version 0.1.0 - Windows Compatible Release**

This version uses `mcp-attr` v0.0.7, which provides better cross-platform compatibility compared to the previous `rust-mcp-sdk`. The new implementation:

- ✅ Works on Windows without Linux-specific dependencies
- ✅ Uses declarative server building with `#[mcp_server]` and `#[tool]` macros
- ✅ Simpler, more maintainable code with fewer lines
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

3. **MCP Server** (`src/main.rs`)
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

## Data Storage Format

Data is stored in TOML format in `gtd.toml`:

```toml
[tasks]

[tasks."task-id"]
id = "task-id"
title = "Task Title"
status = "Inbox"
project = "project-id"
context = "context-id"
notes = "Some notes"
start_date = "2024-12-25"

[projects]

[projects."project-id"]
id = "project-id"
name = "Project Name"
description = "Project Description"
status = "Active"

[contexts]

[contexts."context-id"]
id = "context-id"
name = "Context Name"
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
./target/release/gtd-mcp-rs
```

## Dependencies

- `mcp-attr` (0.0.7): MCP protocol implementation with declarative server building (provides `schemars` 0.8 for JSON Schema generation)
- `tokio` (1.x): Async runtime
- `serde` (1.x): Serialization framework
- `toml` (0.9): TOML parsing and generation
- `anyhow` (1.x): Error handling
- `uuid` (1.x): Unique ID generation (kept for backward compatibility, but no longer used)
- `chrono` (0.4): Date and time handling for task start dates

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

1. Context management tools (add_context, list_contexts)
2. ~~Task update and deletion~~ ✅ Trash management implemented (move to trash, empty trash)
3. Project completion tracking
4. Task dependencies
5. ~~Due dates and reminders~~ ✅ Start dates implemented for GTD tickler file workflow
6. Tags and labels
7. Search and filtering capabilities
8. Backup and restore functionality
9. Multiple GTD workflow support

## Git Integration

The `gtd.toml` file is git-friendly and can be:
- Version controlled in a git repository
- Synchronized across devices using git push/pull
- Branched for different workflows
- Merged when needed

Add `gtd.toml` to your git repository to enable synchronization:

```bash
git add gtd.toml
git commit -m "Update tasks and projects"
git push
```
