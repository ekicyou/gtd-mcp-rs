# gtd-mcp-rs

[![CI](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml)

GTD MCP Server

A Model Context Protocol (MCP) server for GTD (Getting Things Done) task management.

**Version 0.1.0 - Cross-Platform Compatible**

This server now uses `mcp-attr` for better Windows compatibility. Previous versions used `rust-mcp-sdk` which had Linux-specific dependencies that prevented Windows builds.

## Features

- ✅ **Cross-Platform**: Works on Windows, Linux, and macOS
- ✅ **LLM-Friendly IDs**: Uses GitHub-style IDs (`#1`, `#2` for tasks, `project-1`, `project-2` for projects) for optimal readability and LLM interaction
- ✅ **MCP Prompts**: Built-in workflow guidance (GTD overview, inbox processing, weekly review, next actions, task creation best practices)
- Task management (inbox, next actions, waiting for, someday/maybe, done, trash, calendar)
- **Task and Project Updates**: Modify existing tasks and projects with full field update support
- **Trash management**: Move tasks to trash and bulk delete
- **Calendar management**: Tasks can have start dates for GTD tickler file workflow
- **Task timestamps**: All tasks include creation date (`created_at`) and update date (`updated_at`) for tracking task age and modifications
- **Referential integrity**: Validates that project and context references exist when creating or updating tasks
- Project management with status tracking (active, on_hold, completed)
- **Context management**: Full CRUD operations for GTD contexts (add, list, update, delete)
- TOML-based storage (gtd.toml)
- Git-friendly data format
- Declarative MCP server implementation with `mcp-attr`

**For a comprehensive feature assessment and enhancement roadmap, see [GTD_ASSESSMENT.md](GTD_ASSESSMENT.md)**

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Development

### Code Quality Checks

Before submitting a pull request, ensure your code passes all checks:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test
```

See [BRANCH_PROTECTION.md](BRANCH_PROTECTION.md) for details on CI/CD and branch protection setup.

## CI/CD

This project uses GitHub Actions for continuous integration:

- **Automated testing** on Linux, macOS, and Windows
- **Code quality checks** (formatting, linting)
- **Daily security audits** with cargo audit
- **Automated dependency updates** via Dependabot

See [CI_SUMMARY.md](CI_SUMMARY.md) for a complete overview of the CI/CD infrastructure.

## Usage

The server uses stdio transport and can be integrated with MCP clients. A file path must be specified:

```bash
cargo run -- gtd.toml
```

### Command-Line Options

The server requires a file path as the first argument and supports the following options:

**Arguments:**
- `<FILE>`: Path to the GTD data file (required)

**Options:**
- `--sync-git`: Enable automatic git synchronization on save

**Examples:**

```bash
# Use gtd.toml in current directory
cargo run -- gtd.toml

# Enable git sync with file
cargo run -- gtd.toml --sync-git

# Use custom file path
cargo run -- /path/to/my-gtd-data.toml

# Use custom file with git sync
cargo run -- /path/to/my-gtd-data.toml --sync-git
```

### Integration with MCP Clients

To use this server with an MCP client (like Claude Desktop or other MCP-compatible clients), add the following configuration:

For Claude Desktop, add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs",
      "args": ["gtd.toml"]
    }
  }
}
```

To enable automatic git synchronization:

```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs",
      "args": ["gtd.toml", "--sync-git"]
    }
  }
}
```

To use a custom file location:

```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs",
      "args": ["/path/to/your/gtd-data.toml"]
    }
  }
}
```

Or with both custom file and git sync:

```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs",
      "args": ["/path/to/your/gtd-data.toml", "--sync-git"]
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

**Automatic Fields:**
- `created_at` (date): Automatically set to current local date when task is created
- `updated_at` (date): Automatically set to current local date when task is created or modified

**Note:** If a project or context is specified, the server validates that it exists before creating the task. This ensures referential integrity in your GTD system.

**Example:**
```json
{
  "title": "Review project proposal",
  "project": "project-1",
  "context": "Office",
  "start_date": "2024-12-25"
}
```

**Note:** IDs are automatically generated as `#1`, `#2`, `#3` for tasks (GitHub issue tracker style) and `project-1`, `project-2` for projects, making them highly readable and easy to reference in conversations.

### list_tasks
List all tasks with optional status filter. Task listings include comprehensive information for each task.

**Parameters:**
- `status` (string, optional): Filter by status (inbox, next_action, waiting_for, someday, done, trash, calendar)

**Output Format:** Each task is displayed with:
- Task ID
- Task title
- Status
- Start date (if set)
- Project reference (if set)
- Context reference (if set)
- Creation date
- Last update date

**Example:**
```json
{
  "status": "inbox"
}
```

### update_task
Update an existing task. All parameters are optional except the task_id. Only provided fields will be updated.

**Parameters:**
- `task_id` (string, required): Task ID to update
- `title` (string, optional): New task title
- `project` (string, optional): New project ID (use empty string to remove)
- `context` (string, optional): New context name (use empty string to remove)
- `notes` (string, optional): New notes (use empty string to remove)
- `start_date` (string, optional): New start date in YYYY-MM-DD format (use empty string to remove)

**Automatic Updates:**
- `updated_at` (date): Automatically updated to current local date when task is modified

**Note:** 
- Project and context references are validated to ensure referential integrity.
- To change task status, use the specialized status movement methods below instead of this method.

**Example:**
```json
{
  "task_id": "#1",
  "title": "Updated title",
  "notes": "Updated notes"
}
```

### Status Movement Methods

These methods provide explicit, intuitive ways to move tasks between different status states. All methods automatically update the `updated_at` timestamp.

#### inbox_task
Move a task to inbox.

**Parameters:**
- `task_id` (string, required): Task ID to move to inbox

**Example:**
```json
{
  "task_id": "#1"
}
```

#### next_action_task
Move a task to next action.

**Parameters:**
- `task_id` (string, required): Task ID to move to next action

**Example:**
```json
{
  "task_id": "#1"
}
```

#### waiting_for_task
Move a task to waiting for.

**Parameters:**
- `task_id` (string, required): Task ID to move to waiting for

**Example:**
```json
{
  "task_id": "#1"
}
```

#### someday_task
Move a task to someday.

**Parameters:**
- `task_id` (string, required): Task ID to move to someday

**Example:**
```json
{
  "task_id": "#1"
}
```

#### done_task
Move a task to done.

**Parameters:**
- `task_id` (string, required): Task ID to move to done

**Example:**
```json
{
  "task_id": "#1"
}
```

#### calendar_task
Move a task to calendar (GTD tickler file concept).

In GTD, moving a task to the calendar means you're deferring it until a specific date - you "forget about it" until that date arrives. This tool requires that the task has a `start_date` set.

**Parameters:**
- `task_id` (string, required): Task ID to move to calendar
- `start_date` (string, optional): Start date in YYYY-MM-DD format. If not provided, the task must already have a `start_date` set.

**Validation:**
- A task must have a `start_date` to be moved to calendar status
- If the task doesn't have a `start_date` and you don't provide one, an error will be returned
- If both the task has a `start_date` and you provide a new one, the new date will override the existing one

**Example (setting new start date):**
```json
{
  "task_id": "#1",
  "start_date": "2024-12-25"
}
```

**Example (using existing start date):**
```json
{
  "task_id": "#1"
}
```

**Note:** The calendar status represents tasks that are scheduled to start on a specific date. Setting `start_date` on a task doesn't automatically move it to calendar - you must explicitly use this tool. However, when a task has calendar status, it must have a valid `start_date`.

### trash_task
Move a task to trash.

**Parameters:**
- `task_id` (string, required): Task ID to move to trash

**Example:**
```json
{
  "task_id": "#1"
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
- `context` (string, optional): Context name (must exist if specified)

**Note:** If a context is specified, the server validates that it exists before creating the project. This ensures referential integrity in your GTD system.

**Example:**
```json
{
  "name": "Q1 Marketing Campaign",
  "description": "Launch new product marketing campaign",
  "context": "Office"
}
```

### list_projects
List all projects with their status, description, and context information.

**Parameters:** None

**Output Format:** Each project is displayed with:
- Project ID
- Project name
- Status (active, on_hold, completed)
- Description (if set)
- Context (if set)

### update_project
Update an existing project. All parameters are optional except the project_id. Only provided fields will be updated.

**Parameters:**
- `project_id` (string, required): Project ID to update
- `name` (string, optional): New project name
- `description` (string, optional): New description (use empty string to remove)
- `status` (string, optional): New status (active, on_hold, completed)
- `context` (string, optional): New context name (use empty string to remove)

**Note:** Context references are validated to ensure referential integrity.

**Example:**
```json
{
  "project_id": "project-1",
  "status": "completed",
  "description": "Successfully launched Q1 campaign",
  "context": "Office"
}
```

### add_context
Add a new context.

**Parameters:**
- `name` (string, required): Context name
- `description` (string, optional): Context description

**Example:**
```json
{
  "name": "Office",
  "description": "Work environment with desk and computer"
}
```

### list_contexts
List all contexts alphabetically sorted.

**Parameters:** None

**Example:**
```json
{}
```

### update_context
Update an existing context's description.

**Parameters:**
- `name` (string, required): Context name
- `description` (string, optional): New description (use empty string to remove)

**Example:**
```json
{
  "name": "Office",
  "description": "Updated description"
}
```

### delete_context
Delete a context from the system.

**Parameters:**
- `name` (string, required): Context name to delete

**Example:**
```json
{
  "name": "Office"
}
```

## MCP Prompts

This server provides several prompts to guide LLMs in using the GTD system effectively. Prompts offer workflow guidance and best practices.

### Available Prompts

#### gtd_overview
Comprehensive overview of the GTD system, including:
- Core concepts (task statuses, projects, contexts)
- Task ID format (#1, #2, project-1, project-2)
- Common workflows (Capture, Process, Review, Do)
- Available tools summary

Use this prompt to get oriented with the system or refresh your understanding of GTD principles.

#### process_inbox
Step-by-step guide for processing inbox items following GTD methodology:
- Is it actionable? (no → someday/trash)
- Less than 2 minutes? (yes → do it now)
- Can you do it yourself? (no → waiting_for)
- Specific date? (yes → calendar)
- Part of project? (assign project)
- Add context and move to next_action

Goal: Process inbox to zero with every item clarified and organized.

#### weekly_review
Complete GTD weekly review process:
- **Get Clear**: Process inbox, empty your head
- **Get Current**: Review calendar, next actions, waiting for, someday tasks
- **Review Projects**: Ensure each has next action, update status
- **Get Creative**: Brainstorm new possibilities

Conduct weekly to maintain system integrity (recommended every 7 days).

#### next_actions
Guide for identifying and managing next actions:
- Characteristics of good next actions (specific, physical, doable, single-step)
- Context-based work (@office, @computer, @phone, @home, @errands)
- Choosing what to do (consider context, time, energy, priority)
- Post-completion steps

#### add_task_guide
Best practices for creating well-formed tasks:
- Good vs. poor task title examples
- When to use optional fields (project, context, notes, start_date)
- Recommended workflow (quick capture → process → add details)

### Using Prompts

Prompts are designed to be concise and token-efficient while providing comprehensive guidance. They help LLMs understand:
- How to use the GTD system effectively
- Best practices for task and project management
- Recommended workflows for common scenarios

## Data Storage

Tasks and projects are stored in the GTD data file specified at startup. This file can be version controlled with git for backup and synchronization.

### Example gtd.toml

```toml
[[inbox]]
id = "#1"
title = "Review project proposal"
project = "project-1"
context = "Office"
start_date = "2024-12-25"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[projects]]
id = "project-1"
name = "Q1 Marketing Campaign"
description = "Launch new product marketing campaign"
status = "active"

