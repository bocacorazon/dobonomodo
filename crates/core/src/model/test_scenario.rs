use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{Dataset, LookupTarget, Project};

// ============================================================================
// Core Test Scenario Definition
// ============================================================================

/// Complete test scenario definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    /// Human-readable scenario name
    pub name: String,

    /// Narrative description of what is being tested
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The Period(s) the Run will execute against
    pub periods: Vec<PeriodDef>,

    /// Input Dataset definition with sample data
    pub input: TestInput,

    /// The Project to execute (inline or reference)
    pub project: ProjectDef,

    /// Expected result data for comparison
    pub expected_output: TestOutput,

    /// Expected trace events (optional)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_trace: Vec<TraceAssertion>,

    /// Test behavior configuration (has defaults)
    #[serde(default)]
    pub config: TestConfig,
}

impl TestScenario {
    /// Validate the scenario structure
    pub fn validate(&self) -> Result<()> {
        // Periods must have at least one entry
        if self.periods.is_empty() {
            bail!("TestScenario must have at least one period");
        }

        // Validate each period
        for period in &self.periods {
            period.validate()?;
        }

        // Validate DataBlocks have exactly one of rows or file
        for (name, block) in &self.input.data {
            block
                .validate()
                .with_context(|| format!("DataBlock '{}'", name))?;
        }

        let mut required_table_keys: HashSet<String> = HashSet::new();
        required_table_keys.insert(self.input.dataset.main_table.name.clone());

        for lookup in &self.input.dataset.lookups {
            if let LookupTarget::Table { name, .. } = &lookup.target {
                if let Some(alias) = &lookup.alias {
                    required_table_keys.insert(alias.clone());
                } else {
                    required_table_keys.insert(name.clone());
                }
            }
        }

        let provided_table_keys: HashSet<String> = self.input.data.keys().cloned().collect();
        let missing_tables: Vec<String> = required_table_keys
            .difference(&provided_table_keys)
            .cloned()
            .collect();
        if !missing_tables.is_empty() {
            bail!(
                "input.data is missing required table data for: {:?}",
                missing_tables
            );
        }

        let unknown_tables: Vec<String> = provided_table_keys
            .difference(&required_table_keys)
            .cloned()
            .collect();
        if !unknown_tables.is_empty() {
            bail!(
                "input.data contains unknown table keys not present in dataset definition: {:?}",
                unknown_tables
            );
        }

        // Validate expected output DataBlock
        self.expected_output
            .data
            .validate()
            .context("expected_output.data")?;

        Ok(())
    }
}

// ============================================================================
// Period Definition
// ============================================================================

/// Defines a Period for test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodDef {
    /// Period identifier (e.g., "2026-01")
    pub identifier: String,

    /// Calendar level name (e.g., "month")
    pub level: String,

    /// Period start date
    pub start_date: NaiveDate,

    /// Period end date
    pub end_date: NaiveDate,
}

impl PeriodDef {
    /// Validate period constraints
    pub fn validate(&self) -> Result<()> {
        if self.start_date > self.end_date {
            bail!(
                "Period '{}': start_date ({}) must be before or equal to end_date ({})",
                self.identifier,
                self.start_date,
                self.end_date
            );
        }
        Ok(())
    }
}

// ============================================================================
// Test Input
// ============================================================================

/// Defines the input dataset schema and data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInput {
    /// Dataset schema definition (reuses core entity)
    pub dataset: Dataset,

    /// Data for each table (keyed by table logical name)
    pub data: HashMap<String, DataBlock>,
}

// ============================================================================
// Data Block
// ============================================================================

/// Defines test data for a single table (inline or file reference)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBlock {
    /// Inline data rows (each map is column→value)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<HashMap<String, serde_json::Value>>>,

    /// Path to external data file (CSV, Parquet)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

impl DataBlock {
    /// Validate that exactly one of rows or file is present
    pub fn validate(&self) -> Result<()> {
        match (&self.rows, &self.file) {
            (Some(_), Some(_)) => bail!("DataBlock cannot have both rows and file"),
            (None, None) => bail!("DataBlock must have either rows or file"),
            _ => Ok(()),
        }
    }
}

