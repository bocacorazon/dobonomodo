use anyhow::{bail, Result};
use clap::Parser;
use dobo_core::model::{ErrorType, SuiteResult, TestErrorDetail, TestResult, TestStatus};
use std::path::{Path, PathBuf};

use crate::harness::{
    discover_scenarios, execute_scenario_with_base_dir, execute_suite as run_suite, parse_scenario,
    report_result, report_result_json, report_suite_result, report_suite_result_json,
    report_suite_result_junit, save_snapshot, OutputFormat,
};

const DEFAULT_SUITE_DIR: &str = "tests/scenarios";

enum ExecutionTarget<'a> {
    Suite(&'a Path),
    Single(&'a Path),
}

/// Execute test scenarios
#[derive(Debug, Parser)]
pub struct TestCommand {
    /// Path to the test scenario YAML file (for single scenario mode)
    #[arg(value_name = "SCENARIO")]
    pub scenario_path: Option<PathBuf>,

    /// Execute all scenarios in directory (suite mode)
    #[arg(long, value_name = "DIR")]
    pub suite: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Disable snapshot on failure
    #[arg(long)]
    pub no_snapshot: bool,

    /// Output format (human, json, junit)
    #[arg(long, value_name = "FORMAT", default_value = "human")]
    pub output: String,
}

impl TestCommand {
    pub fn execute(&self) -> Result<i32> {
        match self.execution_target() {
            ExecutionTarget::Suite(suite_path) => self.execute_suite(suite_path),
            ExecutionTarget::Single(scenario_path) => self.execute_single(scenario_path),
        }
    }

    fn execution_target(&self) -> ExecutionTarget<'_> {
        if let Some(suite_path) = &self.suite {
            ExecutionTarget::Suite(suite_path)
        } else if let Some(scenario_path) = &self.scenario_path {
            ExecutionTarget::Single(scenario_path)
        } else {
            ExecutionTarget::Suite(Path::new(DEFAULT_SUITE_DIR))
        }
    }

    fn execute_single(&self, scenario_path: &Path) -> Result<i32> {
        let output_format = self.output_format()?;

        // Parse scenario
        let scenario = match parse_scenario(scenario_path) {
            Ok(scenario) => scenario,
            Err(error) => {
                let result = build_error_result(
                    scenario_path.display().to_string(),
                    ErrorType::ParseError,
                    error,
                );
                self.report_single(&result, output_format)?;
                return Ok(2);
            }
        };

        // Execute scenario
        let result = match execute_scenario_with_base_dir(&scenario, scenario_path.parent()) {
            Ok(result) => result,
            Err(error) => {
                let result =
                    build_error_result(scenario.name.clone(), ErrorType::ExecutionError, error);
                self.report_single(&result, output_format)?;
                return Ok(2);
            }
        };

        // Report result
        self.report_single(&result, output_format)?;

        // Save snapshot if needed
        if !self.no_snapshot && result.status == TestStatus::Fail {
            save_snapshot(&result, scenario_path)?;
        }

        // Return exit code
        Ok(match result.status {
            TestStatus::Pass => 0,
            TestStatus::Fail => 1,
            TestStatus::Error => 2,
        })
    }

    fn execute_suite(&self, suite_path: &Path) -> Result<i32> {
        let output_format = self.output_format()?;

        // Discover scenarios
        let scenarios = discover_scenarios(suite_path)?;

        if scenarios.is_empty() {
            eprintln!("No test scenarios found in: {}", suite_path.display());
            return Ok(2);
        }

        if should_print_discovery_banner(output_format) {
            println!(
                "Discovered {} scenarios in: {}",
                scenarios.len(),
                suite_path.display()
            );
            println!();
        }

        // Execute suite
        let suite_result = run_suite(&scenarios)?;

        // Report results
        self.report_suite(&suite_result, output_format)?;

        if !self.no_snapshot {
            self.save_suite_snapshots(&suite_result, &scenarios)?;
        }

        // Return exit code based on results
        Ok(if suite_result.errors > 0 {
            2
        } else if suite_result.failed > 0 {
            1
        } else {
            0
        })
    }

    fn output_format(&self) -> Result<OutputFormat> {
        match self.output.to_ascii_lowercase().as_str() {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            "junit" => Ok(OutputFormat::Junit),
            other => bail!("Unsupported output format: {other}. Use human, json, or junit."),
        }
    }

    fn report_single(&self, result: &TestResult, output_format: OutputFormat) -> Result<()> {
        match output_format {
            OutputFormat::Human => report_result(result, self.verbose),
            OutputFormat::Json => report_result_json(result)?,
            OutputFormat::Junit => {
                let suite_result = SuiteResult {
                    total: 1,
                    passed: usize::from(result.status == TestStatus::Pass),
                    failed: usize::from(result.status == TestStatus::Fail),
                    errors: usize::from(result.status == TestStatus::Error),
                    results: vec![result.clone()],
                };
                let mut stdout = std::io::stdout();
                report_suite_result_junit(&suite_result, &mut stdout)?;
            }
        }
        Ok(())
    }

    fn report_suite(&self, suite_result: &SuiteResult, output_format: OutputFormat) -> Result<()> {
        match output_format {
            OutputFormat::Human => report_suite_result(suite_result),
            OutputFormat::Json => report_suite_result_json(suite_result)?,
            OutputFormat::Junit => {
                let mut stdout = std::io::stdout();
                report_suite_result_junit(suite_result, &mut stdout)?;
            }
        }
        Ok(())
    }

    fn save_suite_snapshots(
        &self,
        suite_result: &SuiteResult,
        scenarios: &[PathBuf],
    ) -> Result<()> {
        for (scenario_path, result) in scenarios.iter().zip(suite_result.results.iter()) {
            if result.status == TestStatus::Fail {
                save_snapshot(result, scenario_path)?;
            }
        }

        Ok(())
    }
}

