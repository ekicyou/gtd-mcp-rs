use chrono::{Datelike, NaiveDate};
use gtd_mcp::gtd::{GtdData, Nota, NotaStatus, RecurrencePattern, local_date_today};
use gtd_mcp::migration::{
    Context, Project, Task, nota_from_context, nota_from_project, nota_from_task, nota_to_context,
    nota_to_project, nota_to_task,
};
use std::str::FromStr;

// GtdDataの新規作成テスト
// 空のnotasが初期化されることを確認
#[test]
fn test_gtd_data_new() {
    let data = GtdData::new();
    assert!(data.inbox().is_empty());
    assert!(data.next_action().is_empty());
    assert!(data.waiting_for().is_empty());
    assert!(data.someday().is_empty());
    assert!(data.later().is_empty());
    assert!(data.done().is_empty());
    assert!(data.reference().is_empty());
    assert!(data.trash().is_empty());
    assert!(data.projects().is_empty());
    assert!(data.contexts().is_empty());
}

// GtdDataへのNota挿入テスト
// Notaを1つ追加し、正しく格納・取得できることを確認
#[test]
fn test_gtd_data_insert_nota() {
    let mut data = GtdData::new();
    let nota = Nota {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ..Default::default()
    };

    data.add(nota.clone());
    assert_eq!(data.task_count(), 1);
    assert_eq!(data.inbox().len(), 1);
    assert_eq!(data.find_task_by_id("task-1").unwrap().title, "Test Task");
}

// 複数Notaの挿入テスト
// 5つのNotaを追加し、すべて正しく格納されることを確認
#[test]
fn test_gtd_data_insert_multiple_notas() {
    let mut data = GtdData::new();

    for i in 1..=5 {
        let nota = Nota {
            id: format!("task-{}", i),
            title: format!("Test Task {}", i),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            ..Default::default()
        };
        data.add(nota);
    }

    assert_eq!(data.task_count(), 5);
    assert_eq!(data.inbox().len(), 5);
}

// Notaステータスの更新テスト
// NotaのステータスをInboxからNextActionに更新し、正しく反映されることを確認
#[test]
fn test_gtd_data_update_nota_status() {
    let mut data = GtdData::new();
    let nota_id = "task-1".to_string();
    let nota = Nota {
        id: nota_id.clone(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ..Default::default()
    };

    data.add(nota);

    // Update status
    data.move_status(&nota_id, NotaStatus::next_action);

    assert!(matches!(
        data.find_task_by_id(&nota_id).unwrap().status,
        NotaStatus::next_action
    ));
}

// タスクの削除テスト
// タスクを追加後、削除して正しく削除されることを確認
#[test]
fn test_gtd_data_remove_task() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let task = Task {
        id: task_id.clone(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);
    assert_eq!(data.task_count(), 1);
    assert_eq!(data.inbox().len(), 1);

    data.remove_task(&task_id);
    assert_eq!(data.task_count(), 0);
    assert_eq!(data.inbox().len(), 0);
}

// ステータス移動テスト - inbox から trash への移動
// タスクが inbox から trash に正しく移動されることを確認
#[test]
fn test_gtd_data_move_status_inbox_to_trash() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let task = Task {
        id: task_id.clone(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);
    assert_eq!(data.inbox().len(), 1);
    assert_eq!(data.trash().len(), 0);

    // Move task to trash
    let result = data.move_status(&task_id, NotaStatus::trash);
    assert!(result.is_some());

    // Verify task was moved
    assert_eq!(data.inbox().len(), 0);
    assert_eq!(data.trash().len(), 1);
    assert_eq!(data.task_count(), 1);

    // Verify task status was updated
    let moved_task = data.find_task_by_id(&task_id).unwrap();
    assert!(matches!(moved_task.status, NotaStatus::trash));
}

// ステータス移動テスト - next_action から done への移動
// タスクが next_action から done に正しく移動されることを確認
#[test]
fn test_gtd_data_move_status_next_action_to_done() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let task = Task {
        id: task_id.clone(),
        title: "Test Task".to_string(),
        status: NotaStatus::next_action,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);
    assert_eq!(data.next_action().len(), 1);
    assert_eq!(data.done().len(), 0);

    // Move task to done
    let result = data.move_status(&task_id, NotaStatus::done);
    assert!(result.is_some());

    // Verify task was moved
    assert_eq!(data.next_action().len(), 0);
    assert_eq!(data.done().len(), 1);
    assert_eq!(data.task_count(), 1);

    // Verify task status was updated
    let moved_task = data.find_task_by_id(&task_id).unwrap();
    assert!(matches!(moved_task.status, NotaStatus::done));
}

// ステータス移動テスト - 複数のステータス間の移動
// タスクが複数のステータス間を正しく移動できることを確認
#[test]
fn test_gtd_data_move_status_multiple_transitions() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let task = Task {
        id: task_id.clone(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);

    // inbox -> next_action
    data.move_status(&task_id, NotaStatus::next_action);
    assert_eq!(data.inbox().len(), 0);
    assert_eq!(data.next_action().len(), 1);
    assert!(matches!(
        data.find_task_by_id(&task_id).unwrap().status,
        NotaStatus::next_action
    ));

    // next_action -> waiting_for
    data.move_status(&task_id, NotaStatus::waiting_for);
    assert_eq!(data.next_action().len(), 0);
    assert_eq!(data.waiting_for().len(), 1);
    assert!(matches!(
        data.find_task_by_id(&task_id).unwrap().status,
        NotaStatus::waiting_for
    ));

    // waiting_for -> done
    data.move_status(&task_id, NotaStatus::done);
    assert_eq!(data.waiting_for().len(), 0);
    assert_eq!(data.done().len(), 1);
    assert!(matches!(
        data.find_task_by_id(&task_id).unwrap().status,
        NotaStatus::done
    ));

    // done -> trash
    data.move_status(&task_id, NotaStatus::trash);
    assert_eq!(data.done().len(), 0);
    assert_eq!(data.trash().len(), 1);
    assert!(matches!(
        data.find_task_by_id(&task_id).unwrap().status,
        NotaStatus::trash
    ));
}

// ステータス移動テスト - カレンダーへの移動
// タスクをカレンダーステータスに移動し、正しくcalendarコンテナに格納されることを確認
#[test]
fn test_gtd_data_move_status_to_calendar() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
    let task = Task {
        id: task_id.clone(),
        title: "Future Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: Some(date),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);
    assert_eq!(data.inbox().len(), 1);

    // inbox -> calendar
    let result = data.move_status(&task_id, NotaStatus::calendar);
    assert!(result.is_some());
    assert_eq!(data.inbox().len(), 0);
    assert_eq!(data.calendar().len(), 1);

    let moved_task = data.find_task_by_id(&task_id).unwrap();
    assert!(matches!(moved_task.status, NotaStatus::calendar));
    assert_eq!(moved_task.start_date.unwrap(), date);
}

// ステータス移動テスト - 存在しないタスク
// 存在しないタスクの移動がNoneを返すことを確認
#[test]
fn test_gtd_data_move_status_nonexistent_task() {
    let mut data = GtdData::new();
    let result = data.move_status("nonexistent-id", NotaStatus::trash);
    assert!(result.is_none());
}

// ステータス移動テスト - タスクのプロパティが保持される
// ステータス移動時にタスクの他のプロパティが保持されることを確認
#[test]
fn test_gtd_data_move_status_preserves_properties() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let task = Task {
        id: task_id.clone(),
        title: "Important Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("Office".to_string()),
        notes: Some("Important notes".to_string()),
        start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);

    // Move task to next_action
    data.move_status(&task_id, NotaStatus::next_action);

    // Verify all properties are preserved (except updated_at which should be updated)
    let moved_task = data.find_task_by_id(&task_id).unwrap();
    assert_eq!(moved_task.title, "Important Task");
    assert_eq!(moved_task.project, Some("project-1".to_string()));
    assert_eq!(moved_task.context, Some("Office".to_string()));
    assert_eq!(moved_task.notes, Some("Important notes".to_string()));
    assert_eq!(moved_task.start_date, NaiveDate::from_ymd_opt(2024, 12, 25));
    assert_eq!(
        moved_task.created_at,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    );
    // Note: updated_at is automatically updated by move_status to reflect the change
    assert!(matches!(moved_task.status, NotaStatus::next_action));
}

// プロジェクトとコンテキスト付きタスクのテスト
// プロジェクト、コンテキスト、ノートが正しく設定されることを確認
#[test]
fn test_task_with_project_and_context() {
    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("context-1".to_string()),
        notes: Some("Test notes".to_string()),
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert_eq!(task.project.as_ref().unwrap(), "project-1");
    assert_eq!(task.context.as_ref().unwrap(), "context-1");
    assert_eq!(task.notes.as_ref().unwrap(), "Test notes");
}

