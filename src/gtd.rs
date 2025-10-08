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

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    inbox,
    next_action,
    waiting_for,
    someday,
    done,
    trash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    active,
    on_hold,
    completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GtdData {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub contexts: HashMap<String, Context>,
}

impl GtdData {
    pub fn new() -> Self {
        Self::default()
    }

    // Helper methods for task operations
    #[allow(dead_code)]
    pub fn find_task_by_id(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn find_task_by_id_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn remove_task(&mut self, id: &str) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    // Helper methods for project operations
    #[allow(dead_code)]
    pub fn find_project_by_id(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    #[allow(dead_code)]
    pub fn find_project_by_id_mut(&mut self, id: &str) -> Option<&mut Project> {
        self.projects.iter_mut().find(|p| p.id == id)
    }

    pub fn add_project(&mut self, project: Project) {
        self.projects.push(project);
    }

    // Helper methods for context operations
    #[allow(dead_code)]
    pub fn find_context_by_name(&self, name: &str) -> Option<&Context> {
        self.contexts.get(name)
    }

    #[allow(dead_code)]
    pub fn add_context(&mut self, context: Context) {
        self.contexts.insert(context.name.clone(), context);
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

        data.add_task(task.clone());
        assert_eq!(data.tasks.len(), 1);
        assert_eq!(data.find_task_by_id("task-1").unwrap().title, "Test Task");
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
            data.add_task(task);
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

        data.add_task(task);

        // Update status
        if let Some(task) = data.find_task_by_id_mut(&task_id) {
            task.status = TaskStatus::next_action;
        }

        assert!(matches!(
            data.find_task_by_id(&task_id).unwrap().status,
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

        data.add_task(task);
        assert_eq!(data.tasks.len(), 1);

        data.remove_task(&task_id);
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

        data.add_project(project.clone());
        assert_eq!(data.projects.len(), 1);
        assert_eq!(data.find_project_by_id("project-1").unwrap().name, "Test Project");
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

        data.add_project(project);

        // Update status
        if let Some(project) = data.find_project_by_id_mut(&project_id) {
            project.status = ProjectStatus::completed;
        }

        assert!(matches!(
            data.find_project_by_id(&project_id).unwrap().status,
            ProjectStatus::completed
        ));
    }

    // コンテキストの作成テスト
    // コンテキストを作成し、IDと名前が正しく設定されることを確認
    #[test]
    fn test_context_creation() {
        let context = Context {
            name: "Office".to_string(),
        };

        assert_eq!(context.name, "Office");
    }

    // GtdDataへのコンテキスト挿入テスト
    // コンテキストを1つ追加し、正しく格納・取得できることを確認
    #[test]
    fn test_gtd_data_insert_context() {
        let mut data = GtdData::new();
        let context = Context {
            name: "Office".to_string(),
        };

        data.add_context(context.clone());
        assert_eq!(data.contexts.len(), 1);
        assert_eq!(data.find_context_by_name("Office").unwrap().name, "Office");
    }

    // 複数コンテキストの挿入テスト
    // 4つのコンテキスト（Office、Home、Phone、Errands）を追加し、すべて正しく格納されることを確認
    #[test]
    fn test_gtd_data_insert_multiple_contexts() {
        let mut data = GtdData::new();
        let contexts = vec![
            "Office",
            "Home",
            "Phone",
            "Errands",
        ];

        for name in contexts {
            let context = Context {
                name: name.to_string(),
            };
            data.add_context(context);
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
            name: "Office".to_string(),
        };

        let serialized = toml::to_string(&context).unwrap();
        let deserialized: Context = toml::from_str(&serialized).unwrap();

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
        data.add_task(task);

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::active,
        };
        data.add_project(project);

        let context = Context {
            name: "Office".to_string(),
        };
        data.add_context(context);

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
            data.add_task(task);
        }

        // Filter by Inbox
        let inbox_tasks: Vec<_> = data
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::inbox))
            .collect();
        assert_eq!(inbox_tasks.len(), 1);

        // Filter by Done
        let done_tasks: Vec<_> = data
            .tasks
            .iter()
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
            data.add_task(task);
        }

