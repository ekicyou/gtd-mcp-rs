# gtd-mcp-rs
GTD MCP Server

A Model Context Protocol (MCP) server for GTD (Getting Things Done) task management.

**Version 0.1.0 - Cross-Platform Compatible**

This server now uses `mcp-attr` for better Windows compatibility. Previous versions used `rust-mcp-sdk` which had Linux-specific dependencies that prevented Windows builds.

## Features

- ✅ **Cross-Platform**: Works on Windows, Linux, and macOS
- Task management (inbox, next actions, waiting for, someday/maybe, done, trash)
- **Trash management**: Move tasks to trash and bulk delete
- **Calendar management**: Tasks can have start dates for GTD tickler file workflow
- **Referential integrity**: Validates that project and context references exist when creating tasks
- Project management
- Context management
- TOML-based storage (gtd.toml)
- Git-friendly data format
- Declarative MCP server implementation with `mcp-attr`

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Usage

The server uses stdio transport and can be integrated with MCP clients:

```bash
cargo run
```

### Integration with MCP Clients

To use this server with an MCP client (like Claude Desktop or other MCP-compatible clients), add the following configuration:

For Claude Desktop, add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs"
    }
  }
}
```

## MCP Tools

The server provides the following tools:

### add_task
Add a new task to the inbox.

**Parameters:**
- `title` (string, required): Task title
- `project` (string, optional): Project ID (must exist if specified)
- `context` (string, optional): Context name (must exist if specified)
- `notes` (string, optional): Additional notes
- `start_date` (string, optional): Start date in YYYY-MM-DD format (for GTD tickler file)

**Note:** If a project or context is specified, the server validates that it exists before creating the task. This ensures referential integrity in your GTD system.

**Example:**
```json
{
  "title": "Review project proposal",
  "project": "proj-123",
  "context": "Office",
  "start_date": "2024-12-25"
}
```

### list_tasks
List all tasks with optional status filter.

**Parameters:**
- `status` (string, optional): Filter by status (inbox, next_action, waiting_for, someday, done, trash)

**Example:**
```json
{
  "status": "inbox"
}
```

### trash_task
Move a task to trash.

**Parameters:**
- `task_id` (string, required): Task ID to move to trash

**Example:**
```json
{
  "task_id": "abc-123"
}
```

### empty_trash
Permanently delete all trashed tasks.

**Parameters:** None

**Example:**
```json
{}
```

### add_project
Add a new project.

**Parameters:**
- `name` (string, required): Project name
- `description` (string, optional): Project description

**Example:**
```json
{
  "name": "Q1 Marketing Campaign",
  "description": "Launch new product marketing campaign"
}
```

### list_projects
List all projects.

**Parameters:** None

## Data Storage

Tasks and projects are stored in `gtd.toml` in the current directory. This file can be version controlled with git for backup and synchronization.

### Example gtd.toml

```toml
[[tasks]]
id = "task-001"
title = "Review project proposal"
status = "inbox"
project = "proj-456"
context = "Office"
start_date = "2024-12-25"

[[projects]]
id = "proj-456"
name = "Q1 Marketing Campaign"
description = "Launch new product marketing campaign"
status = "active"

[contexts.Office]
name = "Office"
description = "Work environment with desk and computer"
```

## Git Integration

The TOML storage format is designed to work well with git:

```bash
# Initialize git repo (if not already done)
git init

# Add and commit your GTD data
git add gtd.toml
git commit -m "Update tasks"

# Sync across devices
git push
git pull
```

## License

MIT License - see LICENSE file for details.

