//! Compilation context for expression validation and compilation

use std::collections::HashMap;

/// Compilation context providing schema and selector information
#[derive(Debug, Clone)]
pub struct CompilationContext {
    /// Schema information: map of table.column -> column type
    pub schema: HashMap<String, ColumnInfo>,

    /// Selector definitions: map of selector name -> expression string
    pub selectors: HashMap<String, String>,

    /// Whether aggregate functions are allowed in this context
    pub allow_aggregates: bool,
}

/// Column information from schema
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnInfo {
    /// Full column name (table.column)
    pub name: String,

    /// Column data type
    pub column_type: ColumnType,
}

/// Column type from schema
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    /// Integer type
    Integer,

    /// Float type
    Float,

    /// String type
    String,

    /// Boolean type
    Boolean,

    /// Date type
    Date,
}

impl CompilationContext {
    /// Create a new empty compilation context
    pub fn new() -> Self {
        Self {
            schema: HashMap::new(),
            selectors: HashMap::new(),
            allow_aggregates: false,
        }
    }

    /// Create a new compilation context with schema
    pub fn with_schema(schema: HashMap<String, ColumnInfo>) -> Self {
        Self {
            schema,
            selectors: HashMap::new(),
            allow_aggregates: false,
        }
    }

    /// Add a column to the schema
    pub fn add_column(&mut self, name: impl Into<String>, column_type: ColumnType) {
        let name = name.into();
        self.schema
            .insert(name.clone(), ColumnInfo { name, column_type });
    }

    /// Add a selector definition
    pub fn add_selector(&mut self, name: impl Into<String>, expression: impl Into<String>) {
        self.selectors.insert(name.into(), expression.into());
    }

    /// Set whether aggregates are allowed
    pub fn with_aggregates(mut self, allow: bool) -> Self {
        self.allow_aggregates = allow;
        self
    }

    /// Look up a column in the schema
    pub fn get_column(&self, name: &str) -> Option<&ColumnInfo> {
        self.schema.get(name)
    }

    /// Look up a selector definition
    pub fn get_selector(&self, name: &str) -> Option<&str> {
        self.selectors.get(name).map(|s| s.as_str())
    }
}

impl Default for CompilationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnType {
    /// Get a human-readable name for the column type
    pub fn name(&self) -> &'static str {
        match self {
            ColumnType::Integer => "Integer",
            ColumnType::Float => "Float",
            ColumnType::String => "String",
            ColumnType::Boolean => "Boolean",
            ColumnType::Date => "Date",
        }
    }

    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, ColumnType::Integer | ColumnType::Float)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_context_new() {
        let ctx = CompilationContext::new();
        assert!(ctx.schema.is_empty());
        assert!(ctx.selectors.is_empty());
        assert!(!ctx.allow_aggregates);
    }

    #[test]
    fn test_add_column() {
        let mut ctx = CompilationContext::new();
        ctx.add_column("transactions.amount", ColumnType::Float);

        let col = ctx.get_column("transactions.amount").unwrap();
        assert_eq!(col.column_type, ColumnType::Float);
    }

    #[test]
    fn test_add_selector() {
        let mut ctx = CompilationContext::new();
        ctx.add_selector("total_revenue", "SUM(transactions.amount)");

        let selector = ctx.get_selector("total_revenue").unwrap();
        assert_eq!(selector, "SUM(transactions.amount)");
    }

    #[test]
    fn test_with_aggregates() {
        let ctx = CompilationContext::new().with_aggregates(true);
        assert!(ctx.allow_aggregates);
    }

    #[test]
    fn test_column_type_checks() {
        assert!(ColumnType::Integer.is_numeric());
        assert!(ColumnType::Float.is_numeric());
        assert!(!ColumnType::String.is_numeric());
    }
}
