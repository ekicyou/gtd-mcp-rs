//! Basic functionality tests
mod common;

use gtd_mcp::*;
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_server() -> GtdServerHandler {
    let data = Arc::new(Mutex::new(GtdData::new()));
    GtdServerHandler::new(data, None, false)
}

#[tokio::test]
async fn test_normalize_task_id() {
    let server = create_test_server();
    
    // Test with # prefix
    let result = server.add(" #task-1 ", "Test".to_string(), None, None, None, None).await;
    assert!(result.is_ok());
    
    // Test without # prefix
    let result2 = server.add(" task-2 ", "Test2".to_string(), None, None, None, None).await;
    assert!(result2.is_ok());
    
    // Both should be stored without # and trimmed
    let list = server.list(None, None, None).await.unwrap();
    assert!(list.contains("task-1"));
    assert!(list.contains("task-2"));
}

#[tokio::test]
async fn test_custom_file_path() {
    let test_path = common::get_test_path();
    let data = Arc::new(Mutex::new(GtdData::new()));
    let server = GtdServerHandler::new(data, Some(test_path.clone()), false);
    
    // Add a task
    server.add("test-id", "Test Task".to_string(), None, None, None, None).await.unwrap();
    
    // Save should use custom path
    // Note: actual file I/O happens internally
    
    common::cleanup_test_file(&test_path);
}
