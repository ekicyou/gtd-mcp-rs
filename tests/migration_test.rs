//! Unit tests for Migration (data migration functions)
//!
//! These tests verify the migration functions for upgrading data structures
//! between different schema versions.

use gtd_mcp::migration::local_date_today;
use gtd_mcp::migration::{
    Project, migrate_projects_v1_to_v2, normalize_string_line_endings, populate_project_ids,
};
use serde::Deserialize;
use std::collections::HashMap;

#[test]
fn test_migrate_projects_v1_to_v2() {
    let projects_vec = vec![
        Project {
            id: "project-1".to_string(),
            title: "First Project".to_string(),
            notes: Some("Notes 1".to_string()),
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
            status: None,
        },
        Project {
            id: "project-2".to_string(),
            title: "Second Project".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: Some("Office".to_string()),
            status: None,
        },
    ];

    let projects_map = migrate_projects_v1_to_v2(projects_vec);

    assert_eq!(projects_map.len(), 2);
    assert!(projects_map.contains_key("project-1"));
    assert!(projects_map.contains_key("project-2"));

    let project1 = &projects_map["project-1"];
    assert_eq!(project1.title, "First Project");
    assert_eq!(project1.notes, Some("Notes 1".to_string()));
}

#[test]
fn test_normalize_string_line_endings() {
    assert_eq!(
        normalize_string_line_endings("hello\r\nworld"),
        "hello\nworld"
    );
    assert_eq!(
        normalize_string_line_endings("hello\rworld"),
        "hello\nworld"
    );
    assert_eq!(
        normalize_string_line_endings("hello\nworld"),
        "hello\nworld"
    );
    assert_eq!(
        normalize_string_line_endings("line1\r\nline2\rline3\nline4"),
        "line1\nline2\nline3\nline4"
    );
}

#[test]
fn test_populate_project_ids() {
    let mut projects = HashMap::new();
    projects.insert(
        "proj-1".to_string(),
        Project {
            id: String::new(), // ID is empty before population
            title: "Test".to_string(),
            notes: None,
            project: None,
            start_date: None,
            created_at: local_date_today(),
            updated_at: local_date_today(),
            context: None,
            status: None,
        },
    );

    populate_project_ids(&mut projects);

    assert_eq!(projects["proj-1"].id, "proj-1");
}

#[test]
fn test_project_legacy_field_names() {
    // Test that "name" is accepted as alias for "title"
    let toml_with_name = r#"
[projects.test-proj]
name = "Legacy Project Name"
description = "Legacy description field"
status = "active"
context = "Office"
"#;

    // Need to wrap in a struct that has projects field
    #[derive(Deserialize)]
    struct Wrapper {
        projects: std::collections::HashMap<String, Project>,
    }

    let wrapper: Wrapper = toml::from_str(toml_with_name).unwrap();
    let mut data = wrapper.projects;

    populate_project_ids(&mut data);
    let project = &data["test-proj"];
    assert_eq!(project.id, "test-proj");
    assert_eq!(project.title, "Legacy Project Name");
    assert_eq!(project.notes, Some("Legacy description field".to_string()));
    assert_eq!(project.context, Some("Office".to_string()));
}

#[test]
fn test_project_legacy_and_new_fields_mixed() {
    // Test that both old and new field names work
    let toml_mixed = r#"
[projects.old-style]
name = "Old Style Project"
description = "Old description"

[projects.new-style]
title = "New Style Project"
notes = "New notes"
"#;

    #[derive(Deserialize)]
    struct Wrapper {
        projects: std::collections::HashMap<String, Project>,
    }

    let wrapper: Wrapper = toml::from_str(toml_mixed).unwrap();
    let mut data = wrapper.projects;
    populate_project_ids(&mut data);

    let old_project = &data["old-style"];
    assert_eq!(old_project.title, "Old Style Project");
    assert_eq!(old_project.notes, Some("Old description".to_string()));

    let new_project = &data["new-style"];
    assert_eq!(new_project.title, "New Style Project");
    assert_eq!(new_project.notes, Some("New notes".to_string()));
}

#[test]
fn test_user_toml_format_reproducer() {
    // Reproduce the exact error from the issue
    let user_toml = r##"
format_version = 2
task_counter = 62
project_counter = 4

[[next_action]]
id = "#13"
title = "4章を攻略する"
project = "FFT"
context = "自宅"
created_at = "2025-10-10"
updated_at = "2025-10-10"

[projects.rust-gui]
name = "Rust GUI フレームワーク開発"
description = "RustでGUIフレームワークを作成するプロジェクト"
status = "active"
context = "自宅"

[projects.ECI]
name = "ECI"
description = "仕事のソフトウェア開発プロジェクト"
status = "active"
context = "仕事"
"##;

    // This should not fail with "data did not match any variant of untagged enum ProjectsFormat"
    let result: Result<gtd_mcp::gtd::GtdData, _> = toml::from_str(user_toml);
    assert!(
        result.is_ok(),
        "Failed to parse user's TOML: {:?}",
        result.err()
    );

    let data = result.unwrap();

    // Verify the projects were loaded correctly
    let projects = data.projects();
    assert_eq!(projects.len(), 2);

    let rust_gui = projects.get("rust-gui").unwrap();
    assert_eq!(rust_gui.title, "Rust GUI フレームワーク開発");
    assert_eq!(
        rust_gui.notes,
        Some("RustでGUIフレームワークを作成するプロジェクト".to_string())
    );
    assert_eq!(rust_gui.context, Some("自宅".to_string()));

    let eci = projects.get("ECI").unwrap();
    assert_eq!(eci.title, "ECI");
    assert_eq!(
        eci.notes,
        Some("仕事のソフトウェア開発プロジェクト".to_string())
    );
    assert_eq!(eci.context, Some("仕事".to_string()));

    // Verify the task was loaded
    assert_eq!(data.next_action().len(), 1);
    let task = data.next_action()[0];
    assert_eq!(task.id, "#13");
    assert_eq!(task.title, "4章を攻略する");
}
