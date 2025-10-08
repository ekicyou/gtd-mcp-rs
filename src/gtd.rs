use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub project: Option<String>,
    pub context: Option<String>,
    pub notes: Option<String>,
    pub start_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    #[allow(non_camel_case_types)]
    inbox,
    #[allow(non_camel_case_types)]
    next_action,
    #[allow(non_camel_case_types)]
    waiting_for,
    #[allow(non_camel_case_types)]
    someday,
    #[allow(non_camel_case_types)]
    done,
    #[allow(non_camel_case_types)]
    trash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    #[allow(non_camel_case_types)]
    active,
    #[allow(non_camel_case_types)]
    on_hold,
    #[allow(non_camel_case_types)]
    completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GtdData {
    pub tasks: HashMap<String, Task>,
    pub projects: HashMap<String, Project>,
    pub contexts: HashMap<String, Context>,
}

impl GtdData {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate};

    // GtdDataの新規作成テスト
    // 空のタスク、プロジェクト、コンテキストのHashMapが初期化されることを確認
    #[test]
    fn test_gtd_data_new() {
        let data = GtdData::new();
        assert!(data.tasks.is_empty());
        assert!(data.projects.is_empty());
        assert!(data.contexts.is_empty());
    }

    // GtdDataへのタスク挿入テスト
    // タスクを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_task() {
        let mut data = GtdData::new();
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };

        data.tasks.insert(task.id.clone(), task.clone());
        assert_eq!(data.tasks.len(), 1);
        assert_eq!(data.tasks.get("task-1").unwrap().title, "Test Task");
    }

    // 複数タスクの挿入テスト
    // 5つのタスクを追加し、すべて正しく格納されることを確認
    #[test]
    fn test_gtd_data_insert_multiple_tasks() {
        let mut data = GtdData::new();

        for i in 1..=5 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Test Task {}", i),
                status: TaskStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
            };
            data.tasks.insert(task.id.clone(), task);
        }

        assert_eq!(data.tasks.len(), 5);
    }

    // タスクステータスの更新テスト
    // タスクのステータスをInboxからNextActionに更新し、正しく反映されることを確認
    #[test]
    fn test_gtd_data_update_task_status() {
        let mut data = GtdData::new();
        let task_id = "task-1".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };

        data.tasks.insert(task_id.clone(), task);

        // Update status
        if let Some(task) = data.tasks.get_mut(&task_id) {
            task.status = TaskStatus::next_action;
        }

        assert!(matches!(
            data.tasks.get(&task_id).unwrap().status,
            TaskStatus::next_action
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
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };

        data.tasks.insert(task_id.clone(), task);
        assert_eq!(data.tasks.len(), 1);

        data.tasks.remove(&task_id);
        assert_eq!(data.tasks.len(), 0);
    }

    // プロジェクトとコンテキスト付きタスクのテスト
    // プロジェクト、コンテキスト、ノートが正しく設定されることを確認
    #[test]
    fn test_task_with_project_and_context() {
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("context-1".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: None,
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
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: Some(date),
        };

        assert_eq!(task.start_date.unwrap(), date);
    }

    // タスクステータスの全バリアントテスト
    // 6種類のタスクステータス（Inbox、NextAction、WaitingFor、Someday、Done、Trash）がすべて正しく動作することを確認
    #[test]
    fn test_task_status_variants() {
        let statuses = vec![
            TaskStatus::inbox,
            TaskStatus::next_action,
            TaskStatus::waiting_for,
            TaskStatus::someday,
            TaskStatus::done,
            TaskStatus::trash,
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
            };

            match status {
                TaskStatus::inbox => assert!(matches!(task.status, TaskStatus::inbox)),
                TaskStatus::next_action => assert!(matches!(task.status, TaskStatus::next_action)),
                TaskStatus::waiting_for => assert!(matches!(task.status, TaskStatus::waiting_for)),
                TaskStatus::someday => assert!(matches!(task.status, TaskStatus::someday)),
                TaskStatus::done => assert!(matches!(task.status, TaskStatus::done)),
                TaskStatus::trash => assert!(matches!(task.status, TaskStatus::trash)),
            }
        }
    }

    // プロジェクトの作成テスト
    // プロジェクトを作成し、ID、名前、説明、ステータスが正しく設定されることを確認
    #[test]
    fn test_project_creation() {
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: Some("Test description".to_string()),
            status: ProjectStatus::active,
        };

        assert_eq!(project.id, "project-1");
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description.as_ref().unwrap(), "Test description");
        assert!(matches!(project.status, ProjectStatus::active));
    }

    // 説明なしプロジェクトのテスト
    // 説明を持たないプロジェクトが正しく作成されることを確認
    #[test]
    fn test_project_without_description() {
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
        };

        assert!(project.description.is_none());
    }

    // プロジェクトステータスの全バリアントテスト
    // 3種類のプロジェクトステータス（Active、OnHold、Completed）がすべて正しく動作することを確認
    #[test]
    fn test_project_status_variants() {
        let statuses = vec![
            ProjectStatus::active,
            ProjectStatus::on_hold,
            ProjectStatus::completed,
        ];

        for status in statuses {
            let project = Project {
                id: "project-1".to_string(),
                name: "Test Project".to_string(),
                description: None,
                status: status.clone(),
            };

            match status {
                ProjectStatus::active => assert!(matches!(project.status, ProjectStatus::active)),
                ProjectStatus::on_hold => assert!(matches!(project.status, ProjectStatus::on_hold)),
                ProjectStatus::completed => {
                    assert!(matches!(project.status, ProjectStatus::completed))
                }
            }
        }
    }

    // GtdDataへのプロジェクト挿入テスト
    // プロジェクトを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_project() {
        let mut data = GtdData::new();
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
        };

        data.projects.insert(project.id.clone(), project.clone());
        assert_eq!(data.projects.len(), 1);
        assert_eq!(data.projects.get("project-1").unwrap().name, "Test Project");
    }

    // プロジェクトステータスの更新テスト
    // プロジェクトのステータスをActiveからCompletedに更新し、正しく反映されることを確認
    #[test]
    fn test_gtd_data_update_project_status() {
        let mut data = GtdData::new();
        let project_id = "project-1".to_string();
        let project = Project {
            id: project_id.clone(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
        };

        data.projects.insert(project_id.clone(), project);

        // Update status
        if let Some(project) = data.projects.get_mut(&project_id) {
            project.status = ProjectStatus::completed;
        }

        assert!(matches!(
            data.projects.get(&project_id).unwrap().status,
            ProjectStatus::completed
        ));
    }

    // コンテキストの作成テスト
    // コンテキストを作成し、IDと名前が正しく設定されることを確認
    #[test]
    fn test_context_creation() {
        let context = Context {
            id: "context-1".to_string(),
            name: "Office".to_string(),
        };

        assert_eq!(context.id, "context-1");
        assert_eq!(context.name, "Office");
    }

    // GtdDataへのコンテキスト挿入テスト
    // コンテキストを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_context() {
        let mut data = GtdData::new();
        let context = Context {
            id: "context-1".to_string(),
            name: "Office".to_string(),
        };

        data.contexts.insert(context.id.clone(), context.clone());
        assert_eq!(data.contexts.len(), 1);
        assert_eq!(data.contexts.get("context-1").unwrap().name, "Office");
    }

    // 複数コンテキストの挿入テスト
    // 4つのコンテキスト（Office、Home、Phone、Errands）を追加し、すべて正しく格納されることを確認
    #[test]
    fn test_gtd_data_insert_multiple_contexts() {
        let mut data = GtdData::new();
        let contexts = vec![
            ("ctx-1", "Office"),
            ("ctx-2", "Home"),
            ("ctx-3", "Phone"),
            ("ctx-4", "Errands"),
        ];

        for (id, name) in contexts {
            let context = Context {
                id: id.to_string(),
                name: name.to_string(),
            };
            data.contexts.insert(context.id.clone(), context);
        }

        assert_eq!(data.contexts.len(), 4);
    }

    // タスクのシリアライゼーションテスト
    // タスクをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    #[test]
    fn test_task_serialization() {
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("context-1".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
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
    // プロジェクトをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    #[test]
    fn test_project_serialization() {
        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: Some("Test description".to_string()),
            status: ProjectStatus::active,
        };

        let serialized = toml::to_string(&project).unwrap();
        let deserialized: Project = toml::from_str(&serialized).unwrap();

        assert_eq!(project.id, deserialized.id);
        assert_eq!(project.name, deserialized.name);
        assert_eq!(project.description, deserialized.description);
    }

    // コンテキストのシリアライゼーションテスト
    // コンテキストをTOML形式にシリアライズし、デシリアライズして元のデータと一致することを確認
    #[test]
    fn test_context_serialization() {
        let context = Context {
            id: "context-1".to_string(),
            name: "Office".to_string(),
        };

        let serialized = toml::to_string(&context).unwrap();
        let deserialized: Context = toml::from_str(&serialized).unwrap();

        assert_eq!(context.id, deserialized.id);
        assert_eq!(context.name, deserialized.name);
    }

    // GtdData全体のシリアライゼーションテスト
    // タスク、プロジェクト、コンテキストを含むGtdDataをTOML形式にシリアライズし、デシリアライズして各要素数が一致することを確認
    #[test]
    fn test_gtd_data_serialization() {
        let mut data = GtdData::new();

        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };
        data.tasks.insert(task.id.clone(), task);

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
        };
        data.projects.insert(project.id.clone(), project);

        let context = Context {
            id: "context-1".to_string(),
            name: "Office".to_string(),
        };
        data.contexts.insert(context.id.clone(), context);

        let serialized = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&serialized).unwrap();

        assert_eq!(data.tasks.len(), deserialized.tasks.len());
        assert_eq!(data.projects.len(), deserialized.projects.len());
        assert_eq!(data.contexts.len(), deserialized.contexts.len());
    }

    // ステータスによるタスクフィルタリングテスト
    // 複数のステータスを持つタスクを追加し、特定のステータスでフィルタリングできることを確認
    #[test]
    fn test_task_filter_by_status() {
        let mut data = GtdData::new();

        let statuses = vec![
            TaskStatus::inbox,
            TaskStatus::next_action,
            TaskStatus::waiting_for,
            TaskStatus::someday,
            TaskStatus::done,
            TaskStatus::trash,
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
            };
            data.tasks.insert(task.id.clone(), task);
        }

        // Filter by Inbox
        let inbox_tasks: Vec<_> = data
            .tasks
            .values()
            .filter(|t| matches!(t.status, TaskStatus::inbox))
            .collect();
        assert_eq!(inbox_tasks.len(), 1);

        // Filter by Done
        let done_tasks: Vec<_> = data
            .tasks
            .values()
            .filter(|t| matches!(t.status, TaskStatus::done))
            .collect();
        assert_eq!(done_tasks.len(), 1);
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
                status: TaskStatus::inbox,
                project: if i % 2 == 0 {
                    Some("project-1".to_string())
                } else {
                    None
                },
                context: None,
                notes: None,
                start_date: None,
            };
            data.tasks.insert(task.id.clone(), task);
        }

        let project_tasks: Vec<_> = data
            .tasks
            .values()
            .filter(|t| t.project.as_ref().map_or(false, |p| p == "project-1"))
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
                status: TaskStatus::inbox,
                project: None,
                context: if i % 2 == 0 {
                    Some("context-1".to_string())
                } else {
                    None
                },
                notes: None,
                start_date: None,
            };
            data.tasks.insert(task.id.clone(), task);
        }

        let context_tasks: Vec<_> = data
            .tasks
            .values()
            .filter(|t| t.context.as_ref().map_or(false, |c| c == "context-1"))
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
            status: TaskStatus::inbox,
            project: Some("project-1".to_string()),
            context: Some("context-1".to_string()),
            notes: Some("Test notes".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 12, 25),
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
        let task = Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::next_action,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        };

        let serialized = toml::to_string(&task).unwrap();
        assert!(serialized.contains("next_action"), "Expected 'next_action' in TOML output");

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::on_hold,
        };

        let serialized = toml::to_string(&project).unwrap();
        assert!(serialized.contains("on_hold"), "Expected 'on_hold' in TOML output");
    }
}
