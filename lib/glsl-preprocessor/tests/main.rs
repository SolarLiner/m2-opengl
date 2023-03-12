use std::path::PathBuf;

use glsl_preprocessor::load_and_parse;

#[test]
fn test_process_file() {
    let tests_files = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    eprintln!("test files: {}", tests_files.display());
    let unwrapped = load_and_parse(tests_files.join("shader.glsl")).unwrap();
    insta::assert_debug_snapshot!(unwrapped);
}