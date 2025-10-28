# gtd-mcp

[![CI](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ekicyou/gtd-mcp-rs/actions/workflows/ci.yml)

**Version 0.8.0** | [日本語版 README](README.ja-jp.md)

A Model Context Protocol (MCP) server for GTD (Getting Things Done) task management. This server enables LLM assistants like Claude to help you manage your tasks and projects using the proven GTD methodology.

## What is gtd-mcp?

gtd-mcp is an MCP server that implements the Getting Things Done (GTD) workflow. It provides a complete task management system that works seamlessly with LLM assistants through the Model Context Protocol.

**Key Features:**
- ✅ Complete GTD workflow support (inbox, next actions, waiting for, someday/maybe, calendar, done, reference material, trash)
- ✅ **Unified nota interface** - single set of tools for tasks, projects, and contexts
- ✅ Project and context management
- ✅ **Flexible task IDs** - client-provided arbitrary strings (e.g., "meeting-prep", "call-sarah")
- ✅ Batch operations for efficient task management
- ✅ TOML-based storage (human-readable, Git-friendly)
- ✅ Optional Git synchronization

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
- "Help me process my inbox"

## MCP Tools

The system provides 5 unified tools that handle all GTD operations:

### Capture and Review

**inbox** - Capture anything that needs attention (GTD Capture step)
- Required: `id` (any string, e.g., "call-john", "website-redesign"), `title`, `status`
- Optional: `project`, `context`, `notes`, `start_date` (YYYY-MM-DD)
- Status determines type: inbox/next_action/etc→task, project→project, context→context
- Use this as the first step in GTD workflow - quickly capture everything to process later

**list** - Review all notas with optional filters (GTD Review step)
- Optional: `status` - Filter by specific status (inbox, next_action, waiting_for, later, calendar, someday, done, reference, trash, project, context)
- Optional: `date` (YYYY-MM-DD) - For calendar status, shows tasks with start_date <= this date
- Optional: `exclude_notes` (boolean) - Reduce token usage by excluding notes
- Review regularly (daily/weekly) to keep your system current

## GTD Status Categories

The system supports the following status categories according to GTD methodology:

### Actionable Items
- **inbox**: Unprocessed items that need to be clarified (start here)
- **next_action**: Ready-to-execute tasks that require your attention
- **waiting_for**: Items blocked on someone else or an external event
- **later**: Deferred tasks that you'll do eventually
- **calendar**: Date-specific or time-specific tasks
- **someday**: Potential future actions (not committed yet)

### Non-Actionable Items
- **reference**: Non-actionable information saved for future reference - important documents, notes, or information you might need later but don't require action
- **done**: Completed tasks (for record-keeping and review)
- **trash**: Discarded items (can be permanently deleted with empty_trash)

### Organizational Structures
- **project**: Multi-step outcomes requiring multiple actions
- **context**: Environments, tools, or situations where actions can be performed (e.g., @office, @home, @computer)

### Organize and Execute

**update** - Clarify and organize nota details (GTD Clarify/Organize step)
- Required: `id`
- Optional: `title`, `status`, `project`, `context`, `notes`, `start_date`
- Can transform types by changing status (task→project, task→context, etc.)
- Use empty string "" to clear optional fields
- After capturing to inbox, use this to add context and clarify next steps

**change_status** - Move notas through GTD workflow stages (GTD Do/Organize step)
- Required: `id`, `new_status`
- Optional: `start_date` (YYYY-MM-DD, required when moving to calendar status)
- Supports all workflow transitions including type transformations
- Common workflow: inbox → next_action → done, or inbox → waiting_for, or inbox → trash

### Maintenance

**empty_trash** - Permanently delete all trashed notas (GTD Purge step)
- No parameters required
- Irreversible operation - run weekly as part of GTD review
- Automatically checks for references to prevent broken links

## Data Storage

Tasks are stored in TOML format (default: `gtd.toml`). The format is human-readable and Git-friendly:

```toml
format_version = 3

[[inbox]]
id = "#1"
title = "Review project proposal"
project = "q1-marketing"
context = "Office"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[project]]
id = "q1-marketing"
title = "Q1 Marketing Campaign"

[[context]]
name = "Office"
notes = "Work environment with desk and computer"
```

The server automatically migrates older format versions (v1, v2) to the current version (v3) when loading data files.

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

- **[FEATURES_JA.md](FEATURES_JA.md)** - Detailed technical specification (Japanese)
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

