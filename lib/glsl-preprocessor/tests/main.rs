use std::path::PathBuf;

use glsl_preprocessor::process_file;

const TESTS_FILES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

#[test]
fn test_process_file() {
    let tests_files = PathBuf::from(TESTS_FILES);
    let unwrapped = process_file(tests_files.join("shader.glsl"));
    insta::assert_snapshot!(unwrapped);
}