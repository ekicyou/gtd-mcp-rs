# gtd-mcp

[![CI](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml)

**Version 0.7.2**

A Model Context Protocol (MCP) server for GTD (Getting Things Done) task management. This server enables LLM assistants like Claude to help you manage your tasks and projects using the proven GTD methodology.

## What is gtd-mcp?

gtd-mcp is an MCP server that implements the Getting Things Done (GTD) workflow. It provides a complete task management system that works seamlessly with LLM assistants through the Model Context Protocol.

**Key Features:**
- ✅ Complete GTD workflow support (inbox, next actions, waiting for, someday/maybe, calendar, done, trash)
- ✅ **Unified nota interface** - single set of tools for tasks, projects, and contexts
- ✅ Project and context management
- ✅ **Flexible task IDs** - client-provided arbitrary strings (e.g., "meeting-prep", "call-sarah")
- ✅ Batch operations for efficient task management
- ✅ TOML-based storage (human-readable, Git-friendly)
- ✅ Optional Git synchronization
- ✅ Built-in workflow prompts for LLM guidance

## Quick Start

### Installation

Install from crates.io:
```bash
cargo install gtd-mcp
```

Or build from source:
```bash
git clone https://github.com/ekicyou/gtd-mcp-rs.git
cd gtd-mcp-rs
cargo build --release
```

The binary will be at `target/release/gtd-mcp` (source build) or `~/.cargo/bin/gtd-mcp` (cargo install).

### Configuration

Add to your MCP client configuration (e.g., Claude Desktop's `claude_desktop_config.json`):

If installed via `cargo install`:
```json
{
  "mcpServers": {
    "gtd": {
      "command": "gtd-mcp",
      "args": ["gtd.toml"]
    }
  }
}
```

If built from source:
```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp",
      "args": ["gtd.toml"]
    }
  }
}
```

With Git synchronization:
```json
{
  "mcpServers": {
    "gtd": {
      "command": "gtd-mcp",
      "args": ["gtd.toml", "--sync-git"]
    }
  }
}
```

### Usage

Once configured, you can ask your LLM assistant to help you manage tasks using the unified nota interface:

- "Add a new task to review the project proposal"
- "Show me my next actions"
- "Update task meeting-prep and add notes"
- "Change status of call-sarah to done"
- "Create a project called website-redesign"
- "What's in my inbox?"
- "Help me process my inbox" (uses built-in GTD workflow prompt)

## MCP Tools

### Core Unified Tools (Recommended)

The system provides 5 core tools that handle all GTD operations in a unified way:

**add** - Capture any nota (task/project/context)
- Required: `id`, `title`, `status`
- Optional: `project`, `context`, `notes`, `start_date` (YYYY-MM-DD)
- Status determines type: inbox/next_action/etc→task, project→project, context→context

**list** - Review all notas with optional status filter
- Optional: `status` - Filter by specific status

**update** - Clarify and organize nota details
- Required: `id`
- Optional: `title`, `status`, `project`, `context`, `notes`, `start_date`
- Can transform types by changing status

**change_status** - Move notas through GTD workflow stages
- Required: `id`, `new_status`
- Supports all workflow transitions including type transformations

**empty_trash** - Permanently delete all trashed notas
- No parameters
- Irreversible operation with safety checks for referenced notas

### Legacy Tools (Backward Compatibility)

The following task-specific tools are maintained for compatibility but the unified tools above are recommended:

### Task Management

**add_task** - Add a new task to inbox (deprecated: use `add` with status="inbox")
- Required: `title`
- Optional: `project`, `context`, `notes`, `start_date` (YYYY-MM-DD)

**list_tasks** - List tasks with optional filters
- Optional: `status` (inbox, next_action, waiting_for, someday, later, done, trash, calendar)
- Optional: `date` (YYYY-MM-DD) - Filter tasks by start date
- Optional: `exclude_notes` (boolean) - Reduce token usage by excluding notes

**update_task** - Update an existing task
- Required: `task_id`
- Optional: `title`, `project`, `context`, `notes`, `start_date`
- Note: Use empty string to remove optional fields

### Status Management

**change_task_status** - Change status of one or more tasks in GTD workflow
- Required: `task_ids` (array like ["#1", "#2", "#3"]), `status` (target status)
- Optional: `start_date` (YYYY-MM-DD format, required for calendar status)
- Supports: inbox, next_action, waiting_for, someday, later, calendar, done, trash
- Batch operation: Move multiple tasks at once

Example (move to next_action):
```json
{
  "task_ids": ["#1", "#2", "#3"],
  "status": "next_action"
}
```

Example (move to calendar with date):
```json
{
  "task_ids": ["#5"],
  "status": "calendar",
  "start_date": "2024-12-25"
}
```

**empty_trash** - Permanently delete all trashed tasks (irreversible)

### Project Management

**add_project** - Create a new project
- Required: `name`, `id`
- Optional: `description`, `context`

**list_projects** - List all projects

**update_project** - Update an existing project
- Required: `project_id`
- Optional: `name`, `description`, `status` (active, on_hold, completed), `context`

**delete_project** - Delete a project
- Required: `project_id`
- Note: Cannot delete project if tasks reference it

### Context Management

**add_context** - Create a new context (e.g., @office, @home)
- Required: `name`
- Optional: `description`

**list_contexts** - List all contexts

**update_context** - Update a context
- Required: `name`
- Optional: `description`

**delete_context** - Delete a context
- Required: `name`
- Note: Cannot delete context if tasks or projects reference it

## MCP Prompts

The server includes built-in prompts to guide LLM assistants through GTD workflows:

- **gtd_overview** - Complete overview of the GTD system
- **process_inbox** - Step-by-step inbox processing guide
- **weekly_review** - GTD weekly review workflow
- **next_actions** - Guide for identifying and managing next actions
- **add_task_guide** - Best practices for creating tasks

## Data Storage

Tasks are stored in TOML format (default: `gtd.toml`). The format is human-readable and Git-friendly:

```toml
format_version = 2

[[inbox]]
id = "#1"
title = "Review project proposal"
project = "q1-marketing"
context = "Office"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[projects.q1-marketing]
name = "Q1 Marketing Campaign"
status = "active"

[contexts.Office]
description = "Work environment with desk and computer"
```

### Git Integration

Enable automatic Git synchronization with the `--sync-git` flag. The server will:
- Pull latest changes before loading
- Commit changes with descriptive messages
- Push to remote after saving

Setup:
```bash
git init
git config user.name "Your Name"
git config user.email "your@email.com"
git remote add origin https://github.com/yourusername/gtd-data.git
```

## Documentation

- **[IMPLEMENTATION.md](doc/IMPLEMENTATION.md)** - Technical implementation details and architecture
- **[GTD_ASSESSMENT.md](doc/GTD_ASSESSMENT.md)** - Feature assessment and enhancement roadmap
- **[RELEASE.md](RELEASE.md)** - Release notes for all versions (newest first)

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Code quality checks
cargo fmt --check
cargo clippy -- -D warnings
```

See [CI_SUMMARY.md](CI_SUMMARY.md) for CI/CD details.

## License

MIT License - see LICENSE file for details.

