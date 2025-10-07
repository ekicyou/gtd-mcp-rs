# gtd-mcp-rs
GTD MCP Server

A Model Context Protocol (MCP) server for GTD (Getting Things Done) task management.

## Features

- Task management (inbox, next actions, waiting for, someday/maybe, done)
- Project management
- Context management
- TOML-based storage (gtd.toml)
- Git-friendly data format

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
- `project` (string, optional): Project ID
- `context` (string, optional): Context ID
- `notes` (string, optional): Additional notes

**Example:**
```json
{
  "title": "Review project proposal",
  "project": "proj-123",
  "context": "ctx-office"
}
```

### list_tasks
List all tasks with optional status filter.

**Parameters:**
- `status` (string, optional): Filter by status (Inbox, NextAction, WaitingFor, Someday, Done)

**Example:**
```json
{
  "status": "Inbox"
}
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
[tasks]

[tasks."abc-123"]
id = "abc-123"
title = "Review project proposal"
status = "Inbox"

[projects]

[projects."proj-456"]
id = "proj-456"
name = "Q1 Marketing Campaign"
status = "Active"

[contexts]
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

