# gtd-mcp-rs Coding Instructions

## Architecture Overview

This is an MCP (Model Context Protocol) server implementing GTD (Getting Things Done) task management in Rust. The architecture follows a three-layer pattern:

- **`src/main.rs`**: MCP server handler using `mcp-attr` declarative macros (`#[mcp_server]`, `#[tool]`)
- **`src/gtd.rs`**: Core domain models (`Task`, `Project`, `Context`, `GtdData`) with TOML serialization
- **`src/storage.rs`**: File persistence layer for `gtd.toml` storage

Data flow: MCP client → stdio → `GtdServerHandler` → `GtdData` (in-memory) → `Storage` → `gtd.toml` file

## Critical Implementation Details

### Enum Naming Convention
**Important**: All enums use snake_case variants (not PascalCase) to match TOML serialization:
```rust
#[allow(non_camel_case_types)]
pub enum TaskStatus {
    inbox,           // NOT Inbox
    next_action,     // NOT NextAction
    waiting_for,
    someday,
    done,
    trash,
}
```
This is enforced by tests (e.g., `test_enum_snake_case_serialization`) and must be preserved.

### MCP Tool Implementation Pattern
All MCP tools follow this pattern:
1. Lock the mutex: `let mut data = self.data.lock().unwrap();`
2. Perform operation on `GtdData`
3. Drop the lock: `drop(data);`
4. Save to disk: `self.save_data()?`
5. Use `bail!()` for errors (from `mcp_attr::bail`)

Example:
```rust
#[tool]
async fn add_task(&self, title: String, ...) -> McpResult<String> {
    let mut data = self.data.lock().unwrap();
    data.add_task(task);
    drop(data);
    if let Err(e) = self.save_data() {
        bail!("Failed to save: {}", e);
    }
    Ok(format!("Task created with ID: {}", task_id))
}
```

### Date Handling
- Use `chrono::NaiveDate` for dates (no time component)
- Parse format: `YYYY-MM-DD` via `NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")`
- Optional dates use `Option<NaiveDate>`

### Data Storage
- TOML format via `toml::to_string_pretty()` for human readability
- File path: `gtd.toml` in current working directory
- Git-friendly format for version control
- Storage operations return `anyhow::Result`

## Development Workflows

### Building
```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Testing
```bash
cargo test              # Run all 41 unit tests
```
Tests use temp files via `env::temp_dir()` and clean up afterward.

### Running the Server
```bash
cargo run               # Starts stdio MCP server
# Or: ./target/release/gtd-mcp-rs
```
The server communicates via stdio (JSON-RPC) with MCP clients like Claude Desktop.

### Integration with Claude Desktop
Add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "gtd": {
      "command": "/path/to/gtd-mcp-rs/target/release/gtd-mcp-rs"
    }
  }
}
```

## Testing Conventions

### Test Organization
- Tests in `#[cfg(test)]` modules at bottom of each file
- Japanese comments explain test purpose (e.g., `// 既存ファイルの上書きテスト`)
- Test names are descriptive: `test_storage_save_and_load_comprehensive`

### Test Data Patterns
- Use `get_test_path()` for temp file paths
- Clean up test files: `let _ = fs::remove_file(&test_path);`
- Test both minimal and fully-populated structs
- Verify TOML output matches expected format in `test_complete_toml_output`

## Dependencies

- **`mcp-attr` (0.0.7)**: Declarative MCP server building (cross-platform, replaces `rust-mcp-sdk`)
- **`tokio`**: Async runtime for MCP server
- **`toml` (0.9)**: Serialization (note: uses `toml::to_string_pretty` for readability)
- **`chrono`**: Date handling with `serde` feature
- **`uuid`**: Generate task/project IDs with `v4` feature
- **`schemars`**: JSON Schema generation for MCP
- **`anyhow`**: Error handling with context

## Code Style Patterns

- Use `#[allow(dead_code)]` for helper methods not yet used externally
- Match expressions for enum filtering (e.g., `matches!(task.status, TaskStatus::inbox)`)
- String formatting: `format!("- [{}] {} (status: {:?})\n", ...)`
- Mutex pattern: lock, modify, drop, persist
- Error propagation: `bail!()` for MCP tools, `?` for internal functions

## Gotchas

1. **Edition 2024**: `Cargo.toml` uses `edition = "2024"` (not 2021)
2. **Cross-platform**: Uses `mcp-attr` instead of `rust-mcp-sdk` for Windows compatibility
3. **TOML structure**: Tasks/projects are arrays (`[[tasks]]`), contexts are tables (`[contexts.Name]`)
4. **ID generation**: Always use `uuid::Uuid::new_v4().to_string()` for new entities
5. **Status filtering**: String matching in `list_tasks` uses hardcoded match arms, not dynamic enum parsing
