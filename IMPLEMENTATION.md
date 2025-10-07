# GTD MCP Server Implementation

## Overview

This is a basic implementation of a GTD (Getting Things Done) MCP (Model Context Protocol) server in Rust. The server provides task and project management capabilities through MCP tools.

## Architecture

### Components

1. **Data Structures** (`src/gtd.rs`)
   - `Task`: Represents a GTD task with status (Inbox, NextAction, WaitingFor, Someday, Done)
   - `Project`: Represents a project with status (Active, OnHold, Completed)
   - `Context`: Represents a context (e.g., @office, @home)
   - `GtdData`: Container for all tasks, projects, and contexts

2. **Storage** (`src/storage.rs`)
   - TOML-based serialization and deserialization
   - Saves data to `gtd.toml` file
   - Git-friendly format for version control

3. **MCP Server** (`src/main.rs`)
   - Uses `rust-mcp-sdk` v0.7.0
   - Implements `ServerHandler` trait
   - Provides stdio transport for MCP communication

## MCP Tools

The server exposes the following tools:

### add_task
Adds a new task to the inbox.

**Parameters:**
- `title` (required): Task title
- `project` (optional): Project ID
- `context` (optional): Context ID
- `notes` (optional): Additional notes

### list_tasks
Lists all tasks with optional status filtering.

**Parameters:**
- `status` (optional): Filter by status (Inbox, NextAction, WaitingFor, Someday, Done)

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

- `rust-mcp-sdk` (0.7.0): MCP protocol implementation
- `tokio` (1.x): Async runtime
- `serde` (1.x): Serialization framework
- `toml` (0.8): TOML parsing and generation
- `anyhow` (1.x): Error handling
- `uuid` (1.x): Unique ID generation
- `async-trait` (0.1): Async trait support

## Future Enhancements

This is a basic implementation. Potential enhancements include:

1. Context management tools (add_context, list_contexts)
2. Task update and deletion
3. Project completion tracking
4. Task dependencies
5. Due dates and reminders
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
