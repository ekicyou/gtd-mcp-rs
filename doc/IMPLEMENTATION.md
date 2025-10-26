# GTD MCP Server Implementation

## Overview

This is a unified nota implementation of a GTD (Getting Things Done) MCP (Model Context Protocol) server in Rust. The server provides task, project, and context management through a simplified 5-tool interface.

**Version 0.7.2**

This version uses `mcp-attr` v0.0.7 for declarative server building and implements a unified nota interface:

- ✅ Uses declarative server building with `#[mcp_server]` and `#[tool]` macros
- ✅ Automatic JSON Schema generation from function signatures
- ✅ Full MCP 2025-03-26 protocol support
- ✅ Unified nota interface (5 tools handle all operations)
- ✅ Client-provided arbitrary string IDs
- ✅ Type transformations via status changes

## Architecture

### Components

1. **Data Structures** (`src/gtd.rs`)
   - `Nota`: Unified type representing tasks, projects, and contexts
   - `NotaStatus`: Enum defining status types (inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context)
   - `Task`, `Project`, `Context`: Type aliases for nota filtering (deprecated internal types)
   - `GtdData`: Container managing all notas with unified storage and access methods

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

The server implements 5 unified tools that handle all GTD operations:

### inbox
Captures anything that needs attention (GTD Capture step). All items start here.

**Parameters:**
- `id` (required): Any string identifier (e.g., "call-john", "website-redesign")
- `title` (required): Brief description of the item
- `status` (required): inbox/next_action/waiting_for/later/calendar/someday/done/project/context/trash
- `project` (optional): Parent project ID (must exist if specified)
- `context` (optional): Context name (e.g., "@office", "@home") (must exist if specified)
- `notes` (optional): Additional details in Markdown format
- `start_date` (optional): Start date in YYYY-MM-DD format (required for calendar status)

**Automatic Fields:**
- `created_at` (date): Automatically set to current local date
- `updated_at` (date): Automatically set to current local date

**Referential Integrity:** Project and context references are validated before creating the nota.

**Type Determination:** Status determines the nota type:
- Task statuses: inbox, next_action, waiting_for, later, calendar, someday, done, trash
- Project status: project
- Context status: context

### list
Reviews all notas with optional filtering (GTD Review step).

**Parameters:**
- `status` (optional): Filter by status (inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context). Empty = all notas.
- `date` (optional): Date filter in YYYY-MM-DD format. For calendar status, only shows tasks with start_date <= this date.
- `exclude_notes` (optional): Set to `true` to exclude notes from output and reduce token usage. Default is `false`.

**Output Format:** Each nota is displayed with:
- Nota ID
- Title
- Status
- Type (task/project/context)
- Start date (if set)
- Project reference (if set)
- Context reference (if set)
- Notes (if set and not excluded)

### update
Updates nota details (GTD Clarify/Organize step). All parameters are optional except the ID.

**Parameters:**
- `id` (required): Nota ID to update
- `title` (optional): New title
- `status` (optional): New status - can transform nota type (task↔project↔context)
- `project` (optional): New project ID (use empty string "" to remove)
- `context` (optional): New context name (use empty string "" to remove)
- `notes` (optional): New notes (use empty string "" to remove)
- `start_date` (optional): New start date in YYYY-MM-DD format (use empty string "" to remove)

**Automatic Updates:**
- `updated_at` (date): Automatically updated to current local date

**Referential Integrity:** Project and context references are validated to ensure they exist.

**Note:** Changing status can transform the nota type (e.g., task to project, task to context).

### change_status
Moves nota through GTD workflow stages (GTD Do/Organize step).

**Parameters:**
- `id` (required): Nota ID to move
- `new_status` (required): Target status (inbox, next_action, waiting_for, later, calendar, someday, done, trash, project, context)
- `start_date` (optional): Start date in YYYY-MM-DD format (required when moving to calendar status if nota doesn't already have a start_date)

**Automatic Updates:**
- `updated_at` (date): Automatically updated to current local date

**Validation:**
- Calendar status requires a start_date (either provided or already set)
- Cannot trash notas that are still referenced by other items

**Type Transformation:** Changing status can transform nota types (e.g., task→project by setting status="project").

### empty_trash
Permanently deletes all trashed notas (GTD Purge step).

**Parameters:** None

**Safety:** Automatically validates that no other notas reference the items being deleted.

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

### inbox_guide
Best practices for capturing notas (tasks, projects, contexts):
- Good vs. poor nota title examples
- When to use optional fields (project, context, notes, start_date)
- Recommended workflow (quick capture → process → add details)

## Data Storage Format

Data is stored in TOML format in `gtd.toml`:

```toml
format_version = 2

[[inbox]]
id = "review-proposal"
title = "Review project proposal"
project = "q1-marketing"
context = "Office"
start_date = "2024-12-25"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[[next_action]]
id = "complete-docs"
title = "Complete documentation"
notes = "Review all sections and update examples"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[projects.q1-marketing]
name = "Q1 Marketing Campaign"
description = "Launch new product marketing campaign"
status = "active"

[contexts.Office]
description = "Work environment with desk and computer"
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

## Client-Provided ID System

The server uses a flexible client-provided ID system for optimal LLM interaction and user control:

### ID Format
- **Arbitrary strings**: Any string identifier chosen by the MCP client (e.g., "call-john", "website-redesign", "meeting-prep")
- **No restrictions**: IDs can be descriptive, numeric, or follow any naming convention the user prefers
- **Unique across all nota types**: Each ID must be unique across tasks, projects, and contexts

### Benefits:
- ✅ **Maximum flexibility** - Users choose meaningful IDs that make sense for their workflow
- ✅ **Descriptive identifiers** - IDs like "call-sarah" are more memorable than auto-generated numbers
- ✅ **LLM-friendly** - Language models can more easily reference and understand semantic IDs
- ✅ **User control** - Complete freedom in ID naming conventions
- ✅ **Portable** - IDs travel with the data and have semantic meaning outside the system

### Example IDs:
```
Tasks: "call-john", "review-q1-report", "buy-groceries", "meeting-prep"
Projects: "website-redesign", "q1-budget", "home-renovation"
Contexts: "@office", "@home", "@computer", "@phone"
```

### Migration from Previous Versions:
Previous versions (pre-0.7.x) used auto-generated counter-based IDs (`#1`, `#2`, `project-1`, `project-2`). The unified nota interface (0.7.x+) switched to client-provided arbitrary string IDs for improved flexibility and semantic clarity.

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
2. **Commits** changes with descriptive messages based on the operation (e.g., "Add nota review-proposal", "Update nota complete-docs")
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