// ============================================================================
// Project Definition
// ============================================================================

/// Polymorphic type for inline project or reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProjectDef {
    /// Full inline project definition
    Inline(Box<Project>),

    /// Reference to existing project
    Ref {
        /// Project ID
        id: Uuid,
        /// Project version number
        version: i32,
    },
}

// ============================================================================
// Test Output
// ============================================================================

/// Defines expected output data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutput {
    /// Expected output rows (inline or file reference)
    pub data: DataBlock,
}

// ============================================================================
// Test Configuration
// ============================================================================

/// Controls test execution and comparison behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Row matching strategy
    #[serde(default)]
    pub match_mode: MatchMode,

    /// Include system columns in comparison
    #[serde(default)]
    pub validate_metadata: bool,

    /// Validate trace events
    #[serde(default)]
    pub validate_traceability: bool,

    /// Save actual output on failure
    #[serde(default = "default_snapshot_on_failure")]
    pub snapshot_on_failure: bool,

    /// Require row order match
    #[serde(default)]
    pub order_sensitive: bool,
}

fn default_snapshot_on_failure() -> bool {
    true
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            match_mode: MatchMode::Exact,
            validate_metadata: false,
            validate_traceability: false,
            snapshot_on_failure: true,
            order_sensitive: false,
        }
    }
}

/// Row matching strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// All rows must match exactly; no extra rows allowed
    #[default]
    Exact,
    /// Expected rows must exist in actual; extra actual rows tolerated
    Subset,
}

// ============================================================================
// Trace Assertion
// ============================================================================

/// Defines expected trace event for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceAssertion {
    /// The operation that should produce this trace event
    pub operation_order: i32,

    /// Type of change (created/updated/deleted)
    pub change_type: TraceChangeType,

    /// Column values identifying the row
    pub row_match: HashMap<String, serde_json::Value>,

    /// Expected column changes (for `updated` only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_diff: Option<HashMap<String, serde_json::Value>>,
}

/// Type of change in trace event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceChangeType {
    Created,
    Updated,
    Deleted,
}

// ============================================================================
// Test Result
// ============================================================================

/// Output of test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Name from TestScenario
    pub scenario_name: String,

    /// Pass/Fail/Error
    pub status: TestStatus,

    /// Non-fatal warnings (e.g., version drift)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Row-level mismatches (empty on pass)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_mismatches: Vec<DataMismatch>,

    /// Trace assertion mismatches
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace_mismatches: Vec<TraceMismatch>,

    /// Present only when status is Error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TestErrorDetail>,

    /// Actual output (when snapshot_on_failure=true and test fails)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_snapshot: Option<DataBlock>,
}

/// Test status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    /// All assertions passed
    Pass,
    /// Data or trace mismatches found
    Fail,
    /// Execution failed (parse error, execution exception, etc.)
    Error,
}

// ============================================================================
// Data Mismatch
// ============================================================================

/// Represents a single data validation failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMismatch {
    /// Type of mismatch
    pub mismatch_type: MismatchType,

    /// Expected row values (for missing_row, value_mismatch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<HashMap<String, serde_json::Value>>,

    /// Actual row values (for extra_row, value_mismatch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<HashMap<String, serde_json::Value>>,

    /// Columns that differ (for value_mismatch only)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub differing_columns: Vec<String>,
}

/// Type of data mismatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MismatchType {
    /// Expected row not found in actual output
    MissingRow,
    /// Actual row not in expected output (only reported in Exact mode)
    ExtraRow,
    /// Row found but column values differ
    ValueMismatch,
}

// ============================================================================
// Trace Mismatch
// ============================================================================

/// Represents a trace validation failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMismatch {
    /// Operation where assertion failed
    pub operation_order: i32,

    /// Type of trace mismatch
    pub mismatch_type: TraceMismatchType,

    /// The expected trace assertion
    pub expected: TraceAssertion,

    /// The actual trace event (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<serde_json::Value>, // Simplified for now
}

