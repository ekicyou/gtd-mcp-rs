# MCP Tools Documentation

This document describes the MCP (Model Context Protocol) tools provided by the GTD MCP server. These tools implement the GTD (Getting Things Done) methodology and are designed to be used by LLM clients like Claude Desktop.

## Overview

The GTD MCP server provides 5 main tools that follow the GTD workflow:

1. **inbox** - Capture: Quickly capture anything needing attention
2. **list** - Review: List and review all notas with filtering
3. **update** - Clarify/Organize: Add details and organize captured items
4. **change_status** - Do/Organize: Move items through workflow stages
5. **empty_trash** - Purge: Permanently delete trashed items

## GTD Workflow

The tools are designed to support the complete GTD workflow:

```
Capture (inbox) → Review (list) → Clarify (update) → Organize (change_status) → Do → Purge (empty_trash)
                      ↑                                                                      
                      └──────────────── Weekly Review ────────────────────────────────────┘
```

## Tool Details

### 1. inbox - GTD Capture

**Purpose**: Quickly capture anything that needs attention. This is the first step in GTD - all items start here.

**GTD Context**: Use this tool whenever something crosses your mind that requires attention, action, or tracking. Don't think, just capture.

**Parameters**:
- `id` (required): Any string identifier (e.g., "call-john", "web-redesign")
- `title` (required): Brief description of the item
- `status` (required): One of: inbox/next_action/waiting_for/later/calendar/someday/done/reference/project/context/trash
- `project` (optional): Parent project ID if this belongs to a project
- `context` (optional): Where this applies (e.g., "@home", "@office")
- `notes` (optional): Markdown-formatted detailed notes
- `start_date` (optional): YYYY-MM-DD format, required for calendar status

**Workflow**:
1. Capture everything into inbox
2. Use `list` to review captured items
3. Use `update` or `change_status` to organize them

**Examples**:
```
inbox(id="call-john", title="Call John about project", status="inbox")
inbox(id="website-redesign", title="Website Redesign Project", status="project")
inbox(id="home", title="Home", status="context", notes="Tasks to do at home")
```

### 2. list - GTD Review

**Purpose**: List and view all notas with filtering options. Essential for daily and weekly reviews.

**GTD Context**: Use this tool regularly to review your system. Filter by status to focus on specific areas.

**Parameters**:
- `status` (optional): Filter by status (inbox/next_action/waiting_for/later/calendar/someday/done/reference/project/context/trash)
- `date` (optional): YYYY-MM-DD format - For calendar status, only shows tasks with start_date <= this date
- `exclude_notes` (optional): Boolean - Set to true to reduce token usage by excluding notes

**Common Filters**:
- No filter: Show all items
- `status="inbox"`: Show uncaptured/unprocessed items
- `status="next_action"`: Show ready-to-do tasks
- `status="project"`: Show all projects
- `status="calendar"` + `date="2024-12-25"`: Show calendar items due by that date

**Workflow**:
- **Daily Review**: `list(status="next_action")` to see what to do today
- **Weekly Review**: `list()` to review entire system, `list(status="inbox")` to process new items

### 3. update - GTD Clarify/Organize

**Purpose**: Update nota details. Add context, clarify next steps, link to projects.

**GTD Context**: After capturing items in inbox, use this tool to clarify what each item is and add relevant details.

**Parameters**:
- `id` (required): ID of nota to update
- `title` (optional): New title
- `status` (optional): New status (changes type if project/context)
- `project` (optional): Project link, use "" to clear
- `context` (optional): Context tag, use "" to clear
- `notes` (optional): Notes in Markdown, use "" to clear
- `start_date` (optional): Start date YYYY-MM-DD, use "" to clear

**Tip**: Use empty string "" to clear optional fields.

**Workflow**:
1. After capturing items with `inbox`
2. Review with `list(status="inbox")`
3. Clarify each item: What is it? What's the context? Does it belong to a project?
4. Use `update` to add details

**Examples**:
```
update(id="call-john", context="@phone", notes="Discuss Q1 budget")
update(id="meeting-prep", project="website-redesign", context="@office")
update(id="old-task", notes="")  # Clear notes
```

### 4. change_status - GTD Do/Organize

**Purpose**: Move notas through workflow stages.