fn should_print_discovery_banner(output_format: OutputFormat) -> bool {
    matches!(output_format, OutputFormat::Human)
}

fn build_error_result(
    scenario_name: String,
    error_type: ErrorType,
    error: anyhow::Error,
) -> TestResult {
    TestResult {
        scenario_name,
        status: TestStatus::Error,
        warnings: vec![],
        data_mismatches: vec![],
        trace_mismatches: vec![],
        error: Some(TestErrorDetail {
            error_type,
            message: error.to_string(),
            details: Some(format!("{:?}", error)),
        }),
        actual_snapshot: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dobo_core::model::DataBlock;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn execution_target_defaults_to_suite_directory() {
        let command = TestCommand {
            scenario_path: None,
            suite: None,
            verbose: false,
            no_snapshot: false,
            output: "human".to_string(),
        };

        match command.execution_target() {
            ExecutionTarget::Suite(path) => assert_eq!(path, Path::new(DEFAULT_SUITE_DIR)),
            ExecutionTarget::Single(_) => panic!("expected suite target"),
        }
    }

    #[test]
    fn execution_target_prefers_explicit_scenario() {
        let scenario = PathBuf::from("scenario.yaml");
        let command = TestCommand {
            scenario_path: Some(scenario.clone()),
            suite: None,
            verbose: false,
            no_snapshot: false,
            output: "human".to_string(),
        };

        match command.execution_target() {
            ExecutionTarget::Single(path) => assert_eq!(path, scenario.as_path()),
            ExecutionTarget::Suite(_) => panic!("expected single target"),
        }
    }

    #[test]
    fn discovery_banner_is_only_for_human_output() {
        assert!(should_print_discovery_banner(OutputFormat::Human));
        assert!(!should_print_discovery_banner(OutputFormat::Json));
        assert!(!should_print_discovery_banner(OutputFormat::Junit));
    }

    #[test]
    fn save_suite_snapshots_persists_failed_results() {
        let temp_dir = tempdir().unwrap();
        let scenario_path = temp_dir.path().join("scenario.yaml");
        std::fs::write(&scenario_path, "name: scenario\n").unwrap();

        let command = TestCommand {
            scenario_path: None,
            suite: None,
            verbose: false,
            no_snapshot: false,
            output: "human".to_string(),
        };

        let result = TestResult {
            scenario_name: "failing-scenario".to_string(),
            status: TestStatus::Fail,
            warnings: vec![],
            data_mismatches: vec![],
            trace_mismatches: vec![],
            error: None,
            actual_snapshot: Some(DataBlock {
                rows: Some(vec![HashMap::from([("id".to_string(), json!(1))])]),
                file: None,
            }),
        };
        let suite_result = SuiteResult {
            total: 1,
            passed: 0,
            failed: 1,
            errors: 0,
            results: vec![result],
        };

        command
            .save_suite_snapshots(&suite_result, &[scenario_path])
            .unwrap();

        let snapshot_dir = temp_dir.path().join(".snapshots");
        assert!(snapshot_dir.exists());
        assert_eq!(std::fs::read_dir(snapshot_dir).unwrap().count(), 1);
    }

    #[test]
    fn execute_single_supports_project_ref_with_fixture_metadata_store() {
        let temp_dir = tempdir().unwrap();
        let scenario_path = temp_dir.path().join("scenario.yaml");
        let project_fixture_path = temp_dir.path().join("projects.yaml");
        let dataset_id = Uuid::now_v7();
        let project_id = Uuid::now_v7();

        std::fs::write(
            &scenario_path,
            format!(
                r#"
name: "project-ref-cli"
periods:
  - identifier: "2026-01"
    level: "month"
    start_date: "2026-01-01"
    end_date: "2026-01-31"
input:
  dataset:
    id: "{dataset_id}"
    name: "orders_dataset"
    owner: "test"
    version: 2
    status: active
    main_table:
      name: "orders"
      temporal_mode: period
      columns:
        - name: "id"
          type: integer
          nullable: false
        - name: "value"
          type: integer
          nullable: false
    lookups: []
    natural_key_columns: ["id"]
  data:
    orders:
      rows:
        - id: 1
          value: 10
          _period: "2026-01"
project:
  id: "{project_id}"
  version: 1
expected_output:
  data:
    rows:
      - id: 1
        value: 10
config:
  match_mode: exact
  validate_metadata: false
  validate_traceability: false
  snapshot_on_failure: true
"#
            ),
        )
        .unwrap();

        std::fs::write(
            &project_fixture_path,
            format!(
                r#"
- id: "{project_id}"
  name: "fixture-project"
  owner: "test"
  version: 3
  status: active
  visibility: private
  input_dataset_id: "{dataset_id}"
  input_dataset_version: 2
  materialization: eager
  operations: []
  selectors: {{}}
  resolver_overrides: {{}}
"#
            ),
        )
        .unwrap();

        let command = TestCommand {
            scenario_path: Some(scenario_path),
            suite: None,
            verbose: false,
            no_snapshot: true,
            output: "human".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn execute_suite_includes_exact_mode_failure_fixture() {
        let temp_dir = tempdir().unwrap();
        let suite_dir = temp_dir.path().join("suite");
        fs::create_dir_all(&suite_dir).unwrap();

        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let source_fixture = workspace_root.join("tests/scenarios/exact-match-test.yaml");
        assert!(source_fixture.is_file());

        let copied_fixture = suite_dir.join("exact-match-test.yaml");
        fs::copy(source_fixture, &copied_fixture).unwrap();

        let command = TestCommand {
            scenario_path: None,
            suite: Some(suite_dir),
            verbose: false,
            no_snapshot: true,
            output: "human".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn execute_single_missing_scenario_file_returns_exit_code_2() {
        let temp_dir = tempdir().unwrap();
        let scenario_path = temp_dir.path().join("missing.yaml");

        let command = TestCommand {
            scenario_path: Some(scenario_path),
            suite: None,
            verbose: false,
            no_snapshot: true,
            output: "human".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 2);
    }

    #[test]
    fn execute_single_malformed_scenario_file_returns_exit_code_2() {
        let temp_dir = tempdir().unwrap();
        let scenario_path = temp_dir.path().join("invalid.yaml");
        std::fs::write(&scenario_path, "name: [\n").unwrap();

        let command = TestCommand {
            scenario_path: Some(scenario_path),
            suite: None,
            verbose: false,
            no_snapshot: true,
            output: "human".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 2);
    }

    #[test]
    fn execute_single_with_json_output_returns_exit_code_0() {
        let temp_dir = tempdir().unwrap();
        let scenario_path = temp_dir.path().join("harness-self-test.yaml");
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let source_fixture = workspace_root.join("tests/scenarios/harness-self-test.yaml");
        fs::copy(source_fixture, &scenario_path).unwrap();

        let command = TestCommand {
            scenario_path: Some(scenario_path),
            suite: None,
            verbose: false,
            no_snapshot: true,
            output: "json".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn execute_single_with_junit_output_returns_exit_code_0() {
        let temp_dir = tempdir().unwrap();
        let scenario_path = temp_dir.path().join("harness-self-test.yaml");
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let source_fixture = workspace_root.join("tests/scenarios/harness-self-test.yaml");
        fs::copy(source_fixture, &scenario_path).unwrap();

        let command = TestCommand {
            scenario_path: Some(scenario_path),
            suite: None,
            verbose: false,
            no_snapshot: true,
            output: "junit".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn execute_suite_with_json_output_returns_failure_exit_code() {
        let temp_dir = tempdir().unwrap();
        let suite_dir = temp_dir.path().join("suite");
        fs::create_dir_all(&suite_dir).unwrap();

        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let source_fixture = workspace_root.join("tests/scenarios/exact-match-test.yaml");
        let copied_fixture = suite_dir.join("exact-match-test.yaml");
        fs::copy(source_fixture, &copied_fixture).unwrap();

        let command = TestCommand {
            scenario_path: None,
            suite: Some(suite_dir),
            verbose: false,
            no_snapshot: true,
            output: "json".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn execute_suite_with_junit_output_returns_failure_exit_code() {
        let temp_dir = tempdir().unwrap();
        let suite_dir = temp_dir.path().join("suite");
        fs::create_dir_all(&suite_dir).unwrap();

        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let source_fixture = workspace_root.join("tests/scenarios/exact-match-test.yaml");
        let copied_fixture = suite_dir.join("exact-match-test.yaml");
        fs::copy(source_fixture, &copied_fixture).unwrap();

        let command = TestCommand {
            scenario_path: None,
            suite: Some(suite_dir),
            verbose: false,
            no_snapshot: true,
            output: "junit".to_string(),
        };

        let exit_code = command.execute().unwrap();
        assert_eq!(exit_code, 1);
    }
}
