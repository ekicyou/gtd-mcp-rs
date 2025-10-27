# Error Handling in GTD MCP Server

## Overview

The GTD MCP server follows JSON-RPC 2.0 specification for error handling. When errors occur during tool execution, the server returns detailed error messages that describe the problem and suggest solutions.

## Known Limitation with Claude.ai Web UI

### Issue

When using the GTD MCP server with **Claude.ai's web interface**, detailed error messages may not be visible to users. Instead, Claude.ai may only display a generic message like:

```
Tool execution failed
```

### Root Cause

This is a **client-side limitation** in how Claude.ai's web UI handles JSON-RPC protocol errors:

1. **Server Behavior (Correct)**: The GTD MCP server correctly returns JSON-RPC error responses with detailed error messages, following the MCP specification:
   ```json
   {
     "jsonrpc": "2.0",
     "id": "request-id",
     "error": {
       "code": -32603,
       "message": "Duplicate ID error: ID 'triton-research' already exists (status: inbox). Each item must have a unique ID. Please choose a different ID.",
       "data": {}
     }
   }
   ```

2. **Client Behavior (Limited)**: Claude.ai's web UI appears to filter or sanitize these protocol-level error messages, showing only a generic "Tool execution failed" message to protect users from potentially confusing technical details.

### Why This Design Choice

The MCP specification provides two ways to return errors:

1. **Protocol-level errors** (JSON-RPC errors): For system/protocol issues like malformed requests, missing parameters, or internal server errors. These are returned with HTTP error codes or JSON-RPC error objects.

2. **Application-level errors** (isError field): For business logic errors that the LLM should be able to reason about and potentially fix. These are returned as successful responses with `isError: true` flag.

The `mcp-attr` library (v0.0.7) currently only supports protocol-level errors via the `bail!()` macro, which is why our errors are sent as JSON-RPC protocol errors.

## Current Error Messages

The GTD MCP server provides detailed, actionable error messages for common issues:

### Duplicate ID Error
```
Duplicate ID error: ID '{id}' already exists (status: {status}). 
Each item must have a unique ID. Please choose a different ID.
```

### Invalid Status Error  
```
Invalid status '{status}'. Valid statuses: inbox, next_action, waiting_for, 
later, calendar, someday, done, reference, trash, project, context
```

### Invalid Project Reference
```
Invalid project reference: Project '{project_id}' does not exist. 
Create the project first or use an existing project ID.
```

### Invalid Context Reference
```
Invalid context reference: Context '{context_name}' does not exist. 
Create the context first or use an existing context name.
```

### Calendar Validation Error
```
Calendar status validation failed: status=calendar requires start_date parameter. 
Please provide a date in YYYY-MM-DD format.
```

### Invalid Date Format
```
Invalid date format '{date_str}'. Use YYYY-MM-DD (e.g., '2025-03-15')
```

### Item Not Found
```
Item not found: Item '{id}' does not exist. Use list() to see available items.
```

### Reference Integrity Error
```
Cannot trash '{id}': still referenced by other items. Remove references first.
```

## Workarounds

### Option 1: Use Claude Desktop (Recommended)

Claude Desktop may handle MCP error messages differently than the web UI. Consider using Claude Desktop for a better development experience when working with MCP servers.

### Option 2: Check Server Logs

The GTD MCP server logs all errors. When troubleshooting, check the server's log output to see the full error messages:

```bash
# Run the server with logging
./target/release/gtd-mcp /path/to/gtd.toml
```

### Option 3: Test with Direct MCP Client

Use an MCP client library to test the server directly and see the full error responses:

```bash
# Example using curl or an MCP testing tool
mcp-cli call --server gtd-mcp --tool inbox --args '{"id": "duplicate", ...}'
```

### Option 4: Rely on LLM Reasoning

Even though the detailed error message isn't shown in the UI, the LLM (Claude) may receive it in the context. The LLM can use this information to:
- Understand what went wrong
- Suggest corrections to the user
- Retry the operation with fixed parameters

## Future Improvements

### Potential Solutions

1. **Library Update**: Future versions of `mcp-attr` may support returning tool responses with `isError: true` field, which might be displayed better by Claude.ai.

2. **Client Update**: Anthropic may update Claude.ai to display protocol-level error messages more clearly.

3. **Hybrid Approach**: Return both protocol errors AND include error information in successful responses (though this would require library changes).

## For Developers

If you're contributing to this project and need to add new error messages:

1. Use `bail!()` macro with detailed, actionable error messages:
   ```rust
   bail!(
       "Duplicate ID error: ID '{}' already exists (status: {:?}). \
        Each item must have a unique ID. Please choose a different ID.",
       id,
       existing_status
   );
   ```

2. Include:
   - **What went wrong**: Clear description of the error
   - **Why it's a problem**: Context about the issue
   - **How to fix it**: Actionable steps to resolve the error

3. Test error messages by:
   - Running unit tests
   - Testing with a direct MCP client
   - Verifying the error appears in server logs

## Related Issues

- Issue #188: Error messages not displayed in Claude.ai web UI
- MCP Specification: [Error Handling Best Practices](https://mcpcat.io/guides/error-handling-custom-mcp-servers/)
- `mcp-attr` Library: [Error Handling Documentation](https://docs.rs/mcp-attr)

## References

- [MCP Specification - Messages](https://modelcontextprotocol.info/specification/2024-11-05/basic/messages/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Error Handling in MCP Servers - MCPcat Guide](https://mcpcat.io/guides/error-handling-custom-mcp-servers/)
