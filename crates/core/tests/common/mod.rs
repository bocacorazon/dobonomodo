use std::fs;
use std::path::PathBuf;

pub fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(file_name)
}

pub fn read_fixture(file_name: &str) -> String {
    let path = fixture_path(file_name);
    fs::read_to_string(path).expect("fixture should be readable")
}