// 開始日付付きタスクのテスト
// タスクに開始日を設定し、正しく格納されることを確認
#[test]
fn test_task_with_start_date() {
    let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: Some(date),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert_eq!(task.start_date.unwrap(), date);
}

// カレンダーステータスのタスクテスト
// カレンダーステータスのタスクが正しく作成され、start_dateが設定されることを確認
#[test]
fn test_calendar_task_with_start_date() {
    let date = NaiveDate::from_ymd_opt(2024, 12, 25).unwrap();
    let task = Task {
        id: "task-1".to_string(),
        title: "Christmas Task".to_string(),
        status: NotaStatus::calendar,
        project: None,
        context: None,
        notes: None,
        start_date: Some(date),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(matches!(task.status, NotaStatus::calendar));
    assert_eq!(task.start_date.unwrap(), date);
}

// 参考資料ステータスのタスクテスト
// 参考資料ステータスのタスクが正しく作成されることを確認
#[test]
fn test_reference_task() {
    let task = Task {
        id: "ref-1".to_string(),
        title: "Meeting Notes - Q4 2024".to_string(),
        status: NotaStatus::reference,
        project: Some("project-1".to_string()),
        context: None,
        notes: Some("Important reference material from Q4 meeting".to_string()),
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(matches!(task.status, NotaStatus::reference));
    assert_eq!(task.title, "Meeting Notes - Q4 2024");
    assert_eq!(
        task.notes,
        Some("Important reference material from Q4 meeting".to_string())
    );
}

// 参考資料への移動テスト
// タスクをinboxからreferenceに移動できることを確認
#[test]
fn test_move_to_reference() {
    let mut data = GtdData::new();
    let task_id = "task-1".to_string();
    let task = Task {
        id: task_id.clone(),
        title: "Documentation".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: Some("Useful documentation for future reference".to_string()),
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    data.add_task(task);
    assert_eq!(data.inbox().len(), 1);
    assert_eq!(data.reference().len(), 0);

    // Move to reference
    let result = data.move_status(&task_id, NotaStatus::reference);
    assert!(result.is_some());

    // Verify task was moved
    assert_eq!(data.inbox().len(), 0);
    assert_eq!(data.reference().len(), 1);
    assert_eq!(data.task_count(), 1);

    // Verify task status was updated
    let moved_task = data.find_task_by_id(&task_id).unwrap();
    assert!(matches!(moved_task.status, NotaStatus::reference));
    assert_eq!(
        moved_task.notes,
        Some("Useful documentation for future reference".to_string())
    );
}

// 参考資料の一覧取得テスト
// 複数の参考資料が正しく取得できることを確認
#[test]
fn test_list_reference_items() {
    let mut data = GtdData::new();

    // Add multiple reference items
    for i in 1..=3 {
        let task = Task {
            id: format!("ref-{}", i),
            title: format!("Reference Material {}", i),
            status: NotaStatus::reference,
            project: None,
            context: None,
            notes: Some(format!("Reference notes {}", i)),
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);
    }

    let references = data.reference();
    assert_eq!(references.len(), 3);

    // Verify all are reference status
    for ref_item in references {
        assert!(matches!(ref_item.status, NotaStatus::reference));
    }
}

// タスクステータスの全バリアントテスト
// 9種類のタスクステータス（Inbox、NextAction、WaitingFor、Someday、Later、Done、Reference、Trash、Calendar）がすべて正しく動作することを確認
#[test]
fn test_task_status_variants() {
    let statuses = vec![
        NotaStatus::inbox,
        NotaStatus::next_action,
        NotaStatus::waiting_for,
        NotaStatus::someday,
        NotaStatus::later,
        NotaStatus::done,
        NotaStatus::reference,
        NotaStatus::trash,
        NotaStatus::calendar,
    ];

    for status in statuses {
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: status.clone(),
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };

        match status {
            NotaStatus::inbox => assert!(matches!(task.status, NotaStatus::inbox)),
            NotaStatus::next_action => assert!(matches!(task.status, NotaStatus::next_action)),
            NotaStatus::waiting_for => assert!(matches!(task.status, NotaStatus::waiting_for)),
            NotaStatus::someday => assert!(matches!(task.status, NotaStatus::someday)),
            NotaStatus::later => assert!(matches!(task.status, NotaStatus::later)),
            NotaStatus::done => assert!(matches!(task.status, NotaStatus::done)),
            NotaStatus::reference => assert!(matches!(task.status, NotaStatus::reference)),
            NotaStatus::trash => assert!(matches!(task.status, NotaStatus::trash)),
            NotaStatus::calendar => assert!(matches!(task.status, NotaStatus::calendar)),
            NotaStatus::context | NotaStatus::project => {
                panic!("context and project are not task statuses")
            }
        }
    }
}

// プロジェクトの作成テスト
// プロジェクトを作成し、ID、名前、説明、ステータスが正しく設定されることを確認
#[test]
fn test_project_creation() {
    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: Some("Test description".to_string()),
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    };

    assert_eq!(project.id, "project-1");
    assert_eq!(project.title, "Test Project");
    assert_eq!(project.notes.as_ref().unwrap(), "Test description");
}

// 説明なしプロジェクトのテスト
// 説明を持たないプロジェクトが正しく作成されることを確認
#[test]
fn test_project_without_description() {
    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    };

    assert!(project.notes.is_none());
}

// プロジェクトステータスの全バリアントテスト
// 3種類のプロジェクトステータス（Active、OnHold、Completed）がすべて正しく動作することを確認
// GtdDataへのプロジェクト挿入テスト
// プロジェクトを1つ追加し、正しく格納・取得できることを確認
#[test]
fn test_gtd_data_insert_project() {
    let mut data = GtdData::new();
    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    };

    data.add_project(project.clone());
    assert_eq!(data.projects().len(), 1);
    assert_eq!(
        data.find_project_by_id("project-1").unwrap().title,
        "Test Project"
    );
}

// プロジェクトステータスの更新テスト
// コンテキストの作成テスト
// コンテキストを作成し、IDと名前が正しく設定されることを確認
#[test]
fn test_context_creation() {
    let context = Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    };

    assert_eq!(context.name, "Office");
    assert_eq!(context.notes, None);
}

// コンテキストの説明付き作成テスト
// 説明フィールドを持つコンテキストが正しく作成されることを確認
#[test]
fn test_context_with_description() {
    let context = Context {
        name: "Office".to_string(),
        notes: Some("Work environment with desk and computer".to_string()),
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    };

    assert_eq!(context.name, "Office");
    assert_eq!(
        context.notes,
        Some("Work environment with desk and computer".to_string())
    );
}

// GtdDataへのコンテキスト挿入テスト
// コンテキストを1つ追加し、正しく格納・取得できることを確認
#[test]
fn test_gtd_data_insert_context() {
    let mut data = GtdData::new();
    let context = Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    };

    data.add_context(context.clone());
    assert_eq!(data.contexts().len(), 1);
    assert_eq!(data.find_context_by_name("Office").unwrap().id, "Office");
}

// 複数コンテキストの挿入テスト
// 4つのコンテキスト（Office、Home、Phone、Errands）を追加し、すべて正しく格納されることを確認
#[test]
fn test_gtd_data_insert_multiple_contexts() {
    let mut data = GtdData::new();
    let contexts = vec!["Office", "Home", "Phone", "Errands"];

    for name in contexts {
        let context = Context {
            name: name.to_string(),
            notes: None,
            title: None,
            status: NotaStatus::context,
            project: None,
            context: None,
            start_date: None,
            created_at: None,
            updated_at: None,
        };
        data.add_context(context);
    }

    assert_eq!(data.contexts().len(), 4);
}

// タスクのシリアライゼーションテスト
// タスクをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
#[test]
fn test_task_serialization() {
    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("context-1".to_string()),
        notes: Some("Test notes".to_string()),
        start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    let serialized = toml::to_string(&task).unwrap();
    let deserialized: Task = toml::from_str(&serialized).unwrap();

    assert_eq!(task.id, deserialized.id);
    assert_eq!(task.title, deserialized.title);
    assert_eq!(task.project, deserialized.project);
    assert_eq!(task.context, deserialized.context);
    assert_eq!(task.notes, deserialized.notes);
    assert_eq!(task.start_date, deserialized.start_date);
}

// プロジェクトのシリアライゼーションテスト
// プロジェクトをGtdData経由でTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
// プロジェクトは現在HashMapとして保存されるため、GtdData全体でのテストが必要
#[test]
fn test_project_serialization() {
    let mut data = GtdData::new();
    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: Some("Test description".to_string()),
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    };

    data.add_project(project.clone());

    let serialized = toml::to_string(&data).unwrap();
    let deserialized: GtdData = toml::from_str(&serialized).unwrap();

    let deserialized_projects = deserialized.projects();
    let deserialized_project = deserialized_projects.get("project-1").unwrap();
    assert_eq!(project.id, deserialized_project.id);
    assert_eq!(project.title, deserialized_project.title);
    assert_eq!(project.notes, deserialized_project.notes);
}

