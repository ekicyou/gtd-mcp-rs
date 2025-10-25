//! Change status tests
mod common;

use gtd_mcp::*;
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_server() -> GtdServerHandler {
    let data = Arc::new(Mutex::new(GtdData::new()));
    GtdServerHandler::new(data, None, false)
}

#[tokio::test]
async fn test_change_status_single_task() {
    let server = create_test_server();
    
    server.add("task1", "Task 1".to_string(), None, None, None, None).await.unwrap();
    
    let result = server.change_status(vec!["task1".to_string()], "next_action".to_string(), None).await;
    assert!(result.is_ok());
    
    let list = server.list(Some("next_action".to_string()), None, None).await.unwrap();
    assert!(list.contains("task1"));
}

#[tokio::test]
async fn test_change_status_multiple_tasks() {
    let server = create_test_server();
    
    server.add("task1", "Task 1".to_string(), None, None, None, None).await.unwrap();
    server.add("task2", "Task 2".to_string(), None, None, None, None).await.unwrap();
    
    let result = server.change_status(
        vec!["task1".to_string(), "task2".to_string()],
        "done".to_string(),
        None
    ).await;
    assert!(result.is_ok());
    
    let list = server.list(Some("done".to_string()), None, None).await.unwrap();
    assert!(list.contains("task1"));
    assert!(list.contains("task2"));
}

#[tokio::test]
async fn test_change_status_with_note() {
    let server = create_test_server();
    
    server.add("task1", "Task 1".to_string(), None, None, None, None).await.unwrap();
    
    let result = server.change_status(
        vec!["task1".to_string()],
        "done".to_string(),
        Some("Completed successfully".to_string())
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_change_status_nonexistent_task() {
    let server = create_test_server();
    
    let result = server.change_status(
        vec!["nonexistent".to_string()],
        "done".to_string(),
        None
    ).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_change_status_invalid_status() {
    let server = create_test_server();
    
    server.add("task1", "Task 1".to_string(), None, None, None, None).await.unwrap();
    
    let result = server.change_status(
        vec!["task1".to_string()],
        "invalid_status".to_string(),
        None
    ).await;
    assert!(result.is_err());
}
