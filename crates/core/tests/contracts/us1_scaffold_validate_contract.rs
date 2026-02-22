use std::fs;
use std::path::PathBuf;

#[test]
fn scaffold_validation_contract_has_expected_path() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let contract_path = repo_root
        .join("specs/001-workspace-scaffold/contracts/workspace-scaffold.openapi.yaml");
    let content = fs::read_to_string(contract_path).expect("contract file should exist");

    assert!(content.contains("/v1/scaffold/validate"));
    assert!(content.contains("operationId: validateScaffold"));
}