[contexts.Office]
description = "Work environment with desk and computer"

task_counter = 1
project_counter = 1
```

## Git Integration

The GTD MCP Server includes automatic git synchronization using the git2 crate:

### Automatic Sync

When the `--sync-git` flag is enabled and the data file is located in a git-managed directory, the server automatically:
1. **Pulls** the latest changes from the remote before loading data
2. **Commits** the updated file with descriptive messages (e.g., "Add task to inbox: Task title", "Mark task #1 as done")
3. **Pushes** the changes to the remote repository after saving

This ensures your GTD data is always synchronized across devices without manual intervention.

**Note:** Git operations now properly propagate errors to the MCP client. If a git operation fails (e.g., no internet connection, merge conflicts, missing remote), the error will be returned to the client rather than being silently ignored.

### Setup

To enable git synchronization, first set up your git repository:

```bash
# Initialize git repo (if not already done)
git init

# Configure git user
git config user.name "Your Name"
git config user.email "your.email@example.com"

# Add remote repository
git remote add origin https://github.com/yourusername/gtd-data.git

# Create initial commit
git add your-data-file.toml
git commit -m "Initial GTD data"
git push -u origin main
```

Then start the server with the `--sync-git` flag to enable automatic synchronization (see the Integration with MCP Clients section above for configuration examples).

### Error Handling

Git operations now properly return errors to the MCP client:
- If the git repository is not configured correctly (e.g., missing remote, invalid credentials)
- If there are network issues preventing pull/push operations
- If there are merge conflicts

When git synchronization fails, the error message will be returned to the MCP client, allowing users to take appropriate action. The data is still written to the local file before git operations are attempted.

## License

MIT License - see LICENSE file for details.

