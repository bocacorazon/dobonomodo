//! Compilation context for expression validation and compilation

use std::collections::HashMap;

/// Compilation context providing schema and selector information
#[derive(Debug, Clone)]
pub struct CompilationContext {
    /// Schema information: map of table.column -> column type
    pub schema: HashMap<String, ColumnInfo>,

    /// Join aliases: map of alias -> logical table name.
    pub join_aliases: HashMap<String, String>,

    /// Selector definitions: map of selector name -> expression string
    pub selectors: HashMap<String, String>,

    /// Whether aggregate functions are allowed in this context
    pub allow_aggregates: bool,

    /// Reference date used for TODAY() resolution.
    pub today: Option<chrono::NaiveDate>,
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
            join_aliases: HashMap::new(),
            selectors: HashMap::new(),
            allow_aggregates: false,
            today: None,
        }
    }

    /// Create a new compilation context with schema
    pub fn with_schema(schema: HashMap<String, ColumnInfo>) -> Self {
        Self {
            schema,
            join_aliases: HashMap::new(),
            selectors: HashMap::new(),
            allow_aggregates: false,
            today: None,
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

    /// Add a join alias mapping.
    pub fn add_join_alias(&mut self, alias: impl Into<String>, table: impl Into<String>) {
        self.join_aliases.insert(alias.into(), table.into());
    }

    /// Set whether aggregates are allowed
    pub fn with_aggregates(mut self, allow: bool) -> Self {
        self.allow_aggregates = allow;
        self
    }

    /// Set the reference date used by TODAY().
    pub fn with_today(mut self, today: chrono::NaiveDate) -> Self {
        self.today = Some(today);
        self
    }

    /// Set join aliases as alias -> table mappings.
    pub fn with_join_aliases(mut self, aliases: HashMap<String, String>) -> Self {
        self.join_aliases = aliases;
        self
    }

    /// Look up a column in the schema
    pub fn get_column(&self, name: &str) -> Option<&ColumnInfo> {
        self.schema.get(name)
    }

    /// Resolve a table/column reference using direct table names and configured join aliases.
    pub fn resolve_column(&self, table: &str, column: &str) -> Option<&ColumnInfo> {
        let qualified = format!("{table}.{column}");
        if let Some(info) = self.schema.get(&qualified) {
            return Some(info);
        }

        if let Some(target_table) = self.join_aliases.get(table) {
            let alias_qualified = format!("{target_table}.{column}");
            return self.schema.get(&alias_qualified);
        }

        None
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
        assert!(ctx.join_aliases.is_empty());
        assert!(ctx.selectors.is_empty());
        assert!(!ctx.allow_aggregates);
        assert!(ctx.today.is_none());
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
    fn test_with_today() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date");
        let ctx = CompilationContext::new().with_today(today);
        assert_eq!(ctx.today, Some(today));
    }

    #[test]
    fn test_column_type_checks() {
        assert!(ColumnType::Integer.is_numeric());
        assert!(ColumnType::Float.is_numeric());
        assert!(!ColumnType::String.is_numeric());
    }

    #[test]
    fn test_resolve_column_with_alias() {
        let mut ctx = CompilationContext::new();
        ctx.add_column("transactions.amount", ColumnType::Float);
        ctx.add_join_alias("t", "transactions");

        let resolved = ctx.resolve_column("t", "amount");
        assert!(resolved.is_some());
        assert_eq!(
            resolved.expect("alias should resolve").column_type,
            ColumnType::Float
        );
    }
}