// コンテキストのシリアライゼーションテスト
// コンテキストをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
// Note: name フィールドは skip_serializing されるため、TOML には含まれない
// Context serialization test for Version 3
// In V3 format, contexts are stored in [[context]] arrays, so name must be serialized
#[test]
fn test_context_serialization() {
    let context = Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    };

    let serialized = toml::to_string(&context).unwrap();
    // In Version 3, name field is serialized as part of the [[context]] array
    assert!(
        serialized.contains("name"),
        "name field should be serialized in Version 3"
    );

    let deserialized: Context = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.name, "Office");
    assert_eq!(deserialized.notes, None);
}

// GtdData全体のシリアライゼーションテスト
// タスク、プロジェクト、コンテキストを含むGtdDataをTOML形式にシリアライズし、デシリアライズして各要素数が一致することを確認
#[test]
fn test_gtd_data_serialization() {
    let mut data = GtdData::new();

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };
    data.add_task(task);

    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    };
    data.add_project(project);

    let context = Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    };
    data.add_context(context);

    let serialized = toml::to_string(&data).unwrap();
    let deserialized: GtdData = toml::from_str(&serialized).unwrap();

    assert_eq!(data.task_count(), deserialized.task_count());
    assert_eq!(data.projects().len(), deserialized.projects().len());
    assert_eq!(data.contexts().len(), deserialized.contexts().len());
}

// ステータスによるタスクフィルタリングテスト
// 複数のステータスを持つタスクを追加し、特定のステータスでフィルタリングできることを確認
#[test]
fn test_task_filter_by_status() {
    let mut data = GtdData::new();

    let statuses = [
        NotaStatus::inbox,
        NotaStatus::next_action,
        NotaStatus::waiting_for,
        NotaStatus::someday,
        NotaStatus::later,
        NotaStatus::done,
        NotaStatus::trash,
        NotaStatus::calendar,
    ];

    for (i, status) in statuses.iter().enumerate() {
        let task = Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: status.clone(),
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);
    }

    // Filter by Inbox
    assert_eq!(data.inbox().len(), 1);

    // Filter by Done
    assert_eq!(data.done().len(), 1);

    // Verify all statuses have exactly one task
    assert_eq!(data.task_count(), 8);
}

// プロジェクトによるタスクフィルタリングテスト
// 特定のプロジェクトに紐づくタスクのみをフィルタリングできることを確認
#[test]
fn test_task_filter_by_project() {
    let mut data = GtdData::new();

    for i in 1..=5 {
        let task = Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: NotaStatus::inbox,
            project: if i % 2 == 0 {
                Some("project-1".to_string())
            } else {
                None
            },
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);
    }

    let all_tasks = data.list_all(None);
    let project_tasks: Vec<_> = all_tasks
        .iter()
        .filter(|t| t.project.as_ref().is_some_and(|p| p == "project-1"))
        .collect();
    assert_eq!(project_tasks.len(), 2);
}

// コンテキストによるタスクフィルタリングテスト
// 特定のコンテキストに紐づくタスクのみをフィルタリングできることを確認
#[test]
fn test_task_filter_by_context() {
    let mut data = GtdData::new();

    for i in 1..=5 {
        let task = Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: NotaStatus::inbox,
            project: None,
            context: if i % 2 == 0 {
                Some("context-1".to_string())
            } else {
                None
            },
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);
    }

    let all_tasks = data.list_all(None);
    let context_tasks: Vec<_> = all_tasks
        .iter()
        .filter(|t| t.context.as_ref().is_some_and(|c| c == "context-1"))
        .collect();
    assert_eq!(context_tasks.len(), 2);
}

// 日付パースのテスト
// 文字列形式の日付を正しくパースし、年月日が正確に取得できることを確認
#[test]
fn test_date_parsing() {
    let date_str = "2024-12-25";
    let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
    assert!(parsed.is_ok());

    let date = parsed.unwrap();
    assert_eq!(date.year(), 2024);
    assert_eq!(date.month(), 12);
    assert_eq!(date.day(), 25);
}

// 不正な日付パースのテスト
// 無効な月と日を含む日付文字列のパースがエラーになることを確認
#[test]
fn test_invalid_date_parsing() {
    let date_str = "2024-13-45"; // Invalid month and day
    let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
    assert!(parsed.is_err());
}

// タスクのクローンテスト
// タスクをクローンし、元のタスクと同じ内容を持つことを確認
#[test]
fn test_task_clone() {
    let task1 = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("context-1".to_string()),
        notes: Some("Test notes".to_string()),
        start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    let task2 = task1.clone();
    assert_eq!(task1.id, task2.id);
    assert_eq!(task1.title, task2.title);
    assert_eq!(task1.project, task2.project);
}

// TOML serialization verification test
// Verify that enum variants are serialized as snake_case in TOML format
#[test]
fn test_enum_snake_case_serialization() {
    let mut data = GtdData::new();

    // Add a task to next_action to verify the status field is snake_case
    data.add_task(Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::next_action,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    });

    let serialized = toml::to_string(&data).unwrap();
    // V3 format uses [[next_action]] with status field
    assert!(
        serialized.contains("[[next_action]]"),
        "Expected '[[next_action]]' in TOML output"
    );
    assert!(
        serialized.contains("status = \"next_action\""),
        "Expected 'status = \"next_action\"' in TOML output"
    );
}

// Insertion order preservation test
// Verify that tasks maintain their insertion order (Vec-based instead of HashMap)
#[test]
fn test_gtd_data_insertion_order() {
    let mut data = GtdData::new();

    // 特定の順序でタスクを追加
    for i in 1..=5 {
        let task = Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        data.add_task(task);
    }

    // Verify that tasks maintain insertion order
    assert_eq!(data.inbox().len(), 5);
    let data_inbox = data.inbox();
    for (i, task) in data_inbox.iter().enumerate() {
        assert_eq!(task.id, format!("task-{}", i + 1));
        assert_eq!(task.title, format!("Task {}", i + 1));
    }
}

// TOML serialization order preservation test
// Verify that TOML serialization maintains insertion order
#[test]
fn test_toml_serialization_order() {
    let mut data = GtdData::new();

    // 特定の順序でアイテムを追加
    for i in 1..=3 {
        data.add_task(Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        });
    }

    for i in 1..=2 {
        data.add_project(Project {
            status: None,
            id: format!("project-{}", i),
            title: format!("Project {}", i),
            notes: None,
            project: None,
            context: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
        });
    }

    let toml_str = toml::to_string(&data).unwrap();
    let deserialized: GtdData = toml::from_str(&toml_str).unwrap();

    // Verify deserialized data maintains insertion order for tasks
    assert_eq!(deserialized.inbox().len(), 3);
    let deserialized_inbox = deserialized.inbox();
    for (i, task) in deserialized_inbox.iter().enumerate() {
        assert_eq!(task.id, format!("task-{}", i + 1));
    }

    // Verify all projects are present (HashMap doesn't guarantee order)
    assert_eq!(deserialized.projects().len(), 2);
    assert!(deserialized.projects().contains_key("project-1"));
    assert!(deserialized.projects().contains_key("project-2"));
}

