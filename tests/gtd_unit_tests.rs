// Unit tests for gtd module - extracted from src/gtd.rs for better modularity
// This file contains tests for the core GtdData struct and its methods

use gtd_mcp::GtdData;
use chrono::{Datelike, NaiveDate};

// GtdDataの新規作成テスト
// 空のタスク、プロジェクト、コンテキストのHashMapが初期化されることを確認
#[test]
fn test_gtd_data_new() {
    let data = GtdData::new();
    assert!(data.inbox.is_empty());
    assert!(data.next_action.is_empty());
    assert!(data.waiting_for.is_empty());
    assert!(data.someday.is_empty());
    assert!(data.later.is_empty());
    assert!(data.done.is_empty());
    assert!(data.trash.is_empty());
    assert!(data.projects.is_empty());
    assert!(data.contexts.is_empty());
}
