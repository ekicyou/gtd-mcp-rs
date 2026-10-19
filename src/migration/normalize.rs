//! Line ending normalization functions
//!
//! This module handles normalization of line endings in string fields
//! to ensure consistent formatting across different platforms.

/// Normalize line endings in a string to LF (\n)
///
/// This handles cases where TOML files contain \r escape sequences that get
/// unescaped to CR bytes during deserialization. We normalize these to LF
/// so they can be properly converted to OS-native format on save.
///
/// # Arguments
///
/// * `s` - String to normalize
///
/// # Returns
///
/// String with normalized line endings
pub fn normalize_string_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}