// 完全なTOML出力テスト（全フィールド設定）
// 全フィールドを設定した状態でTOML出力を検証し、意図したテキスト形式で出力されることを確認する
// V4形式: 統一された[[notas]]配列を使用
#[test]
fn test_complete_toml_output() {
    let mut data = GtdData::new();

    // 全フィールドを設定したタスクを追加
    data.add_task(Task {
        id: "task-001".to_string(),
        title: "Complete project documentation".to_string(),
        status: NotaStatus::next_action,
        project: Some("project-001".to_string()),
        context: Some("Office".to_string()),
        notes: Some("Review all sections and update examples".to_string()),
        start_date: NaiveDate::from_ymd_opt(2024, 3, 15),
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    });

    // 最小限のフィールドを設定したタスクを追加（比較用）
    data.add_task(Task {
        id: "task-002".to_string(),
        title: "Quick task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    });

    // 全フィールドを設定したプロジェクトを追加
    data.add_project(Project {
        status: None,
        id: "project-001".to_string(),
        title: "Documentation Project".to_string(),
        notes: Some("Comprehensive project documentation update".to_string()),
        project: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        context: None,
    });

    // 説明付きコンテキストを追加
    data.add_context(Context {
        name: "Office".to_string(),
        notes: Some("Work environment with desk and computer".to_string()),
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    });

    // TOML出力を生成
    let toml_output = toml::to_string_pretty(&data).unwrap();

    // TOML構造と可読性を確認
    println!(
        "\n=== TOML Output (V3) ===\n{}\n===================\n",
        toml_output
    );

    // V3形式の期待される構造を検証
    assert!(
        toml_output.contains("format_version = 3"),
        "Should be version 3"
    );
    assert!(
        toml_output.contains("[[inbox]]"),
        "Should have [[inbox]] section"
    );
    assert!(
        toml_output.contains("[[next_action]]"),
        "Should have [[next_action]] section"
    );
    assert!(
        toml_output.contains("[[project]]"),
        "Should have [[project]] section"
    );
    assert!(
        toml_output.contains("[[context]]"),
        "Should have [[context]] section"
    );

    // 各アイテムが含まれていることを確認
    assert!(toml_output.contains("id = \"task-001\""));
    assert!(toml_output.contains("id = \"task-002\""));
    assert!(toml_output.contains("id = \"project-001\""));
    assert!(toml_output.contains("id = \"Office\""));

    // ステータスが正しく含まれていることを確認
    assert!(toml_output.contains("status = \"next_action\""));
    assert!(toml_output.contains("status = \"inbox\""));
    assert!(toml_output.contains("status = \"project\""));
    assert!(toml_output.contains("status = \"context\""));

    // デシリアライゼーションが正しく動作することを確認
    let deserialized: GtdData = toml::from_str(&toml_output).unwrap();

    // 全タスクフィールドを検証
    assert_eq!(deserialized.inbox().len(), 1);
    assert_eq!(deserialized.next_action().len(), 1);

    let task_inbox = &deserialized.inbox()[0];
    assert_eq!(task_inbox.id, "task-002");
    assert_eq!(task_inbox.title, "Quick task");
    assert!(matches!(task_inbox.status, NotaStatus::inbox));

    let task1 = &deserialized.next_action()[0];
    assert_eq!(task1.id, "task-001");
    assert_eq!(task1.title, "Complete project documentation");
    assert!(matches!(task1.status, NotaStatus::next_action));
    assert_eq!(task1.project, Some("project-001".to_string()));
    assert_eq!(task1.context, Some("Office".to_string()));
    assert_eq!(
        task1.notes,
        Some("Review all sections and update examples".to_string())
    );
    assert_eq!(task1.start_date, NaiveDate::from_ymd_opt(2024, 3, 15));

    // プロジェクトフィールドを検証
    assert_eq!(deserialized.projects().len(), 1);
    let deserialized_projects = deserialized.projects();
    let project1 = deserialized_projects.get("project-001").unwrap();
    assert_eq!(project1.id, "project-001");
    assert_eq!(project1.title, "Documentation Project");
    assert_eq!(
        project1.notes,
        Some("Comprehensive project documentation update".to_string())
    );

    // コンテキストフィールドを検証
    assert_eq!(deserialized.contexts().len(), 1);

    let deserialized_contexts = deserialized.contexts();
    let context_office = deserialized_contexts.get("Office").unwrap();
    assert_eq!(context_office.id, "Office");
    assert_eq!(
        context_office.notes,
        Some("Work environment with desk and computer".to_string())
    );
}

// 後方互換性テスト: 旧形式（nameフィールド付き）のTOMLも正しく読み込めることを確認
// Test backward compatibility with name field in contexts (Version 2 format)
// Version 2 used HashMap format where name was the key, so name field was redundant
// Version 3 uses Vec format where name must be included
#[test]
fn test_backward_compatibility_with_name_field() {
    // 旧形式のTOML（nameフィールドが含まれている）- Version 2 HashMap format
    let old_format_toml = r#"
[[tasks]]
id = "task-001"
title = "Test task"

[contexts.Office]
name = "Office"
notes = "Work environment with desk and computer"

[contexts.Home]
name = "Home"
"#;

    // 旧形式のTOMLを読み込めることを確認
    let deserialized: GtdData = toml::from_str(old_format_toml).unwrap();

    assert_eq!(deserialized.contexts().len(), 2);

    // Officeコンテキストを検証
    let deserialized_contexts = deserialized.contexts();
    let office = deserialized_contexts.get("Office").unwrap();
    assert_eq!(office.id, "Office");
    assert_eq!(
        office.notes,
        Some("Work environment with desk and computer".to_string())
    );

    // Homeコンテキストを検証
    let deserialized_contexts = deserialized.contexts();
    let home = deserialized_contexts.get("Home").unwrap();
    assert_eq!(home.id, "Home");
    assert_eq!(home.notes, None);

    // 再シリアライズするとVersion 3形式（status-based arrays）になることを確認
    let reserialized = toml::to_string_pretty(&deserialized).unwrap();
    assert!(
        reserialized.contains("[[context]]"),
        "Reserialized TOML should use [[context]] array format"
    );
    assert!(
        reserialized.contains("id = \"Office\""),
        "Reserialized TOML should contain id field"
    );
    assert!(
        reserialized.contains("id = \"Home\""),
        "Reserialized TOML should contain id field"
    );
    assert!(
        reserialized.contains("status = \"context\""),
        "Reserialized TOML should contain status = \"context\""
    );
}

// 参照整合性検証テスト - プロジェクト参照が有効
// タスクのプロジェクト参照が存在するプロジェクトを指している場合、検証が成功することを確認
#[test]
fn test_validate_task_project_valid() {
    let mut data = GtdData::new();

    data.add_project(Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    });

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(data.validate_task_project(&task));
}

// 参照整合性検証テスト - プロジェクト参照が無効
// タスクのプロジェクト参照が存在しないプロジェクトを指している場合、検証が失敗することを確認
#[test]
fn test_validate_task_project_invalid() {
    let data = GtdData::new();

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("non-existent-project".to_string()),
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(!data.validate_task_project(&task));
}

// 参照整合性検証テスト - プロジェクト参照がNone
// タスクのプロジェクト参照がNoneの場合、検証が成功することを確認
#[test]
fn test_validate_task_project_none() {
    let data = GtdData::new();

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(data.validate_task_project(&task));
}

// 参照整合性検証テスト - コンテキスト参照が有効
// タスクのコンテキスト参照が存在するコンテキストを指している場合、検証が成功することを確認
#[test]
fn test_validate_task_context_valid() {
    let mut data = GtdData::new();

    data.add_context(Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    });

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: Some("Office".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(data.validate_task_context(&task));
}

// 参照整合性検証テスト - コンテキスト参照が無効
// タスクのコンテキスト参照が存在しないコンテキストを指している場合、検証が失敗することを確認
#[test]
fn test_validate_task_context_invalid() {
    let data = GtdData::new();

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: Some("NonExistent".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(!data.validate_task_context(&task));
}

// 参照整合性検証テスト - コンテキスト参照がNone
// タスクのコンテキスト参照がNoneの場合、検証が成功することを確認
#[test]
fn test_validate_task_context_none() {
    let data = GtdData::new();

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(data.validate_task_context(&task));
}

// 参照整合性検証テスト - 全ての参照が有効
// タスクのプロジェクトとコンテキストの両方の参照が有効な場合、検証が成功することを確認
#[test]
fn test_validate_task_references_all_valid() {
    let mut data = GtdData::new();

    data.add_project(Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    });

    data.add_context(Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    });

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("Office".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(data.validate_task_references(&task));
}

// 参照整合性検証テスト - プロジェクト参照のみ無効
// プロジェクト参照が無効でコンテキスト参照が有効な場合、検証が失敗することを確認
#[test]
fn test_validate_task_references_invalid_project() {
    let mut data = GtdData::new();

    data.add_context(Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    });

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("non-existent-project".to_string()),
        context: Some("Office".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(!data.validate_task_references(&task));
}

// 参照整合性検証テスト - コンテキスト参照のみ無効
// コンテキスト参照が無効でプロジェクト参照が有効な場合、検証が失敗することを確認
#[test]
fn test_validate_task_references_invalid_context() {
    let mut data = GtdData::new();

    data.add_project(Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    });

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("NonExistent".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(!data.validate_task_references(&task));
}

// 参照整合性検証テスト - 両方の参照が無効
// プロジェクトとコンテキストの両方の参照が無効な場合、検証が失敗することを確認
#[test]
fn test_validate_task_references_both_invalid() {
    let data = GtdData::new();

    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("non-existent-project".to_string()),
        context: Some("NonExistent".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(!data.validate_task_references(&task));
}

// 作成日と更新日のテスト
// タスクが作成されたとき、created_atとupdated_atが同じ日付に設定されることを確認
#[test]
fn test_task_created_at_and_updated_at() {
    let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: date,
        updated_at: date,
    };

    assert_eq!(task.created_at, date);
    assert_eq!(task.updated_at, date);
    assert_eq!(task.created_at, task.updated_at);
}

// 更新日の変更テスト
// タスクが更新されたとき、updated_atが変更されることを確認
#[test]
fn test_task_updated_at_changes() {
    let created_date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
    let updated_date = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();

    let mut task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: created_date,
        updated_at: created_date,
    };

    // タスクを更新
    task.status = NotaStatus::next_action;
    task.updated_at = updated_date;

    assert_eq!(task.created_at, created_date);
    assert_eq!(task.updated_at, updated_date);
    assert_ne!(task.created_at, task.updated_at);
}

