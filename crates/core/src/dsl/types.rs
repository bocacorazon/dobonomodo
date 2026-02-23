//! Type system for DSL expressions

use crate::dsl::ast::ExprAST;
use serde::{Deserialize, Serialize};

/// Expression type for type checking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExprType {
    /// Numeric type (integers and floats)
    Number,

    /// String/text type
    String,

    /// Boolean type
    Boolean,

    /// Date type
    Date,

    /// NULL type (compatible with any type)
    Null,

    /// Any type (used when type cannot be determined)
    Any,
}

/// A typed expression AST with inferred return type
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExprAST {
    /// The underlying AST
    pub ast: ExprAST,

    /// The inferred return type of the expression
    pub return_type: ExprType,
}

impl TypedExprAST {
    /// Create a new typed expression
    pub fn new(ast: ExprAST, return_type: ExprType) -> Self {
        Self { ast, return_type }
    }
}

impl ExprType {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &ExprType) -> bool {
        match (self, other) {
            // Null is compatible with everything
            (ExprType::Null, _) | (_, ExprType::Null) => true,
            // Any is compatible with everything
            (ExprType::Any, _) | (_, ExprType::Any) => true,
            // Same types are compatible
            (a, b) if a == b => true,
            // Otherwise incompatible
            _ => false,
        }
    }

    /// Get a human-readable name for the type
    pub fn name(&self) -> &'static str {
        match self {
            ExprType::Number => "Number",
            ExprType::String => "String",
            ExprType::Boolean => "Boolean",
            ExprType::Date => "Date",
            ExprType::Null => "NULL",
            ExprType::Any => "Any",
        }
    }

    /// Check if this is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, ExprType::Number | ExprType::Null | ExprType::Any)
    }

    /// Check if this is a boolean type
    pub fn is_boolean(&self) -> bool {
        matches!(self, ExprType::Boolean | ExprType::Null | ExprType::Any)
    }

    /// Check if this is a string type
    pub fn is_string(&self) -> bool {
        matches!(self, ExprType::String | ExprType::Null | ExprType::Any)
    }

    /// Check if this is a date type
    pub fn is_date(&self) -> bool {
        matches!(self, ExprType::Date | ExprType::Null | ExprType::Any)
    }
}

impl std::fmt::Display for ExprType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::LiteralValue;

    #[test]
    fn test_type_compatibility() {
        assert!(ExprType::Number.is_compatible_with(&ExprType::Number));
        assert!(ExprType::Null.is_compatible_with(&ExprType::Number));
        assert!(ExprType::Number.is_compatible_with(&ExprType::Null));
        assert!(ExprType::Any.is_compatible_with(&ExprType::String));
        assert!(!ExprType::Number.is_compatible_with(&ExprType::String));
    }

    #[test]
    fn test_type_checks() {
        assert!(ExprType::Number.is_numeric());
        assert!(!ExprType::String.is_numeric());
        assert!(ExprType::Boolean.is_boolean());
        assert!(!ExprType::Number.is_boolean());
        assert!(ExprType::String.is_string());
        assert!(ExprType::Date.is_date());
    }

    #[test]
    fn test_typed_expr_ast() {
        let ast = ExprAST::Literal(LiteralValue::Number(42.0));
        let typed = TypedExprAST::new(ast.clone(), ExprType::Number);
        assert_eq!(typed.ast, ast);
        assert_eq!(typed.return_type, ExprType::Number);
    }

    #[test]
    fn test_type_display() {
        assert_eq!(ExprType::Number.to_string(), "Number");
        assert_eq!(ExprType::String.to_string(), "String");
        assert_eq!(ExprType::Boolean.to_string(), "Boolean");
    }
}
