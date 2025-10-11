# Version 0.3.0 Release Notes

## Summary

This release renames the crate from `gtd-mcp-rs` to `gtd-mcp` for better naming consistency and removes the redundant `-rs` suffix. The version is also bumped to 0.3.0 to reflect this significant change.

## Changes

### Crate Rename
- **Crate name**: Changed from `gtd-mcp-rs` to `gtd-mcp`
- **Binary name**: Changed from `gtd-mcp-rs` to `gtd-mcp`
- **Version**: Updated from 0.2.0 to 0.3.0

### Rationale
The `-rs` suffix is often redundant in the Rust ecosystem, especially when the context is clear. Many popular Rust projects (e.g., `tokio`, `serde`, `clap`) don't use language-specific suffixes. The name `gtd-mcp` is more concise and clearer as it describes what the project is: a GTD (Getting Things Done) implementation of the Model Context Protocol.

### Documentation Updates
All documentation has been updated to reflect the new crate name:
- README.md
- IMPLEMENTATION.md
- GTD_ASSESSMENT.md
- .github/copilot-instructions.md
- .github/workflows/release.yml

### Integration Changes

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

### Build Changes
The release binaries will now be named with the `gtd-mcp` prefix:
- `gtd-mcp-x86_64-unknown-linux-gnu.tar.gz` (Linux glibc)
- `gtd-mcp-x86_64-unknown-linux-musl.tar.gz` (Linux static)
- `gtd-mcp-x86_64-pc-windows-msvc.zip` (Windows)
- `gtd-mcp-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `gtd-mcp-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)

## Testing Performed

- ✅ All 142 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.3.0

## Breaking Changes

**Binary name change**: Users must update their MCP client configurations to use the new binary name `gtd-mcp` instead of `gtd-mcp-rs`. The functionality remains unchanged.

## How to Create a Release

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

## Distribution Binaries

When the v0.3.0 tag is pushed, GitHub Actions will build and publish binaries for:
- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

## Next Steps

To publish version 0.3.0:
1. Merge this PR to main
2. Create and push the v0.3.0 tag from the main branch
3. GitHub Actions will automatically create the release with all binary artifacts
