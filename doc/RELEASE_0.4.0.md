# Version 0.4.0 Release Notes

## Summary

This release updates gtd-mcp to version 0.4.0 with significant API improvements. All status movement methods now support batch operations, allowing multiple tasks to be moved at once. This is a breaking change from version 0.3.2.

## Changes

### Version Update
- **Version**: Updated from 0.3.2 to 0.4.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

### API Changes - Batch Operations

All status movement methods now support moving multiple tasks at once. This is a **breaking change** - the method names and signatures have changed:

#### Method Renames (Breaking Changes)
- `inbox_task` → `inbox_tasks` (now accepts `task_ids: Vec<String>`)
- `next_action_task` → `next_action_tasks` (now accepts `task_ids: Vec<String>`)
- `waiting_for_task` → `waiting_for_tasks` (now accepts `task_ids: Vec<String>`)
- `someday_task` → `someday_tasks` (now accepts `task_ids: Vec<String>`)
- `later_task` → `later_tasks` (now accepts `task_ids: Vec<String>`)
- `done_task` → `done_tasks` (now accepts `task_ids: Vec<String>`)

#### Enhanced Methods
- `trash_tasks` - Already supported batch operations, unchanged
- `calendar_tasks` - Already supported batch operations, unchanged

### Documentation Updates

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

## Testing Performed

- ✅ All 168 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.4.0

## Breaking Changes

**Important**: This release contains breaking changes to the API.

### Status Movement Methods

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

### Migration Guide

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

## Benefits of This Release

1. **Improved Efficiency**: Move multiple tasks with a single operation
2. **Better LLM Interaction**: Language models can now process multiple tasks more efficiently
3. **Consistent API**: All status movement methods now follow the same pattern
4. **Backward Compatible Data**: Existing `gtd.toml` files work without modification

## How to Create a Release

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

## Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

## Next Steps

To publish version 0.4.0:
1. Merge this PR to main
2. Create and push the v0.4.0 tag from the main branch
3. GitHub Actions will automatically create the release with all binary artifacts
