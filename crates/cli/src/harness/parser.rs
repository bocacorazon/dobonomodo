use anyhow::{Context, Result};
use dobo_core::model::TestScenario;
use std::path::Path;

/// Parse a test scenario from YAML file
/// T100: Improved error messages with file location and field paths
pub fn parse_scenario(path: &Path) -> Result<TestScenario> {
    // Check if file exists first for better error message
    if !path.exists() {
        anyhow::bail!(
            "Scenario file not found: {}\nPlease check the file path and try again.",
            path.display()
        );
    }

    let content = std::fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read scenario file: {}\nPlease check file permissions.",
            path.display()
        )
    })?;

    // Use serde_path_to_error for better field-level error reporting
    let deserializer = serde_yaml::Deserializer::from_str(&content);
    let scenario: TestScenario =
        serde_path_to_error::deserialize(deserializer).with_context(|| {
            format!(
                "Failed to parse YAML from: {}\n\
             This usually means there's a syntax error or missing required field.\n\
             Check the YAML structure against the documentation.",
                path.display()
            )
        })?;

    // Validate the scenario
    scenario.validate().with_context(|| {
        format!(
            "Validation failed for scenario: {}\n\
             The YAML was parsed successfully but contains invalid data.\n\
             Check the error message above for specific validation issues.",
            path.display()
        )
    })?;

    Ok(scenario)
}

#[cfg(test)]
mod tests {
    use super::parse_scenario;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_scenario_reports_missing_file_with_context() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("missing.yaml");

        let error = parse_scenario(&missing).unwrap_err().to_string();
        assert!(error.contains("Scenario file not found"));
        assert!(error.contains(&missing.display().to_string()));
    }

    #[test]
    fn parse_scenario_reports_yaml_parse_errors_with_context() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("invalid.yaml");
        fs::write(&path, "name: [\n").unwrap();

        let error = parse_scenario(&path).unwrap_err().to_string();
        assert!(error.contains("Failed to parse YAML"));
        assert!(error.contains(&path.display().to_string()));
    }

    #[test]
    fn parse_scenario_reports_validation_errors_with_context() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("invalid-structure.yaml");
        fs::write(
            &path,
            r#"
name: invalid scenario
periods: []
input:
  dataset:
    id: "018f6f30-7a35-7000-8000-000000000001"
    name: dataset
    owner: owner
    version: 1
    status: active
    main_table:
      name: orders
      temporal_mode: period
      columns: []
  data: {}
project:
  id: "018f6f30-7a35-7000-8000-000000000002"
  version: 1
expected_output:
  data:
    rows: []
"#,
        )
        .unwrap();

        let error = parse_scenario(&path).unwrap_err().to_string();
        assert!(error.contains("Validation failed for scenario"));
        assert!(error.contains("contains invalid data"));
        assert!(error.contains(&path.display().to_string()));
    }
}
