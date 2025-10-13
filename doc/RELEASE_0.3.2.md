# Version 0.3.2 Release Notes

## Summary

This release updates gtd-mcp to version 0.3.2 with a routine version increment.

## Changes

### Version Update
- **Version**: Updated from 0.3.1 to 0.3.2
- **Crate name**: gtd-mcp (unchanged)
- **Binary name**: gtd-mcp (unchanged)

### Documentation Updates
All documentation has been updated to reflect the new version:
- Cargo.toml
- README.md
- IMPLEMENTATION.md

## Testing Performed

- ✅ All 154 unit tests pass
- ✅ Code formatting check passes (`cargo fmt --check`)
- ✅ Clippy linting passes with no warnings (`cargo clippy -- -D warnings`)
- ✅ Debug build compiles successfully
- ✅ Release build compiles successfully
- ✅ Binary version output shows 0.3.2

## Breaking Changes

None. This is a routine version update with no changes to functionality.

## How to Create a Release

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

## Distribution Binaries

When the v0.3.2 tag is pushed, GitHub Actions will build and publish binaries for:
- **Linux**: x86_64-unknown-linux-gnu (glibc-based)
- **Linux**: x86_64-unknown-linux-musl (static, portable)
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin (Intel Macs)
- **macOS**: aarch64-apple-darwin (Apple Silicon)

## Next Steps

To publish version 0.3.2:
1. Merge this PR to main
2. Create and push the v0.3.2 tag from the main branch
3. GitHub Actions will automatically create the release with all binary artifacts