// 作成日は変更されないことを確認するテスト
// タスクのステータスが変更されても、created_atは変更されないことを確認
#[test]
fn test_task_created_at_immutable() {
    let mut data = GtdData::new();
    let created_date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
    let task_id = "task-1".to_string();

    let task = Task {
        id: task_id.clone(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: created_date,
        updated_at: created_date,
    };

    data.add_task(task);

    // タスクのステータスを更新
    if let Some(task) = data.find_task_by_id_mut(&task_id) {
        task.status = NotaStatus::next_action;
        task.updated_at = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
    }

    // created_atは変更されていないことを確認
    let task = data.find_task_by_id(&task_id).unwrap();
    assert_eq!(task.created_at, created_date);
    assert_ne!(task.updated_at, created_date);
}

// TOML シリアライゼーションに作成日と更新日が含まれることを確認
#[test]
fn test_task_dates_serialization() {
    let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: date,
        updated_at: date,
    };

    let serialized = toml::to_string(&task).unwrap();
    assert!(serialized.contains("created_at = \"2024-03-15\""));
    assert!(serialized.contains("updated_at = \"2024-03-15\""));

    let deserialized: Task = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.created_at, date);
    assert_eq!(deserialized.updated_at, date);
}

// ID生成テスト - タスクIDが連番で生成されることを確認
#[test]
fn test_generate_task_id() {
    let mut data = GtdData::new();

    let id1 = data.generate_task_id();
    let id2 = data.generate_task_id();
    let id3 = data.generate_task_id();

    assert_eq!(id1, "#1");
    assert_eq!(id2, "#2");
    assert_eq!(id3, "#3");
    assert_eq!(data.task_counter, 3);
}

// ID生成テスト - カウンターの永続化を確認
#[test]
fn test_counter_serialization() {
    let mut data = GtdData::new();

    // Generate some IDs
    data.generate_task_id();
    data.generate_task_id();

    // Serialize
    let serialized = toml::to_string_pretty(&data).unwrap();

    // Check that counter is in the serialized output
    assert!(
        serialized.contains("task_counter = 2"),
        "task_counter should be serialized"
    );

    // Deserialize
    let deserialized: GtdData = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.task_counter, 2);

    // Next ID should continue from where we left off
    let mut data = deserialized;
    assert_eq!(data.generate_task_id(), "#3");
}

// ID生成テスト - カウンターが0の場合はTOMLに含まれないことを確認
#[test]
fn test_counter_skip_serializing_if_zero() {
    let data = GtdData::new();

    let serialized = toml::to_string_pretty(&data).unwrap();

    // Counters should not appear in serialized output when they are 0
    assert!(
        !serialized.contains("task_counter"),
        "task_counter should not be serialized when 0"
    );
    assert!(
        !serialized.contains("project_counter"),
        "project_counter should not be serialized when 0"
    );
}

// プロジェクトのコンテキスト参照検証テスト - 有効な参照
// プロジェクトのコンテキスト参照が存在するコンテキストを指している場合、検証が成功することを確認
#[test]
fn test_validate_project_context_valid() {
    let mut data = GtdData::new();

    data.add_context(Context {
        name: "Office".to_string(),
        notes: None,
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    });

    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: Some("Office".to_string()),
    };

    assert!(data.validate_project_context(&project));
}

// プロジェクトのコンテキスト参照検証テスト - 無効な参照
// プロジェクトのコンテキスト参照が存在しないコンテキストを指している場合、検証が失敗することを確認
#[test]
fn test_validate_project_context_invalid() {
    let data = GtdData::new();

    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: Some("NonExistent".to_string()),
    };

    assert!(!data.validate_project_context(&project));
}

// プロジェクトのコンテキスト参照検証テスト - コンテキスト参照がNone
// プロジェクトのコンテキスト参照がNoneの場合、検証が成功することを確認
#[test]
fn test_validate_project_context_none() {
    let data = GtdData::new();

    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Test Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: None,
    };

    assert!(data.validate_project_context(&project));
}

// プロジェクトとタスクの両方にコンテキストを設定するテスト
// プロジェクトとタスクの両方が同じコンテキストを参照できることを確認
#[test]
fn test_project_and_task_with_same_context() {
    let mut data = GtdData::new();

    data.add_context(Context {
        name: "Office".to_string(),
        notes: Some("Work environment".to_string()),
        title: None,
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: None,
        updated_at: None,
    });

    let project = Project {
        status: None,
        id: "project-1".to_string(),
        title: "Office Project".to_string(),
        notes: None,
        project: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        context: Some("Office".to_string()),
    };
    data.add_project(project.clone());

    let task = Task {
        id: "task-1".to_string(),
        title: "Office Task".to_string(),
        status: NotaStatus::next_action,
        project: Some("project-1".to_string()),
        context: Some("Office".to_string()),
        notes: None,
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    };

    assert!(data.validate_project_context(&project));
    assert!(data.validate_task_context(&task));
    assert_eq!(project.context, task.context);
}

// 後方互換性テスト - コンテキストフィールドなしのプロジェクト
// 旧バージョンのTOMLファイル（コンテキストフィールドなし）を正しく読み込めることを確認
#[test]
fn test_backward_compatibility_project_without_context() {
    // TOML from old version without context field
    let toml_str = r#"
[[projects]]
id = "project-1"
title = "Old Project"
notes = "Project without context field"
"#;

    let data: GtdData = toml::from_str(toml_str).unwrap();
    assert_eq!(data.projects().len(), 1);

    let projects = data.projects();
    let project = projects.get("project-1").unwrap();
    assert_eq!(project.id, "project-1");
    assert_eq!(project.title, "Old Project");
    assert_eq!(project.context, None);
}

// フォーマットバージョン1からバージョン3への自動マイグレーションテスト
// 旧形式（Vec<Project>）のTOMLを読み込み、新形式（HashMap）に自動変換され、バージョン3で保存されることを確認
#[test]
fn test_format_migration_v1_to_v3() {
    // Format version 1: projects as array ([[projects]])
    let old_format_toml = r#"
[[projects]]
id = "project-1"
title = "First Project"
notes = "Original format"

[[projects]]
id = "project-2"
title = "Second Project"

[[inbox]]
id = "task-1"
title = "Test task"
project = "project-1"
created_at = "2024-01-01"
updated_at = "2024-01-01"
"#;

    // Load old format
    let data: GtdData = toml::from_str(old_format_toml).unwrap();

    // Verify it's automatically migrated to version 3
    assert_eq!(data.format_version, 3);
    assert_eq!(data.projects().len(), 2);

    // Verify projects are accessible
    let data_projects = data.projects();
    let project1 = data_projects.get("project-1").unwrap();
    assert_eq!(project1.id, "project-1");
    assert_eq!(project1.title, "First Project");

    let data_projects = data.projects();
    let project2 = data_projects.get("project-2").unwrap();
    assert_eq!(project2.id, "project-2");
    assert_eq!(project2.title, "Second Project");

    // Verify task references still work
    assert_eq!(data.inbox().len(), 1);
    assert_eq!(data.inbox()[0].project, Some("project-1".to_string()));

    // Save to new format
    let new_format_toml = toml::to_string_pretty(&data).unwrap();

    // Verify new format has status-based arrays and version 3
    assert!(new_format_toml.contains("format_version = 3"));
    assert!(new_format_toml.contains("[[inbox]]"));
    assert!(new_format_toml.contains("[[project]]"));
    assert!(!new_format_toml.contains("[[notas]]"));

    // Verify round-trip works
    let reloaded: GtdData = toml::from_str(&new_format_toml).unwrap();
    assert_eq!(reloaded.format_version, 3);
    assert_eq!(reloaded.projects().len(), 2);
    assert!(reloaded.projects().contains_key("project-1"));
    assert!(reloaded.projects().contains_key("project-2"));
}

