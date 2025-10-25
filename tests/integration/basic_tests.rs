use super::common::*;
use gtd_mcp::*;
use tempfile::NamedTempFile;

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
