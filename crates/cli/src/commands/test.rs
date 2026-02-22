use anyhow::{bail, Result};
use clap::Parser;
use dobo_core::model::{SuiteResult, TestResult, TestStatus};
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
        let scenario = parse_scenario(scenario_path)?;

        // Execute scenario
        let result = execute_scenario_with_base_dir(&scenario, scenario_path.parent())?;

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

        println!(
            "Discovered {} scenarios in: {}",
            scenarios.len(),
            suite_path.display()
        );
        println!();

        // Execute suite
        let suite_result = run_suite(&scenarios)?;

        // Report results
        self.report_suite(&suite_result, output_format)?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