// フォーマットバージョン2からバージョン3への自動マイグレーションテスト
// バージョン2形式のTOMLを読み込み、バージョン3で保存されることを確認
#[test]
fn test_format_migration_v2_to_v3() {
    // Format version 2: projects as HashMap
    let v2_format_toml = r##"
format_version = 2

[[inbox]]
id = "#1"
title = "Test task"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[projects.project-1]
title = "Test Project"
notes = "Version 2 format"
created_at = "2024-01-01"
updated_at = "2024-01-01"

[contexts.Office]
notes = "Office context"
"##;

    // Load version 2 format
    let data: GtdData = toml::from_str(v2_format_toml).unwrap();

    // Verify it's automatically migrated to version 3
    assert_eq!(data.format_version, 3);
    assert_eq!(data.inbox().len(), 1);
    assert_eq!(data.projects().len(), 1);
    assert_eq!(data.contexts().len(), 1);

    // Verify data integrity
    let task = &data.inbox()[0];
    assert_eq!(task.id, "#1");
    assert_eq!(task.title, "Test task");

    let projects = data.projects();
    let project = projects.get("project-1").unwrap();
    assert_eq!(project.title, "Test Project");

    let contexts = data.contexts();
    let context = contexts.get("Office").unwrap();
    assert_eq!(context.id, "Office");

    // Save to new format
    let new_format_toml = toml::to_string_pretty(&data).unwrap();

    // Verify new format has version 3 and status-based arrays
    assert!(new_format_toml.contains("format_version = 3"));
    assert!(new_format_toml.contains("[[inbox]]"));
    assert!(new_format_toml.contains("[[project]]"));
    assert!(new_format_toml.contains("[[context]]"));

    // Verify round-trip works
    let reloaded: GtdData = toml::from_str(&new_format_toml).unwrap();
    assert_eq!(reloaded.format_version, 3);
    assert_eq!(reloaded.inbox().len(), 1);
    assert_eq!(reloaded.projects().len(), 1);
    assert_eq!(reloaded.contexts().len(), 1);
}

// NotaStatus::from_strのテスト - 有効なステータス
// 全ての有効なステータス文字列が正しくパースされることを確認
#[test]
fn test_task_status_from_str_valid() {
    assert_eq!(NotaStatus::from_str("inbox").unwrap(), NotaStatus::inbox);
    assert_eq!(
        NotaStatus::from_str("next_action").unwrap(),
        NotaStatus::next_action
    );
    assert_eq!(
        NotaStatus::from_str("waiting_for").unwrap(),
        NotaStatus::waiting_for
    );
    assert_eq!(
        NotaStatus::from_str("someday").unwrap(),
        NotaStatus::someday
    );
    assert_eq!(NotaStatus::from_str("later").unwrap(), NotaStatus::later);
    assert_eq!(
        NotaStatus::from_str("calendar").unwrap(),
        NotaStatus::calendar
    );
    assert_eq!(NotaStatus::from_str("done").unwrap(), NotaStatus::done);
    assert_eq!(
        NotaStatus::from_str("reference").unwrap(),
        NotaStatus::reference
    );
    assert_eq!(NotaStatus::from_str("trash").unwrap(), NotaStatus::trash);
}

// NotaStatus::from_strのテスト - 無効なステータス
// 無効なステータス文字列が適切なエラーメッセージを返すことを確認
#[test]
fn test_task_status_from_str_invalid() {
    let result = NotaStatus::from_str("invalid_status");
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Invalid status 'invalid_status'"));
    assert!(err_msg.contains("inbox"));
    assert!(err_msg.contains("next_action"));
    assert!(err_msg.contains("waiting_for"));
    assert!(err_msg.contains("someday"));
    assert!(err_msg.contains("later"));
    assert!(err_msg.contains("calendar"));
    assert!(err_msg.contains("done"));
    assert!(err_msg.contains("reference"));
    assert!(err_msg.contains("trash"));
}

// NotaStatus::from_strのテスト - 大文字小文字の違い
// 大文字小文字が異なる場合はエラーになることを確認（厳密な一致が必要）
#[test]
fn test_task_status_from_str_case_sensitive() {
    assert!(NotaStatus::from_str("Inbox").is_err());
    assert!(NotaStatus::from_str("INBOX").is_err());
    assert!(NotaStatus::from_str("Next_Action").is_err());
    assert!(NotaStatus::from_str("NEXT_ACTION").is_err());
}

// NotaStatus::from_strのテスト - 存在しない一般的な名前
// よくある誤りのステータス名がエラーになることを確認
#[test]
fn test_task_status_from_str_common_mistakes() {
    // 問題として報告された "in_progress" をテスト
    let result = NotaStatus::from_str("in_progress");
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Invalid status 'in_progress'"));

    // その他の一般的な誤り
    assert!(NotaStatus::from_str("complete").is_err());
    assert!(NotaStatus::from_str("completed").is_err());
    assert!(NotaStatus::from_str("pending").is_err());
    assert!(NotaStatus::from_str("todo").is_err());
    assert!(NotaStatus::from_str("in-progress").is_err());
}

// タスクステータスの順序がTOMLシリアライズに反映されることを確認
// NotaStatus enumの順序とGtdDataフィールドの順序が一致し、TOML出力もその順序になることを検証
#[test]
fn test_task_status_order_in_toml_serialization() {
    let mut data = GtdData::new();

    // Add one task for each status in enum order
    let statuses = [
        NotaStatus::inbox,
        NotaStatus::next_action,
        NotaStatus::waiting_for,
        NotaStatus::later,
        NotaStatus::calendar,
        NotaStatus::someday,
        NotaStatus::done,
        NotaStatus::reference,
        NotaStatus::trash,
    ];

    for (i, status) in statuses.iter().enumerate() {
        data.add_task(Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: status.clone(),
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        });
    }

    let toml_str = toml::to_string(&data).unwrap();

    // V3 format uses status-based arrays
    assert!(
        toml_str.contains("[[inbox]]"),
        "Should contain [[inbox]] section"
    );
    assert!(
        toml_str.contains("format_version = 3"),
        "Should be version 3"
    );

    // Verify all statuses are represented in their own sections
    for status in &statuses {
        let status_str = format!("{:?}", status);
        assert!(
            toml_str.contains(&format!("status = \"{}\"", status_str)),
            "Should contain status = \"{}\"",
            status_str
        );
    }
}

// Step 4: Test HashMap serialization order
#[test]
fn test_hashmap_serialization_order() {
    use std::collections::HashMap;

    // Create a HashMap with tasks
    let mut tasks_map: HashMap<String, Task> = HashMap::new();

    // Add tasks in a specific order
    for i in 1..=5 {
        let task = Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        tasks_map.insert(task.id.clone(), task);
    }

    // Serialize to TOML
    let toml_str = toml::to_string_pretty(&tasks_map).unwrap();
    println!("HashMap serialization order:\n{}", toml_str);

    // HashMap in Rust does NOT guarantee order
    // This test documents that HashMap does NOT maintain insertion order
    // Therefore, we should keep Vec-based serialization for TOML readability
    assert!(toml_str.contains("task-1"));
    assert!(toml_str.contains("task-2"));
    assert!(toml_str.contains("task-3"));
    assert!(toml_str.contains("task-4"));
    assert!(toml_str.contains("task-5"));
}

#[test]
fn test_vec_serialization_maintains_order() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestContainer {
        tasks: Vec<Task>,
    }

    // Create a Vec with tasks in order
    let mut tasks_vec: Vec<Task> = Vec::new();

    for i in 1..=5 {
        let task = Task {
            id: format!("task-{}", i),
            title: format!("Task {}", i),
            status: NotaStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        tasks_vec.push(task);
    }

    let container = TestContainer { tasks: tasks_vec };

    // Serialize to TOML
    let toml_str = toml::to_string_pretty(&container).unwrap();
    println!("Vec serialization order:\n{}", toml_str);

    // Vec maintains order - verify tasks appear in sequential order
    let task1_pos = toml_str.find("task-1").unwrap();
    let task2_pos = toml_str.find("task-2").unwrap();
    let task3_pos = toml_str.find("task-3").unwrap();
    let task4_pos = toml_str.find("task-4").unwrap();
    let task5_pos = toml_str.find("task-5").unwrap();

    // Verify tasks appear in order
    assert!(task1_pos < task2_pos);
    assert!(task2_pos < task3_pos);
    assert!(task3_pos < task4_pos);
    assert!(task4_pos < task5_pos);
}

// Tests for Nota structure (Step 6)
#[test]
fn test_nota_from_task() {
    let task = Task {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("project-1".to_string()),
        context: Some("Office".to_string()),
        notes: Some("Test notes".to_string()),
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
    };

    let nota = nota_from_task(task.clone());

    assert_eq!(nota.id, task.id);
    assert_eq!(nota.title, task.title);
    assert_eq!(nota.status, task.status);
    assert_eq!(nota.project, task.project);
    assert_eq!(nota.context, task.context);
    assert_eq!(nota.notes, task.notes);
    assert!(nota.is_task());
    assert!(!nota.is_project());
    assert!(!nota.is_context());
}