        let project_tasks: Vec<_> = data
            .tasks
            .iter()
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
            data.add_task(task);
        }

        let context_tasks: Vec<_> = data
            .tasks
            .iter()
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
        assert!(
            serialized.contains("next_action"),
            "Expected 'next_action' in TOML output"
        );

        let project = Project {
            id: "project-1".to_string(),
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::on_hold,
        };

        let serialized = toml::to_string(&project).unwrap();
        assert!(
            serialized.contains("on_hold"),
            "Expected 'on_hold' in TOML output"
        );
    }

    // Insertion order preservation test
    // Verify that tasks maintain their insertion order (Vec-based instead of HashMap)
    #[test]
    fn test_gtd_data_insertion_order() {
        let mut data = GtdData::new();

        // Add tasks in specific order
        for i in 1..=5 {
            let task = Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: TaskStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
            };
            data.add_task(task);
        }

        // Verify that tasks maintain insertion order
        assert_eq!(data.tasks.len(), 5);
        for (i, task) in data.tasks.iter().enumerate() {
            assert_eq!(task.id, format!("task-{}", i + 1));
            assert_eq!(task.title, format!("Task {}", i + 1));
        }
    }

    // TOML serialization order preservation test
    // Verify that TOML serialization maintains insertion order
    #[test]
    fn test_toml_serialization_order() {
        let mut data = GtdData::new();

        // Add items in specific order
        for i in 1..=3 {
            data.add_task(Task {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                status: TaskStatus::inbox,
                project: None,
                context: None,
                notes: None,
                start_date: None,
            });
        }

        for i in 1..=2 {
            data.add_project(Project {
                id: format!("project-{}", i),
                name: format!("Project {}", i),
                description: None,
                status: ProjectStatus::active,
            });
        }

        let toml_str = toml::to_string(&data).unwrap();
        let deserialized: GtdData = toml::from_str(&toml_str).unwrap();

        // Verify deserialized data maintains insertion order
        assert_eq!(deserialized.tasks.len(), 3);
        for (i, task) in deserialized.tasks.iter().enumerate() {
            assert_eq!(task.id, format!("task-{}", i + 1));
        }

        assert_eq!(deserialized.projects.len(), 2);
        for (i, project) in deserialized.projects.iter().enumerate() {
            assert_eq!(project.id, format!("project-{}", i + 1));
        }
    }

    // Complete TOML output test with all fields populated
    // Verify the actual TOML text output with all fields set and check readability
    #[test]
    fn test_complete_toml_output() {
        let mut data = GtdData::new();

        // Add a task with all fields populated
        data.add_task(Task {
            id: "task-001".to_string(),
            title: "Complete project documentation".to_string(),
            status: TaskStatus::next_action,
            project: Some("project-001".to_string()),
            context: Some("context-001".to_string()),
            notes: Some("Review all sections and update examples".to_string()),
            start_date: NaiveDate::from_ymd_opt(2024, 3, 15),
        });

        // Add a task with minimal fields for comparison
        data.add_task(Task {
            id: "task-002".to_string(),
            title: "Quick task".to_string(),
            status: TaskStatus::inbox,
            project: None,
            context: None,
            notes: None,
            start_date: None,
        });

        // Add a project with all fields
        data.add_project(Project {
            id: "project-001".to_string(),
            name: "Documentation Project".to_string(),
            description: Some("Comprehensive project documentation update".to_string()),
            status: ProjectStatus::active,
        });

        // Add a context
        data.add_context(Context {
            name: "Office".to_string(),
        });

        // Generate TOML output
        let toml_output = toml::to_string_pretty(&data).unwrap();

        // Verify the TOML structure and readability
        println!("\n=== TOML Output ===\n{}\n===================\n", toml_output);

        // Expected TOML structure (with exact text matching)
        let expected_toml = r#"[[tasks]]
id = "task-001"
title = "Complete project documentation"
status = "next_action"
project = "project-001"
context = "context-001"
notes = "Review all sections and update examples"
start_date = "2024-03-15"

[[tasks]]
id = "task-002"
title = "Quick task"
status = "inbox"

[[projects]]
id = "project-001"
name = "Documentation Project"
description = "Comprehensive project documentation update"
status = "active"

[contexts.Office]
name = "Office"
"#;

        // Assert exact TOML output matches expected format
        assert_eq!(toml_output, expected_toml, "TOML output should match expected format");

        // Verify deserialization works correctly
        let deserialized: GtdData = toml::from_str(&toml_output).unwrap();

        // Verify all task fields
        assert_eq!(deserialized.tasks.len(), 2);
        let task1 = &deserialized.tasks[0];
        assert_eq!(task1.id, "task-001");
        assert_eq!(task1.title, "Complete project documentation");
        assert!(matches!(task1.status, TaskStatus::next_action));
        assert_eq!(task1.project, Some("project-001".to_string()));
        assert_eq!(task1.context, Some("context-001".to_string()));
        assert_eq!(task1.notes, Some("Review all sections and update examples".to_string()));
        assert_eq!(task1.start_date, NaiveDate::from_ymd_opt(2024, 3, 15));

        // Verify project fields
        assert_eq!(deserialized.projects.len(), 1);
        let project1 = &deserialized.projects[0];
        assert_eq!(project1.id, "project-001");
        assert_eq!(project1.name, "Documentation Project");
        assert_eq!(project1.description, Some("Comprehensive project documentation update".to_string()));
        assert!(matches!(project1.status, ProjectStatus::active));

        // Verify context fields
        assert_eq!(deserialized.contexts.len(), 1);
        let context1 = deserialized.contexts.get("Office").unwrap();
        assert_eq!(context1.name, "Office");
    }
}
