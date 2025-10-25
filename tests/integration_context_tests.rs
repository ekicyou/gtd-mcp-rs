// Context-related integration tests
use gtd_mcp::*;
use std::env;
use std::fs;

fn get_test_path(name: &str) -> String {
    format!("{}/gtd-test-{}.toml", env::temp_dir().display(), name)
}



    use super::*;
    use chrono::NaiveDate;
    use tempfile::NamedTempFile;

    fn get_test_handler() -> (GtdServerHandler, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let handler = GtdServerHandler::new(temp_file.path().to_str().unwrap(), false).unwrap();
        (handler, temp_file)
    }

    #[test]
    fn test_custom_file_path() {
        // カスタムファイルパスでハンドラーを作成
        let temp_file = NamedTempFile::new().unwrap();
        let custom_path = temp_file.path().to_str().unwrap();

        let handler = GtdServerHandler::new(custom_path, false).unwrap();

        // ストレージのファイルパスが正しく設定されていることを確認
        assert_eq!(handler.storage.file_path().to_str().unwrap(), custom_path);

        // データの保存と読み込みが正しく動作することを確認
        let mut data = handler.data.lock().unwrap();
        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        };
        data.add(task);
        drop(data);

        // 保存
        let save_result = handler.save_data();
        assert!(save_result.is_ok());

        // ファイルが作成されていることを確認
        assert!(std::path::Path::new(custom_path).exists());

        // 新しいハンドラーで読み込み
        let handler2 = GtdServerHandler::new(custom_path, false).unwrap();
        let loaded_data = handler2.data.lock().unwrap();
        assert_eq!(loaded_data.task_count(), 1);
        let loaded_task = loaded_data.find_task_by_id("test-task").unwrap();
        assert_eq!(loaded_task.title, "Test Task");
    }

    #[test]
    fn test_normalize_task_id() {
        // Test with arbitrary task IDs - normalize should just trim
        assert_eq!(GtdServerHandler::normalize_task_id("task-1"), "task-1");
        assert_eq!(
            GtdServerHandler::normalize_task_id("meeting-prep"),
            "meeting-prep"
        );
        assert_eq!(
            GtdServerHandler::normalize_task_id("call-sarah"),
            "call-sarah"
        );

        // Test with whitespace - should be trimmed
        assert_eq!(GtdServerHandler::normalize_task_id(" task-1 "), "task-1");
        assert_eq!(
            GtdServerHandler::normalize_task_id("  meeting-prep  "),
            "meeting-prep"
        );

        // Old-style IDs with # are also valid
        assert_eq!(GtdServerHandler::normalize_task_id("#1"), "#1");
        assert_eq!(GtdServerHandler::normalize_task_id(" #42 "), "#42");
    }



mod common;

#[cfg(test)]
mod context_tests {
    use super::*;
    use crate::common::*;

    #[tokio::test]
    async fn test_update_task_invalid_context_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a task
        let result = handler
            .inbox(
                "task-13".to_string(),
                "Test Task".to_string(),
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok());
        let task_id = result
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .to_string();

        // Try to update with non-existent context
        let result = handler
            .update(
                task_id,
                None,
                None,
                Some("NonExistent".to_string()),
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_context() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .inbox("Office".to_string(), Some("Work environment".to_string()))
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Office"));

        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.name, "Office");
        assert_eq!(context.notes, Some("Work environment".to_string()));
    }

    #[tokio::test]
    async fn test_add_context_duplicate() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.inbox("Office".to_string(), None).await;
        assert!(result.is_ok());

        // Try to add duplicate
        let result = handler.inbox("Office".to_string(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_contexts_empty() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.list().await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No contexts found"));
    }

    #[tokio::test]
    async fn test_list_contexts() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox("Office".to_string(), Some("Work environment".to_string()))
            .await
            .unwrap();
        handler.inbox("Home".to_string(), None).await.unwrap();

        let result = handler.list().await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Office"));
        assert!(output.contains("Home"));
        assert!(output.contains("Work environment"));
    }

    #[tokio::test]
    async fn test_update_context() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox("Office".to_string(), Some("Old description".to_string()))
            .await
            .unwrap();

        let result = handler
            .update("Office".to_string(), Some("New description".to_string()))
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.notes, Some("New description".to_string()));
    }

    #[tokio::test]
    async fn test_update_context_remove_description() {
        let (handler, _temp_file) = get_test_handler();

        handler
            .inbox("Office".to_string(), Some("Old description".to_string()))
            .await
            .unwrap();

        let result = handler
            .update("Office".to_string(), Some("".to_string()))
            .await;
        assert!(result.is_ok());

        let data = handler.data.lock().unwrap();
        let context = data.find_context_by_name("Office").unwrap();
        assert_eq!(context.notes, None);
    }

    #[tokio::test]
    async fn test_update_context_not_found() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler
            .update("NonExistent".to_string(), Some("Description".to_string()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_context() {
        let (handler, _temp_file) = get_test_handler();

        handler.inbox("Office".to_string(), None).await.unwrap();

        let result = handler.delete_context("Office".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("deleted"));

        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_context_not_found() {
        let (handler, _temp_file) = get_test_handler();

        let result = handler.delete_context("NonExistent".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_context_with_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler.inbox("Office".to_string(), None).await.unwrap();

        // Add a task that references the context
        handler
            .inbox(
                "task-2006".to_string(),
                "Office work".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail
        let result = handler.delete_context("Office".to_string()).await;
        assert!(result.is_err());

        // Verify context still exists
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
        assert!(data.contexts.contains_key("Office"));
    }

    #[tokio::test]
    async fn test_delete_context_after_removing_task_reference() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler.inbox("Office".to_string(), None).await.unwrap();

        // Add a task that references the context
        let task_id = handler
            .inbox(
                "task-2008".to_string(),
                "Office work".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Extract task ID from the response
        let task_id = task_id.split("ID: ").nth(1).unwrap().trim().to_string();

        // Remove the context reference from the task
        handler
            .update(task_id, None, None, Some(String::new()), None, None)
            .await
            .unwrap();

        // Now deletion should succeed
        let result = handler.delete_context("Office".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("deleted"));

        // Verify context is gone
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_context_with_multiple_task_references() {
        let (handler, _temp_file) = get_test_handler();

        // Add a context
        handler.inbox("Office".to_string(), None).await.unwrap();

        // Add multiple tasks that reference the context
        handler
            .inbox(
                "task-2009".to_string(),
                "Task 1".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        handler
            .inbox(
                "task-2010".to_string(),
                "Task 2".to_string(),
                None,
                Some("Office".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // Try to delete the context - should fail with the first task found
        let result = handler.delete_context("Office".to_string()).await;
        assert!(result.is_err());

        // Verify context still exists
        let data = handler.data.lock().unwrap();
        assert_eq!(data.contexts.len(), 1);
    }

}
