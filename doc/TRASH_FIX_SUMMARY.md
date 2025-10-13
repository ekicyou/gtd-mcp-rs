# Issue Fix Summary: Inbox to Trash Functionality

## Problem Report
User reported "Tool execution failed" when trying to trash a task directly from inbox status. The error message had no details or stack trace. However, the workaround of going inbox → done → trash worked successfully.

## Investigation Results
- All existing tests passed (138 tests)
- Core functionality works correctly at both GtdData and MCP handler levels
- Direct inbox → trash transition works as expected in tests
- No bugs found in the move_status logic or trash_task implementation

## Root Cause Analysis
The reported issue appears to be related to error message propagation through the MCP protocol rather than a functional bug. The generic "Tool execution failed" message suggests that:
1. Detailed error information may not have been reaching the MCP client
2. There may have been a transient environmental issue (permissions, file access, etc.)
3. Improved logging was needed for debugging

## Changes Made

### 1. Enhanced Error Handling in `trash_task` (src/main.rs)
- Added pre-validation to check if task exists before attempting move
- Added diagnostic logging using `eprintln!` for debugging:
  - Log when task is not found
  - Log the task's original status when moving
  - Log success/failure of save operations
  - Log internal errors if they occur
- Improved error messages to be more specific and actionable:
  - "Task not found: {}. Please check the task ID and try again."
  - "Failed to save task to trash: {}. The task may not have been moved."
  - "Failed to move task {} to trash. Internal error occurred."

### 2. Added PartialEq and Eq Traits (src/gtd.rs)
- Added `PartialEq` and `Eq` derives to `TaskStatus` enum
- Added `PartialEq` and `Eq` derives to `ProjectStatus` enum
- These traits enable better enum comparisons and may help with edge cases

### 3. Comprehensive Test Coverage (src/main.rs)
Added three new tests to validate the functionality:

#### test_trash_task_from_inbox
- Tests direct inbox → trash transition
- Validates task ends up in correct container
- Validates status is correctly updated

#### test_trash_task_workflow_comparison  
- Tests both workflows side-by-side:
  1. inbox → trash (direct, reported as failing)
  2. inbox → done → trash (indirect, reported as working)
- Validates both tasks end up in trash
- Proves both workflows work correctly

#### test_trash_task_error_messages
- Tests error handling with invalid task IDs
- Validates error logging works correctly
- Tests various invalid ID formats (#999, invalid-id, task-999)

## Test Results
All 141 tests pass successfully, including:
- 138 original tests
- 3 new tests specifically for this issue

## Benefits of Changes
1. **Better Debugging**: stderr logging provides diagnostic information
2. **Better UX**: More descriptive error messages help users understand issues
3. **Better Coverage**: Tests validate both reported workflows
4. **Better Reliability**: Pre-validation prevents unexpected internal states

## Recommendation
If the issue occurs again, check:
1. Server stderr output for diagnostic logs
2. File permissions on gtd.toml
3. Git configuration if --sync-git is enabled
4. Available disk space
5. MCP client logs for additional context

The enhanced logging will provide specific information about what failed, making it much easier to diagnose the root cause.
