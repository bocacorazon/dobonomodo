use anyhow::Result;
use dobo_core::model::{SuiteResult, TestResult, TestStatus};
use std::io::Write;
use std::path::Path;

/// Report test result in human-readable format
pub fn report_result(result: &TestResult, verbose: bool) {
    println!("Test: {}", result.scenario_name);

    match result.status {
        TestStatus::Pass => {
            println!("Status: PASS");
            println!();
            println!("✓ All expected rows found");
            println!("✓ No extra rows");
            println!("✓ No value mismatches");
        }
        TestStatus::Fail => {
            println!("Status: FAIL");
            println!();

            if !result.data_mismatches.is_empty() {
                println!("Data Mismatches ({}):", result.data_mismatches.len());
                for mismatch in &result.data_mismatches {
                    match mismatch.mismatch_type {
                        dobo_core::model::MismatchType::MissingRow => {
                            if let Some(expected) = &mismatch.expected {
                                println!("  ✗ Missing row: {:?}", expected);
                            }
                        }
                        dobo_core::model::MismatchType::ExtraRow => {
                            if let Some(actual) = &mismatch.actual {
                                println!("  ✗ Extra row: {:?}", actual);
                            }
                        }
                        dobo_core::model::MismatchType::ValueMismatch => {
                            println!("  ✗ Value mismatch");
                            if let Some(expected) = &mismatch.expected {
                                println!("      Expected: {:?}", expected);
                            }
                            if let Some(actual) = &mismatch.actual {
                                println!("      Actual:   {:?}", actual);
                            }
                            if !mismatch.differing_columns.is_empty() {
                                println!(
                                    "      Differing columns: {:?}",
                                    mismatch.differing_columns
                                );
                            }
                        }
                    }

                    if !verbose {
                        // Only show first few mismatches in non-verbose mode
                        if result.data_mismatches.len() > 5 {
                            println!(
                                "  ... and {} more mismatches (use --verbose to see all)",
                                result.data_mismatches.len() - 5
                            );
                            break;
                        }
                    }
                }
            }

            if !result.trace_mismatches.is_empty() {
                println!();
                println!("Trace Mismatches ({}):", result.trace_mismatches.len());
                for mismatch in &result.trace_mismatches {
                    println!(
                        "  ✗ Operation {}: {:?}",
                        mismatch.operation_order, mismatch.mismatch_type
                    );
                }
            }
        }
        TestStatus::Error => {
            println!("Status: ERROR");
            println!();

            if let Some(error) = &result.error {
                println!("Error: {}", error.message);
                if verbose {
                    if let Some(details) = &error.details {
                        println!();
                        println!("Details:");
                        println!("{}", details);
                    }
                }
            }
        }
    }

    // Show warnings if any
    if !result.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &result.warnings {
            println!("  ⚠ {}", warning);
        }
    }
}