**GTD Context**: Use this tool to move items through the GTD workflow as you process and complete them.

**Parameters**:
- `id` (required): Nota ID
- `new_status` (required): New status (inbox/next_action/waiting_for/later/calendar/someday/done/reference/project/context/trash)
- `start_date` (optional): Start date YYYY-MM-DD (required for calendar status)

**Workflow Stages**:
- `inbox` → `next_action`: Ready to do
- `inbox` → `waiting_for`: Blocked, waiting for someone/something
- `inbox` → `later`: Deferred but planned
- `inbox` → `calendar`: Date-specific (requires start_date)
- `inbox` → `someday`: Maybe someday
- `inbox` → `reference`: Non-actionable reference material
- `next_action` → `done`: Completed
- Any → `trash`: Mark for deletion (use before empty_trash)

**Tip**: Setting status="project" or "context" transforms the nota type. Use change_status before empty_trash to delete items.

**Examples**:
```
change_status(id="call-john", new_status="next_action")
change_status(id="meeting-prep", new_status="calendar", start_date="2024-12-25")
change_status(id="old-task", new_status="done")
change_status(id="spam", new_status="trash")
```

### 5. empty_trash - GTD Purge

**Purpose**: Permanently delete trashed items. Part of weekly review process.

**GTD Context**: Run this weekly to purge items you've decided to discard. The system checks references to prevent broken links.

**Parameters**: None

**Workflow**:
1. Use `change_status` to move items to trash
2. Review trashed items with `list(status="trash")`
3. Run `empty_trash` to permanently delete all trashed items

**Safety**: The system prevents deletion of items that are still referenced by other items (e.g., a project that still has tasks).

**Example**:
```
# First, trash the item
change_status(id="old-task", new_status="trash")

# Then, permanently delete all trashed items
empty_trash()
```

## GTD Status Types

The server supports the following status types:

- **inbox**: Unprocessed items (start here)
- **next_action**: Ready-to-execute tasks (focus here for daily work)
- **waiting_for**: Blocked tasks awaiting someone/something
- **later**: Deferred but planned tasks
- **calendar**: Date-specific tasks (requires start_date)
- **someday**: Potential future actions
- **done**: Completed tasks
- **reference**: Non-actionable reference material for future use
- **trash**: Deleted tasks (will be permanently removed by empty_trash)
- **project**: Multi-step endeavors
- **context**: Environments/tools (e.g., @office, @home, @phone)

## Best Practices

### Daily Review
1. `list(status="calendar", date="2024-12-25")` - Check today's calendar items
2. `list(status="next_action")` - Review ready-to-do tasks
3. Process completed tasks: `change_status(id="task-1", new_status="done")`

### Weekly Review
1. `list(status="inbox")` - Process all inbox items
2. For each inbox item:
   - Clarify with `update`
   - Organize with `change_status`
3. `list(status="project")` - Review all projects
4. `list(status="waiting_for")` - Check on blocked items
5. `list(status="someday")` - Review potential future actions
6. `empty_trash()` - Purge deleted items

### Capturing Items
- **Tasks**: `inbox(id="task-id", title="...", status="inbox")`
- **Projects**: `inbox(id="project-id", title="...", status="project")`
- **Contexts**: `inbox(id="context-name", title="...", status="context")`

### Token Optimization
- Use `list(status="...", exclude_notes=true)` to reduce token usage when notes aren't needed
- Keep tool documentation concise
- Use meaningful but short IDs

## Technical Notes

### ID Format
- Task IDs: Use descriptive strings (e.g., "call-john", "meeting-prep")
- Project IDs: Use meaningful abbreviations (e.g., "website-redesign", "q1-budget")
- Context IDs: Typically just the context name (e.g., "Office", "Home")

### Date Format
- All dates use YYYY-MM-DD format (e.g., "2024-12-25")
- Dates are required for calendar status
- Date filtering in `list` only affects calendar status items

### Reference Integrity
- The system prevents deletion of items that are still referenced
- Before deleting a project, remove all task references to it
- Before deleting a context, remove all references from tasks and projects

## See Also

- [GTD Methodology Assessment](GTD_ASSESSMENT.md) - Full GTD implementation details
- [Implementation Notes](IMPLEMENTATION.md) - Technical implementation details
- [README.md](../README.md) - Project overview and setup
