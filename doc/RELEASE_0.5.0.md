# Version 0.5.0 Release Notes

## Summary

This release updates gtd-mcp to version 0.5.0 with an important API change. The `add_project` method now requires an explicit project ID instead of auto-generating one. This is a breaking change from version 0.4.0, but existing `gtd.toml` files are automatically migrated on load.

## Changes

### Version Update
- **Version**: Updated from 0.4.0 to 0.5.0
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

### API Changes - Required Project ID

The `add_project` method now requires a project ID to be explicitly provided:

#### Breaking Change
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

### Data Format Migration

The underlying data format remains compatible:
- **Format Version**: 2 (unchanged)
- **Projects Storage**: HashMap with project ID as key (unchanged)
- **Automatic Migration**: Old TOML files from version 1 are still automatically migrated on load

### Documentation Updates

All documentation has been updated to reflect the new version:
- Cargo.toml
- README.md
- RELEASE_0.5.0.md

## Testing Performed

- ✅ All 175 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.5.0

## Breaking Changes

**Important**: This release contains a breaking change to the `add_project` API.

### Project Creation

The `add_project` method signature has changed:

**Old signature (v0.4.0):**
- Project ID was auto-generated based on a counter
- Users only needed to provide name and optional fields

**New signature (v0.5.0):**
- Project ID must be explicitly provided
- Provides better control over project identifiers
- Prevents confusion about auto-generated IDs

### Migration Guide

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

## Benefits of This Release

1. **Better Control**: Users have explicit control over project identifiers
2. **Predictable IDs**: No confusion about auto-generated ID patterns
3. **Easier Integration**: Scripts and integrations can use known project IDs
4. **Backward Compatible Data**: Existing `gtd.toml` files work without modification
5. **Format Migration**: Old format (Vec) is still automatically converted to new format (HashMap)

## How to Create a Release

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

## Distribution Binaries

The following binaries are automatically built for this release:

- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

All binaries are available from the GitHub release page.

## Next Steps

To publish version 0.5.0:
1. Merge this PR to main
2. Create and push the v0.5.0 tag from the main branch
3. GitHub Actions will automatically create the release with all binary artifacts
