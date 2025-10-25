// Common test utilities for integration tests
use gtd_mcp::*;
use std::env;
use std::fs;
use std::sync::Mutex;

/// Counter for generating unique test file paths
static TEST_COUNTER: Mutex<u32> = Mutex::new(0);

/// Get a unique test file path for isolation
pub fn get_test_path() -> String {
    let mut counter = TEST_COUNTER.lock().unwrap();
    *counter += 1;
    let temp_dir = env::temp_dir();
    temp_dir
        .join(format!("gtd_test_{}.toml", *counter))
        .to_str()
        .unwrap()
        .to_string()
}

/// Cleanup test file if it exists
pub fn cleanup_test_file(path: &str) {
    let _ = fs::remove_file(path);
}
