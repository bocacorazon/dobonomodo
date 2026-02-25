// Operation implementations for the engine

pub mod aggregate;
pub mod output;
pub mod update;

// Re-export main types for convenience
pub use output::{
    execute_output, extract_schema, ColumnDef, ColumnType, OutputError, OutputOperation,
    OutputResult, OutputSchema, TemporalMode,
};

pub use update::{
    execute_update, Assignment, UpdateError, UpdateExecutionContext, UpdateOperation,
};
