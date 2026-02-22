//! Abstract Syntax Tree (AST) definitions for DSL expressions

use serde::{Deserialize, Serialize};

/// Represents a parsed expression as a typed tree structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExprAST {
    /// Literal value (number, string, boolean, date, NULL)
    Literal(LiteralValue),

    /// Column reference (table.column)
    ColumnRef { table: String, column: String },

    /// Binary operation (arithmetic, comparison, logical)
    BinaryOp {
        op: BinaryOperator,
        left: Box<ExprAST>,
        right: Box<ExprAST>,
    },

    /// Unary operation (NOT, negation)
    UnaryOp {
        op: UnaryOperator,
        operand: Box<ExprAST>,
    },

    /// Function call (e.g., SUM(amount), CONCAT(first, last))
    FunctionCall { name: String, args: Vec<ExprAST> },
}

/// Literal value types in the DSL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    /// Number (integer or floating point)
    Number(f64),

    /// String literal
    String(String),

    /// Boolean literal
    Boolean(bool),

    /// Date literal
    Date(chrono::NaiveDate),

    /// NULL literal
    Null,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /

    // Comparison
    Equal,              // =
    NotEqual,           // <>
    LessThan,           // <
    LessThanOrEqual,    // <=
    GreaterThan,        // >
    GreaterThanOrEqual, // >=

    // Logical
    And, // AND
    Or,  // OR
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    /// Logical NOT
    Not,

    /// Arithmetic negation
    Negate,
}

impl ExprAST {
    /// Create a literal number node
    pub fn number(value: f64) -> Self {
        ExprAST::Literal(LiteralValue::Number(value))
    }

    /// Create a literal string node
    pub fn string(value: impl Into<String>) -> Self {
        ExprAST::Literal(LiteralValue::String(value.into()))
    }

    /// Create a literal boolean node
    pub fn boolean(value: bool) -> Self {
        ExprAST::Literal(LiteralValue::Boolean(value))
    }

    /// Create a NULL node
    pub fn null() -> Self {
        ExprAST::Literal(LiteralValue::Null)
    }

    /// Create a date literal node
    pub fn date(value: chrono::NaiveDate) -> Self {
        ExprAST::Literal(LiteralValue::Date(value))
    }

    /// Create a column reference node
    pub fn column_ref(table: impl Into<String>, column: impl Into<String>) -> Self {
        ExprAST::ColumnRef {
            table: table.into(),
            column: column.into(),
        }
    }

    /// Create a binary operation node
    pub fn binary_op(op: BinaryOperator, left: ExprAST, right: ExprAST) -> Self {
        ExprAST::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Create a unary operation node
    pub fn unary_op(op: UnaryOperator, operand: ExprAST) -> Self {
        ExprAST::UnaryOp {
            op,
            operand: Box::new(operand),
        }
    }

    /// Create a function call node
    pub fn function_call(name: impl Into<String>, args: Vec<ExprAST>) -> Self {
        ExprAST::FunctionCall {
            name: name.into(),
            args,
        }
    }
}

impl std::fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Equal => "=",
            BinaryOperator::NotEqual => "<>",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessThanOrEqual => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterThanOrEqual => ">=",
            BinaryOperator::And => "AND",
            BinaryOperator::Or => "OR",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UnaryOperator::Not => "NOT",
            UnaryOperator::Negate => "-",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_constructors() {
        assert_eq!(
            ExprAST::number(42.0),
            ExprAST::Literal(LiteralValue::Number(42.0))
        );
        assert_eq!(
            ExprAST::string("hello"),
            ExprAST::Literal(LiteralValue::String("hello".to_string()))
        );
        assert_eq!(
            ExprAST::boolean(true),
            ExprAST::Literal(LiteralValue::Boolean(true))
        );
        assert_eq!(ExprAST::null(), ExprAST::Literal(LiteralValue::Null));
    }

    #[test]
    fn test_column_ref() {
        let col = ExprAST::column_ref("transactions", "amount");
        match col {
            ExprAST::ColumnRef { table, column } => {
                assert_eq!(table, "transactions");
                assert_eq!(column, "amount");
            }
            _ => panic!("Expected ColumnRef"),
        }
    }

    #[test]
    fn test_binary_op() {
        let expr = ExprAST::binary_op(
            BinaryOperator::Add,
            ExprAST::number(1.0),
            ExprAST::number(2.0),
        );
        match expr {
            ExprAST::BinaryOp { op, left, right } => {
                assert_eq!(op, BinaryOperator::Add);
                assert_eq!(*left, ExprAST::number(1.0));
                assert_eq!(*right, ExprAST::number(2.0));
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_operator_display() {
        assert_eq!(BinaryOperator::Add.to_string(), "+");
        assert_eq!(BinaryOperator::Equal.to_string(), "=");
        assert_eq!(UnaryOperator::Not.to_string(), "NOT");
    }
}
