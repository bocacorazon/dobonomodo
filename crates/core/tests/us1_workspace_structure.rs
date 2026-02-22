use std::path::PathBuf;

#[test]
fn workspace_contains_required_crates_and_modules() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    let required_paths = [
        "Cargo.toml",
        "crates/core/Cargo.toml",
        "crates/api-server/Cargo.toml",
        "crates/engine-worker/Cargo.toml",
        "crates/cli/Cargo.toml",
        "crates/test-resolver/Cargo.toml",
        "crates/core/src/model/mod.rs",
        "crates/core/src/dsl/mod.rs",
        "crates/core/src/engine/mod.rs",
        "crates/core/src/resolver/mod.rs",
        "crates/core/src/trace/mod.rs",
        "crates/core/src/validation/mod.rs",
    ];

    for path in required_paths {
        assert!(repo_root.join(path).exists(), "missing required path: {path}");
    }
}
