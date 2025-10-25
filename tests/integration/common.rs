use gtd_mcp::*;
use std::env;
use tempfile::NamedTempFile;

pub fn get_test_path(name: &str) -> String {
    format!("{}/gtd-test-{}.toml", env::temp_dir().display(), name)
}

pub fn get_test_handler() -> (GtdServerHandler, NamedTempFile) {
    let temp_file = NamedTempFile::new().unwrap();
    let handler = GtdServerHandler::new(temp_file.path().to_str().unwrap(), false).unwrap();
    (handler, temp_file)
}
