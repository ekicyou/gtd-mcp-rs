//! Inbox/Add operation tests
mod common;

use gtd_mcp::*;
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_server() -> GtdServerHandler {
    let data = Arc::new(Mutex::new(GtdData::new()));
    GtdServerHandler::new(data, None, false)
}

#[tokio::test]
async fn test_add_simple_task() {
    let server = create_test_server();
    
    let result = server.add("task1", "My First Task".to_string(), None, None, None, None).await;
    assert!(result.is_ok());
    
    let list = server.list(None, None, None).await.unwrap();
    assert!(list.contains("task1"));
    assert!(list.contains("My First Task"));
}

#[tokio::test]
async fn test_add_with_project() {
    let server = create_test_server();
    
    let result = server.add(
        "task1",
        "Task with project".to_string(),
        Some("project1".to_string()),
        None,
        None,
        None
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_with_context() {
    let server = create_test_server();
    
    let result = server.add(
        "task1",
        "Task with context".to_string(),
        None,
        Some("home".to_string()),
        None,
        None
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_with_notes() {
    let server = create_test_server();
    
    let result = server.add(
        "task1",
        "Task with notes".to_string(),
        None,
        None,
        Some("This is a detailed note".to_string()),
        None
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_with_start_date() {
    let server = create_test_server();
    
    let result = server.add(
        "task1",
        "Task with date".to_string(),
        None,
        None,
        None,
        Some("2025-01-15".to_string())
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_duplicate_id() {
    let server = create_test_server();
    
    server.add("task1", "First task".to_string(), None, None, None, None).await.unwrap();
    
    let result = server.add("task1", "Duplicate task".to_string(), None, None, None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
async fn test_add_multiple_tasks() {
    let server = create_test_server();
    
    server.add("task1", "Task 1".to_string(), None, None, None, None).await.unwrap();
    server.add("task2", "Task 2".to_string(), None, None, None, None).await.unwrap();
    server.add("task3", "Task 3".to_string(), None, None, None, None).await.unwrap();
    
    let list = server.list(None, None, None).await.unwrap();
    assert!(list.contains("task1"));
    assert!(list.contains("task2"));
    assert!(list.contains("task3"));
}
