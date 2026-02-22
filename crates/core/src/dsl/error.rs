//! Error types for DSL parsing, validation, and compilation

use thiserror::Error;

/// Errors that can occur during expression parsing
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Syntax error at line {line}, column {column}: {message}")]
    SyntaxError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Unexpected token '{token}' at line {line}, column {column}")]
    UnexpectedToken {
        token: String,
        line: usize,
        column: usize,
    },

    #[error("Unclosed string literal at line {line}, column {column}")]
    UnclosedString { line: usize, column: usize },

    #[error("Unclosed parenthesis at line {line}, column {column}")]
    UnclosedParenthesis { line: usize, column: usize },

    #[error("Invalid number format '{value}' at line {line}, column {column}")]
    InvalidNumber {
        value: String,
        line: usize,
        column: usize,
    },

    #[error(
        "Invalid date format '{value}' at line {line}, column {column}. Expected ISO 8601 format"
    )]
    InvalidDate {
        value: String,
        line: usize,
        column: usize,
    },

    #[error("Parser internal error: {message}")]
    InternalError { message: String },
}

/// Errors that can occur during AST validation
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ValidationError {
    #[error("Unresolved column reference: {table}.{column}")]
    UnresolvedColumnRef { table: String, column: String },

    #[error("Type mismatch: expected {expected}, got {actual} in expression '{context}'")]
    TypeMismatch {
        expected: String,
        actual: String,
        context: String,
    },

    #[error("Aggregate function '{function}' used outside aggregate context")]
    InvalidAggregateContext { function: String },

    #[error("Unresolved selector reference: {{{selector}}}")]
    UnresolvedSelectorRef { selector: String },

    #[error("Circular selector reference detected: {cycle}")]
    CircularSelectorRef { cycle: String },

    #[error("Maximum selector interpolation depth ({max_depth}) exceeded")]
    MaxInterpolationDepth { max_depth: usize },

    #[error("Invalid function '{function}': {reason}")]
    InvalidFunction { function: String, reason: String },

    #[error(
        "Wrong number of arguments for function '{function}': expected {expected}, got {actual}"
    )]
    WrongArgumentCount {
        function: String,
        expected: String,
        actual: usize,
    },

    #[error("Division by zero in expression '{context}'")]
    DivisionByZero { context: String },

    #[error("Validation internal error: {message}")]
    InternalError { message: String },
}

/// Errors that can occur during compilation to Polars expressions
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CompilationError {
    #[error("Parse failure: {0}")]
    ParseFailure(#[from] ParseError),

    #[error("Validation failure: {0}")]
    ValidationFailure(#[from] ValidationError),

    #[error("Unsupported function '{function}': {reason}")]
    UnsupportedFunction { function: String, reason: String },

    #[error("Polars compatibility error: {message}")]
    PolarsCompatibility { message: String },

    #[error("Compilation internal error: {message}")]
    InternalError { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::SyntaxError {
            line: 1,
            column: 5,
            message: "unexpected token".to_string(),
        };
        assert!(err.to_string().contains("line 1"));
        assert!(err.to_string().contains("column 5"));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::UnresolvedColumnRef {
            table: "transactions".to_string(),
            column: "amount".to_string(),
        };
        assert!(err.to_string().contains("transactions.amount"));
    }

    #[test]
    fn test_compilation_error_display() {
        let err = CompilationError::UnsupportedFunction {
            function: "CUSTOM_FN".to_string(),
            reason: "not implemented".to_string(),
        };
        assert!(err.to_string().contains("CUSTOM_FN"));
    }
}