/// Report suite results in human-readable format
pub fn report_suite_result(suite_result: &SuiteResult) {
    println!("Test Suite Results");
    println!("==================");
    println!();
    println!("Total:  {}", suite_result.total);
    println!(
        "Passed: {} ({:.1}%)",
        suite_result.passed,
        if suite_result.total > 0 {
            (suite_result.passed as f64 / suite_result.total as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Failed: {} ({:.1}%)",
        suite_result.failed,
        if suite_result.total > 0 {
            (suite_result.failed as f64 / suite_result.total as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Errors: {} ({:.1}%)",
        suite_result.errors,
        if suite_result.total > 0 {
            (suite_result.errors as f64 / suite_result.total as f64) * 100.0
        } else {
            0.0
        }
    );
    println!();

    // List individual results
    for result in &suite_result.results {
        let status_symbol = match result.status {
            TestStatus::Pass => "✓",
            TestStatus::Fail => "✗",
            TestStatus::Error => "⚠",
        };
        println!("{} {}", status_symbol, result.scenario_name);
    }
}

/// Save snapshot to file
pub fn save_snapshot(result: &TestResult, scenario_path: &Path) -> Result<()> {
    if let Some(snapshot) = &result.actual_snapshot {
        let snapshots_dir = scenario_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".snapshots");

        std::fs::create_dir_all(&snapshots_dir)?;

        // Create snapshot filename from scenario name
        let snapshot_name = sanitize_snapshot_name(&result.scenario_name);
        let snapshot_file = snapshots_dir.join(format!("{}-actual.yaml", snapshot_name));

        let yaml = serde_yaml::to_string(snapshot)?;
        std::fs::write(&snapshot_file, yaml)?;

        println!();
        println!("Snapshot saved to: {}", snapshot_file.display());
    }

    Ok(())
}

/// Output format for test results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
    Junit,
}

/// T097: Add JSON output format
pub fn report_result_json(result: &TestResult) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    println!("{}", json);
    Ok(())
}

/// T097: Add JSON output for suite results
pub fn report_suite_result_json(suite_result: &SuiteResult) -> Result<()> {
    let json = serde_json::to_string_pretty(suite_result)?;
    println!("{}", json);
    Ok(())
}

/// T098: Add JUnit XML output format
pub fn report_suite_result_junit<W: Write>(
    suite_result: &SuiteResult,
    writer: &mut W,
) -> Result<()> {
    // Calculate total time (simplified - would need actual durations)
    let total_time = suite_result.results.len() as f64 * 0.05; // Mock: 50ms per test

    writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(
        writer,
        "<testsuites tests=\"{}\" failures=\"{}\" errors=\"{}\" time=\"{:.3}\">",
        suite_result.total, suite_result.failed, suite_result.errors, total_time
    )?;

    writeln!(
        writer,
        "  <testsuite name=\"test-harness\" tests=\"{}\" failures=\"{}\" errors=\"{}\" time=\"{:.3}\">",
        suite_result.total, suite_result.failed, suite_result.errors, total_time
    )?;

    for result in &suite_result.results {
        let test_time = 0.05; // Mock: 50ms per test
        match result.status {
            TestStatus::Pass => {
                writeln!(
                    writer,
                    "    <testcase name=\"{}\" time=\"{:.3}\"/>",
                    xml_escape(&result.scenario_name),
                    test_time
                )?;
            }
            TestStatus::Fail => {
                writeln!(
                    writer,
                    "    <testcase name=\"{}\" time=\"{:.3}\">",
                    xml_escape(&result.scenario_name),
                    test_time
                )?;

                let mut failure_message = String::new();
                if !result.data_mismatches.is_empty() {
                    failure_message.push_str(&format!(
                        "{} data mismatches\n",
                        result.data_mismatches.len()
                    ));
                }
                if !result.trace_mismatches.is_empty() {
                    failure_message.push_str(&format!(
                        "{} trace mismatches\n",
                        result.trace_mismatches.len()
                    ));
                }

                writeln!(
                    writer,
                    "      <failure message=\"{}\" type=\"TestFailure\">",
                    xml_escape(&failure_message)
                )?;
                writeln!(writer, "{}", xml_escape(&failure_message))?;
                writeln!(writer, "      </failure>")?;
                writeln!(writer, "    </testcase>")?;
            }
            TestStatus::Error => {
                writeln!(
                    writer,
                    "    <testcase name=\"{}\" time=\"{:.3}\">",
                    xml_escape(&result.scenario_name),
                    test_time
                )?;

                let error_message = result
                    .error
                    .as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "Unknown error".to_string());

                writeln!(
                    writer,
                    "      <error message=\"{}\" type=\"{:?}\">",
                    xml_escape(&error_message),
                    result
                        .error
                        .as_ref()
                        .map(|e| format!("{:?}", e.error_type))
                        .unwrap_or_else(|| "UnknownError".to_string())
                )?;
                writeln!(writer, "{}", xml_escape(&error_message))?;
                writeln!(writer, "      </error>")?;
                writeln!(writer, "    </testcase>")?;
            }
        }
    }

    writeln!(writer, "  </testsuite>")?;
    writeln!(writer, "</testsuites>")?;

    Ok(())
}

/// Escape XML special characters
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sanitize_snapshot_name(name: &str) -> String {
    let mut output = String::new();
    let mut previous_was_dash = false;

    for ch in name.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            previous_was_dash = false;
            ch.to_ascii_lowercase()
        } else {
            if !previous_was_dash {
                output.push('-');
                previous_was_dash = true;
            }
            continue;
        };
        output.push(mapped);
    }

    let trimmed = output.trim_matches('-');
    if trimmed.is_empty() {
        "snapshot".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dobo_core::model::{DataBlock, TestResult};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn save_snapshot_sanitizes_unsafe_scenario_name() {
        let temp = TempDir::new().unwrap();
        let scenario_path = temp.path().join("scenario.yaml");
        std::fs::write(&scenario_path, "name: test").unwrap();

        let result = TestResult {
            scenario_name: "../escape".to_string(),
            status: TestStatus::Fail,
            warnings: vec![],
            data_mismatches: vec![],
            trace_mismatches: vec![],
            error: None,
            actual_snapshot: Some(DataBlock {
                rows: Some(vec![HashMap::from([(
                    "id".to_string(),
                    serde_json::Value::from(1),
                )])]),
                file: None,
            }),
        };

        save_snapshot(&result, &scenario_path).unwrap();

        let expected_path = temp.path().join(".snapshots").join("escape-actual.yaml");
        assert!(expected_path.exists());
        assert!(!temp.path().join("..").join("escape-actual.yaml").exists());
    }
}