/// Type of trace mismatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMismatchType {
    /// Expected trace event not found
    MissingEvent,
    /// Unexpected trace event found
    ExtraEvent,
    /// Trace event found but diff values incorrect
    DiffMismatch,
}

// ============================================================================
// Error Detail
// ============================================================================

/// Execution error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestErrorDetail {
    /// Category of error
    pub error_type: ErrorType,

    /// Human-readable error message
    pub message: String,

    /// Additional technical details (stack trace, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Type of error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// YAML parsing failure
    ParseError,
    /// Dataset schema invalid
    SchemaValidationError,
    /// Pipeline execution failure
    ExecutionError,
    /// Data file reference not found
    FileNotFound,
    /// ProjectRef resolution failure
    ProjectNotFound,
}

// ============================================================================
// Suite Result
// ============================================================================

/// Aggregated results from test suite execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResult {
    /// Total number of scenarios executed
    pub total: usize,

    /// Number of passed scenarios
    pub passed: usize,

    /// Number of failed scenarios
    pub failed: usize,

    /// Number of errored scenarios
    pub errors: usize,

    /// Individual test results
    pub results: Vec<TestResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// T103: Comprehensive unit tests for TestScenario::validate()
    #[test]
    fn test_validate_empty_periods_fails() {
        let scenario = TestScenario {
            name: "test".to_string(),
            description: None,
            periods: vec![],
            input: create_test_input(),
            project: create_test_project(),
            expected_output: create_test_output(),
            expected_trace: vec![],
            config: TestConfig::default(),
        };

        let result = scenario.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one period"));
    }

    #[test]
    fn test_validate_period_start_after_end_fails() {
        let scenario = TestScenario {
            name: "test".to_string(),
            description: None,
            periods: vec![PeriodDef {
                identifier: "2026-01".to_string(),
                level: "month".to_string(),
                start_date: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            }],
            input: create_test_input(),
            project: create_test_project(),
            expected_output: create_test_output(),
            expected_trace: vec![],
            config: TestConfig::default(),
        };

        let result = scenario.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("start_date") && err_msg.contains("end_date"));
    }

    #[test]
    fn test_validate_datablock_both_rows_and_file_fails() {
        let mut scenario = create_valid_scenario();
        scenario.input.data.insert(
            "invalid_table".to_string(),
            DataBlock {
                rows: Some(vec![HashMap::new()]),
                file: Some("test.csv".to_string()),
            },
        );

        let result = scenario.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("both rows and file") || err_msg.contains("DataBlock"));
    }

    #[test]
    fn test_validate_datablock_neither_rows_nor_file_fails() {
        let mut scenario = create_valid_scenario();
        scenario.input.data.insert(
            "invalid_table2".to_string(),
            DataBlock {
                rows: None,
                file: None,
            },
        );

        let result = scenario.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("either rows or file") || err_msg.contains("DataBlock"));
    }

    #[test]
    fn test_validate_expected_output_invalid_datablock_fails() {
        let mut scenario = create_valid_scenario();
        scenario.expected_output = TestOutput {
            data: DataBlock {
                rows: None,
                file: None,
            },
        };

        let result = scenario.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_scenario_succeeds() {
        let scenario = create_valid_scenario();
        assert!(scenario.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_required_table_data_fails() {
        let mut scenario = create_valid_scenario();
        scenario.input.data.clear();

        let result = scenario.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing required table data"));
    }

    #[test]
    fn test_validate_unknown_input_table_key_fails() {
        let mut scenario = create_valid_scenario();
        scenario.input.data.insert(
            "unknown_table".to_string(),
            DataBlock {
                rows: Some(vec![HashMap::new()]),
                file: None,
            },
        );

        let result = scenario.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unknown table keys"));
    }

    #[test]
    fn test_validate_lookup_alias_requires_matching_input_data_key() {
        let mut scenario = create_valid_scenario();
        scenario.input.dataset.lookups = vec![crate::model::LookupDef {
            alias: Some("products_alias".to_string()),
            target: crate::model::LookupTarget::Table {
                name: "products".to_string(),
                temporal_mode: None,
                columns: vec![],
            },
            join_conditions: vec![],
        }];

        let result = scenario.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("products_alias"));
    }

    #[test]
    fn test_validate_lookup_alias_data_key_succeeds_when_provided() {
        let mut scenario = create_valid_scenario();
        scenario.input.dataset.lookups = vec![crate::model::LookupDef {
            alias: Some("products_alias".to_string()),
            target: crate::model::LookupTarget::Table {
                name: "products".to_string(),
                temporal_mode: None,
                columns: vec![],
            },
            join_conditions: vec![],
        }];
        scenario.input.data.insert(
            "products_alias".to_string(),
            DataBlock {
                rows: Some(vec![HashMap::new()]),
                file: None,
            },
        );

        assert!(scenario.validate().is_ok());
    }

    #[test]
    fn test_period_def_validate_start_equals_end_succeeds() {
        let period = PeriodDef {
            identifier: "2026-01-01".to_string(),
            level: "day".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        };

        assert!(period.validate().is_ok());
    }

    #[test]
    fn test_datablock_validate_rows_only_succeeds() {
        let block = DataBlock {
            rows: Some(vec![HashMap::new()]),
            file: None,
        };

        assert!(block.validate().is_ok());
    }

    #[test]
    fn test_datablock_validate_file_only_succeeds() {
        let block = DataBlock {
            rows: None,
            file: Some("test.csv".to_string()),
        };

        assert!(block.validate().is_ok());
    }

    // Helper functions for creating test data
    fn create_test_input() -> TestInput {
        TestInput {
            dataset: create_test_dataset(),
            data: HashMap::from([(
                "test_table".to_string(),
                DataBlock {
                    rows: Some(vec![HashMap::new()]),
                    file: None,
                },
            )]),
        }
    }

    fn create_test_dataset() -> Dataset {
        Dataset {
            id: Uuid::now_v7(),
            name: "test_dataset".to_string(),
            description: Some("Test".to_string()),
            owner: "test".to_string(),
            version: 1,
            status: crate::model::DatasetStatus::Active,
            resolver_id: None,
            main_table: crate::model::TableRef {
                name: "test_table".to_string(),
                temporal_mode: Some(crate::model::TemporalMode::Period),
                columns: vec![],
            },
            lookups: vec![],
            natural_key_columns: vec![],
            created_at: None,
            updated_at: None,
        }
    }

    fn create_test_project() -> ProjectDef {
        use std::collections::BTreeMap;
        ProjectDef::Inline(Box::new(Project {
            id: Uuid::now_v7(),
            name: "test_project".to_string(),
            description: None,
            owner: "test".to_string(),
            version: 1,
            status: crate::model::ProjectStatus::Active,
            visibility: crate::model::Visibility::Private,
            input_dataset_id: Uuid::now_v7(),
            input_dataset_version: 1,
            materialization: crate::model::Materialization::Eager,
            operations: vec![],
            selectors: BTreeMap::new(),
            resolver_overrides: BTreeMap::new(),
            conflict_report: None,
            created_at: None,
            updated_at: None,
        }))
    }

    fn create_test_output() -> TestOutput {
        TestOutput {
            data: DataBlock {
                rows: Some(vec![HashMap::new()]),
                file: None,
            },
        }
    }

    fn create_valid_scenario() -> TestScenario {
        TestScenario {
            name: "valid_test".to_string(),
            description: Some("A valid test scenario".to_string()),
            periods: vec![PeriodDef {
                identifier: "2026-01".to_string(),
                level: "month".to_string(),
                start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            }],
            input: create_test_input(),
            project: create_test_project(),
            expected_output: create_test_output(),
            expected_trace: vec![],
            config: TestConfig::default(),
        }
    }
}