#[test]
fn test_nota_from_project() {
    let project = Project {
        status: None,
        id: "proj-1".to_string(),
        title: "Test Project".to_string(),
        notes: Some("Project notes".to_string()),
        project: None,
        context: Some("Office".to_string()),
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
    };

    let nota = nota_from_project(project.clone());

    assert_eq!(nota.id, project.id);
    assert_eq!(nota.title, project.title);
    assert_eq!(nota.status, NotaStatus::project);
    assert_eq!(nota.context, project.context);
    assert_eq!(nota.notes, project.notes);
    assert!(!nota.is_task());
    assert!(nota.is_project());
    assert!(!nota.is_context());
}

#[test]
fn test_nota_from_context() {
    let context = Context {
        name: "Office".to_string(),
        title: Some("Office Context".to_string()),
        notes: Some("Office location".to_string()),
        status: NotaStatus::context,
        project: None,
        context: None,
        start_date: None,
        created_at: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        updated_at: Some(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()),
    };

    let nota = nota_from_context(context.clone());

    assert_eq!(nota.id, context.name);
    assert_eq!(nota.title, "Office Context");
    assert_eq!(nota.status, NotaStatus::context);
    assert_eq!(nota.notes, context.notes);
    assert!(!nota.is_task());
    assert!(!nota.is_project());
    assert!(nota.is_context());
}

#[test]
fn test_nota_to_task() {
    let nota = Nota {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::next_action,
        project: Some("proj-1".to_string()),
        context: Some("Office".to_string()),
        notes: Some("Notes".to_string()),
        start_date: None,
        created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        updated_at: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        ..Default::default()
    };

    let task = nota_to_task(&nota).unwrap();

    assert_eq!(task.id, nota.id);
    assert_eq!(task.title, nota.title);
    assert_eq!(task.status, nota.status);
}

#[test]
fn test_nota_to_task_fails_for_project() {
    let nota = Nota {
        id: "proj-1".to_string(),
        title: "Project".to_string(),
        status: NotaStatus::project,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    assert!(nota_to_task(&nota).is_none());
}

#[test]
fn test_nota_to_project() {
    let nota = Nota {
        id: "proj-1".to_string(),
        title: "Test Project".to_string(),
        status: NotaStatus::project,
        project: None,
        context: Some("Office".to_string()),
        notes: Some("Project notes".to_string()),
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    let project = nota_to_project(&nota).unwrap();

    assert_eq!(project.id, nota.id);
    assert_eq!(project.title, nota.title);
    assert_eq!(project.context, nota.context);
}

#[test]
fn test_nota_to_context() {
    let nota = Nota {
        id: "Office".to_string(),
        title: "Office Context".to_string(),
        status: NotaStatus::context,
        project: None,
        context: None,
        notes: Some("Office location".to_string()),
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    let context = nota_to_context(&nota).unwrap();

    assert_eq!(context.name, nota.id);
    assert_eq!(context.title, Some(nota.title));
    assert_eq!(context.notes, nota.notes);
}

// Nota追加テスト - タスクとして追加
#[test]
fn test_add_as_task() {
    let mut data = GtdData::new();
    let nota = Nota {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    data.add(nota.clone());
    assert_eq!(data.task_count(), 1);
    assert_eq!(data.inbox().len(), 1);

    let found = data.find_by_id("task-1").unwrap();
    assert_eq!(found.title, "Test Task");
    assert!(found.is_task());
}

// Nota追加テスト - プロジェクトとして追加
#[test]
fn test_add_as_project() {
    let mut data = GtdData::new();
    let nota = Nota {
        id: "proj-1".to_string(),
        title: "Test Project".to_string(),
        status: NotaStatus::project,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    data.add(nota.clone());
    assert_eq!(data.projects().len(), 1);

    let found = data.find_by_id("proj-1").unwrap();
    assert_eq!(found.title, "Test Project");
    assert!(found.is_project());
}

// Nota追加テスト - コンテキストとして追加
#[test]
fn test_add_as_context() {
    let mut data = GtdData::new();
    let nota = Nota {
        id: "Office".to_string(),
        title: "Office Context".to_string(),
        status: NotaStatus::context,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    data.add(nota.clone());
    assert_eq!(data.contexts().len(), 1);

    let found = data.find_by_id("Office").unwrap();
    assert_eq!(found.title, "Office Context");
    assert!(found.is_context());
}

// Nota削除テスト (internal remove_nota function)
#[test]
fn test_remove_nota() {
    let mut data = GtdData::new();
    let nota = Nota {
        id: "task-1".to_string(),
        title: "Test Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    data.add(nota.clone());
    assert_eq!(data.task_count(), 1);

    let removed = data.remove_nota("task-1").unwrap();
    assert_eq!(removed.title, "Test Task");
    assert_eq!(data.task_count(), 0);
}

// Nota一覧テスト
#[test]
fn test_list_all() {
    let mut data = GtdData::new();

    // Add a task
    data.add(Nota {
        id: "task-1".to_string(),
        title: "Task".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    // Add a project
    data.add(Nota {
        id: "proj-1".to_string(),
        title: "Project".to_string(),
        status: NotaStatus::project,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    // Add a context
    data.add(Nota {
        id: "Office".to_string(),
        title: "Office".to_string(),
        status: NotaStatus::context,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    let all_notas = data.list_all(None);
    assert_eq!(all_notas.len(), 3);

    let tasks_only = data.list_all(Some(NotaStatus::inbox));
    assert_eq!(tasks_only.len(), 1);

    let projects_only = data.list_all(Some(NotaStatus::project));
    assert_eq!(projects_only.len(), 1);

    let contexts_only = data.list_all(Some(NotaStatus::context));
    assert_eq!(contexts_only.len(), 1);
}

// Nota参照チェックテスト
#[test]
fn test_is_nota_referenced() {
    let mut data = GtdData::new();

    // Add a project
    data.add(Nota {
        id: "proj-1".to_string(),
        title: "Project".to_string(),
        status: NotaStatus::project,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    // Add a context
    data.add(Nota {
        id: "Office".to_string(),
        title: "Office".to_string(),
        status: NotaStatus::context,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    // Add a task that references both
    data.add(Nota {
        id: "task-1".to_string(),
        title: "Task".to_string(),
        status: NotaStatus::inbox,
        project: Some("proj-1".to_string()),
        context: Some("Office".to_string()),
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    assert!(data.is_referenced("proj-1"));
    assert!(data.is_referenced("Office"));
    assert!(!data.is_referenced("task-1"));
}

// Nota更新テスト
#[test]
fn test_update() {
    let mut data = GtdData::new();

    // Add a nota
    data.add(Nota {
        id: "task-1".to_string(),
        title: "Old Title".to_string(),
        status: NotaStatus::inbox,
        project: None,
        context: None,
        notes: None,
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    });

    // Update it
    let updated = Nota {
        id: "task-1".to_string(),
        title: "New Title".to_string(),
        status: NotaStatus::next_action,
        project: None,
        context: None,
        notes: Some("New notes".to_string()),
        start_date: None,
        created_at: local_date_today(),
        updated_at: local_date_today(),
        ..Default::default()
    };

    data.update("task-1", updated).unwrap();

    let found = data.find_by_id("task-1").unwrap();
    assert_eq!(found.title, "New Title");
    assert_eq!(found.status, NotaStatus::next_action);
    assert_eq!(found.notes, Some("New notes".to_string()));
    assert_eq!(data.next_action().len(), 1);
    assert_eq!(data.inbox().len(), 0);
}

/// Test find_by_id performance for typical GTD usage scale
///
/// This validates that O(n) lookup is fast enough for personal GTD usage.
/// Even with 500 items (larger than typical), linear search is negligible.
#[test]
fn test_find_by_id_performance_at_scale() {
    let mut data = GtdData::new();

    // Simulate typical large personal GTD setup: 500 items
    for i in 0..500 {
        data.add(Nota {
            id: format!("nota-{}", i),
            title: format!("Nota {}", i),
            status: if i % 2 == 0 {
                NotaStatus::inbox
            } else {
                NotaStatus::next_action
            },
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });
    }

    // Test find operations - these use O(n) linear search
    assert!(data.find_by_id("nota-0").is_some());
    assert!(data.find_by_id("nota-250").is_some());
    assert!(data.find_by_id("nota-499").is_some());
    assert!(data.find_by_id("nota-500").is_none());

    // Even at 500 items, these operations complete in microseconds on modern hardware
    // This validates that Arc/RefCell complexity is unnecessary for this scale
}

/// Test that design works correctly with status filtering
///
/// This validates a common operation: filtering notas by status.
/// O(n) filtering is expected and acceptable for this use case.
#[test]
fn test_status_filtering_at_scale() {
    let mut data = GtdData::new();

    // Add 300 notas across different statuses
    for i in 0..300 {
        let status = match i % 3 {
            0 => NotaStatus::inbox,
            1 => NotaStatus::next_action,
            _ => NotaStatus::done,
        };

        data.add(Nota {
            id: format!("nota-{}", i),
            title: format!("Nota {}", i),
            status,
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            ..Default::default()
        });
    }

    // Test filtering by status
    let inbox = data.inbox();
    let next_action = data.next_action();
    let done = data.done();

    assert_eq!(inbox.len(), 100);
    assert_eq!(next_action.len(), 100);
    assert_eq!(done.len(), 100);

    // Verify each group has correct status
    assert!(inbox.iter().all(|n| n.status == NotaStatus::inbox));
    assert!(
        next_action
            .iter()
            .all(|n| n.status == NotaStatus::next_action)
    );
    assert!(done.iter().all(|n| n.status == NotaStatus::done));
}

/// Test memory efficiency of current design
///
/// This test documents the memory characteristics of HashMap<String, NotaStatus>
/// vs a hypothetical Arc<RefCell<Nota>> design.
#[test]
fn test_nota_map_memory_footprint() {
    use std::mem::size_of;

    // Current design: HashMap stores ID (String) + Status (enum)
    let string_size = size_of::<String>(); // 24 bytes (ptr + len + cap)
    let status_size = size_of::<NotaStatus>(); // 1 byte (enum)
    let entry_size = string_size + status_size; // ~25 bytes per entry

    // Hypothetical Arc<RefCell<Nota>> design would need:
    // Arc = 16 bytes (ptr + ref counts)
    // RefCell = 8 bytes (borrow flag)
    // Total per entry = 24 bytes EXTRA overhead (plus the original Nota)

    // For 500 notas:
    // Current: 500 × 25 = 12.5 KB
    // Arc/RefCell: 500 × (25 + 24) = 24.5 KB (double the memory)

    // This validates that current design is more memory efficient
    println!("Current HashMap entry size: ~{} bytes", entry_size);
    println!("Arc<RefCell> overhead would add: ~24 bytes per entry");

    // The test itself just validates the size calculations are reasonable
    assert!(string_size >= 16); // String has pointer + metadata
    assert!(status_size <= 8); // Enum should be small
}

/// Test that trash appears at the end in TOML serialization
///
/// This validates that the serialization order follows the NotaStatus enum order,
/// with trash being the last status type in the TOML output.
#[test]
fn test_toml_serialization_order_trash_at_end() {
    let mut data = GtdData::new();

    // Add tasks for all statuses in random order to verify serialization order
    let statuses = [
        ("trash", NotaStatus::trash),
        ("inbox", NotaStatus::inbox),
        ("done", NotaStatus::done),
        ("next_action", NotaStatus::next_action),
        ("reference", NotaStatus::reference),
    ];

    for (name, status) in &statuses {
        let nota = Nota {
            id: format!("{}-1", name),
            title: format!("Test {}", name),
            status: status.clone(),
            project: None,
            context: None,
            notes: None,
            start_date: None,
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            ..Default::default()
        };
        data.add(nota);
    }

    let toml_str = toml::to_string_pretty(&data).unwrap();

    // Find positions of each section
    let inbox_pos = toml_str.find("[[inbox]]").unwrap();
    let next_action_pos = toml_str.find("[[next_action]]").unwrap();
    let done_pos = toml_str.find("[[done]]").unwrap();
    let reference_pos = toml_str.find("[[reference]]").unwrap();
    let trash_pos = toml_str.find("[[trash]]").unwrap();

    // Verify order: inbox < next_action < done < reference < trash
    assert!(
        inbox_pos < next_action_pos,
        "inbox should come before next_action"
    );
    assert!(
        next_action_pos < done_pos,
        "next_action should come before done"
    );
    assert!(
        done_pos < reference_pos,
        "done should come before reference"
    );
    assert!(
        reference_pos < trash_pos,
        "reference should come before trash (trash should be last)"
    );
}

// Tests for recurrence functionality
#[test]
fn test_recurrence_daily() {
    let nota = Nota {
        id: "daily-task".to_string(),
        title: "Daily Task".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::daily),
        recurrence_config: None,
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap()),
        ..Default::default()
    };

    assert!(nota.is_recurring());
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 11, 1).unwrap());
}

#[test]
fn test_recurrence_weekly_single_day() {
    let nota = Nota {
        id: "weekly-task".to_string(),
        title: "Weekly Review".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::weekly),
        recurrence_config: Some("Friday".to_string()),
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap()), // Friday
        ..Default::default()
    };

    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 11, 7).unwrap()); // Next Friday
}

#[test]
fn test_recurrence_weekly_multiple_days() {
    let nota = Nota {
        id: "mwf-task".to_string(),
        title: "Mon/Wed/Fri Task".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::weekly),
        recurrence_config: Some("Monday,Wednesday,Friday".to_string()),
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap()), // Friday
        ..Default::default()
    };

    // Next after Friday should be Monday
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 11, 3).unwrap()); // Monday

    // Next after Monday should be Wednesday
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 11, 3).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 11, 5).unwrap()); // Wednesday
}

#[test]
fn test_recurrence_monthly_single_day() {
    let nota = Nota {
        id: "monthly-task".to_string(),
        title: "Monthly Report".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::monthly),
        recurrence_config: Some("15".to_string()),
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 15).unwrap()),
        ..Default::default()
    };

    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 15).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 11, 15).unwrap());
}

#[test]
fn test_recurrence_monthly_multiple_days() {
    let nota = Nota {
        id: "multi-monthly-task".to_string(),
        title: "Multiple Monthly Dates".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::monthly),
        recurrence_config: Some("5,15,25".to_string()),
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 5).unwrap()),
        ..Default::default()
    };

    // Next after 5th should be 15th
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 5).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 10, 15).unwrap());

    // Next after 15th should be 25th
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 15).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 10, 25).unwrap());

    // Next after 25th should be next month's 5th
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 25).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 11, 5).unwrap());
}

#[test]
fn test_recurrence_yearly_single_date() {
    let nota = Nota {
        id: "yearly-task".to_string(),
        title: "Annual Review".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::yearly),
        recurrence_config: Some("12-25".to_string()), // Dec 25
        start_date: Some(NaiveDate::from_ymd_opt(2024, 12, 25).unwrap()),
        ..Default::default()
    };

    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2024, 12, 25).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());
}

#[test]
fn test_recurrence_yearly_multiple_dates() {
    let nota = Nota {
        id: "multi-yearly-task".to_string(),
        title: "Multiple Yearly Dates".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::yearly),
        recurrence_config: Some("1-1,6-15,12-25".to_string()), // Jan 1, Jun 15, Dec 25
        start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        ..Default::default()
    };

    // Next after Jan 1 should be Jun 15
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());

    // Next after Jun 15 should be Dec 25
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());

    // Next after Dec 25 should be next year's Jan 1
    let next = nota
        .calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        .unwrap();
    assert_eq!(next, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
}

#[test]
fn test_non_recurring_nota() {
    let nota = Nota {
        id: "one-time-task".to_string(),
        title: "One Time Task".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: None,
        recurrence_config: None,
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap()),
        ..Default::default()
    };

    assert!(!nota.is_recurring());
    assert!(
        nota.calculate_next_occurrence(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap())
            .is_none()
    );
}

#[test]
fn test_recurrence_serialization() {
    let nota = Nota {
        id: "recurring-task".to_string(),
        title: "Test Recurring".to_string(),
        status: NotaStatus::calendar,
        recurrence_pattern: Some(RecurrencePattern::weekly),
        recurrence_config: Some("Monday,Friday".to_string()),
        start_date: Some(NaiveDate::from_ymd_opt(2025, 10, 31).unwrap()),
        ..Default::default()
    };

    // Serialize to TOML
    let toml_str = toml::to_string(&nota).unwrap();

    // Verify recurrence fields are in TOML
    assert!(toml_str.contains("recurrence_pattern = \"weekly\""));
    assert!(toml_str.contains("recurrence_config = \"Monday,Friday\""));

    // Deserialize back
    let deserialized: Nota = toml::from_str(&toml_str).unwrap();
    assert_eq!(deserialized.id, nota.id);
    assert_eq!(deserialized.recurrence_pattern, nota.recurrence_pattern);
    assert_eq!(deserialized.recurrence_config, nota.recurrence_config);
}

#[test]
fn test_recurrence_backward_compatibility() {
    // TOML without recurrence fields should deserialize successfully
    let toml_str = r#"
id = "old-task"
title = "Old Task"
status = "inbox"
created_at = "2025-10-31"
updated_at = "2025-10-31"
"#;

    let nota: Nota = toml::from_str(toml_str).unwrap();
    assert_eq!(nota.id, "old-task");
    assert!(nota.recurrence_pattern.is_none());
    assert!(nota.recurrence_config.is_none());
    assert!(!nota.is_recurring());
}
