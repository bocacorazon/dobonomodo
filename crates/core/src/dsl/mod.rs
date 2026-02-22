//! DSL Parser & Expression Compiler
//!
//! This module provides parsing, validation, and compilation functionality for
//! the DobONoMoDo expression DSL. It transforms expression strings into validated
//! Abstract Syntax Trees (ASTs) and compiles them into Polars `Expr` objects.
//!
//! # Architecture
//!
//! - **Parser**: Transforms expression strings into AST using pest grammar
//! - **Validator**: Type checking, column resolution, and semantic validation
//! - **Compiler**: Transforms validated AST into Polars expressions
//! - **Interpolation**: Expands {{SELECTOR}} references before parsing
//!
//! # Key APIs
//!
//! - [`parse_expression`]: parse source text into [`ExprAST`]
//! - [`interpolate_selectors`]: expand `{NAME}`/`{{NAME}}` references
//! - [`validate_expression`]: resolve columns and enforce type rules
//! - [`compile_expression`]: compile AST to Polars [`Expr`]
//! - [`compile_with_interpolation`]: full source-to-Expr pipeline
//!
//! # Example
//!
//! ```ignore
//! use dobo_core::dsl::{parse_expression, validate_expression, compile_expression};
//!
//! let ast = parse_expression("transactions.amount * 1.1")?;
//! let typed_ast = validate_expression(&ast, &schema, &context)?;
//! let polars_expr = compile_expression(&typed_ast)?;
//! ```
//!
//! ```ignore
//! use dobo_core::dsl::{compile_with_interpolation, CompilationContext, ColumnType};
//!
//! let mut context = CompilationContext::new().with_aggregates(true);
//! context.add_column("transactions.amount", ColumnType::Float);
//! context.add_selector("HIGH", "transactions.amount > 1000");
//!
//! let compiled = compile_with_interpolation("{HIGH} AND SUM(transactions.amount) > 0", &context)?;
//! let expr = compiled.into_expr();
//! ```

pub mod ast;
pub mod compiler;
pub mod context;
pub mod error;
pub mod interpolation;
pub mod parser;
pub mod types;
pub mod validation;

// Re-export main types and functions
pub use ast::*;
pub use compiler::{
    compile_expression, compile_expression_with_source, compile_with_interpolation,
    CompiledExpression,
};
pub use context::*;
pub use error::*;
pub use interpolation::interpolate_selectors;
pub use parser::{parse_expression, parse_expression_with_span, Span};
pub use types::*;
pub use validation::{infer_type, resolve_column, validate_expression};

/// Returns this module name (used by foundation compile checks).
pub fn module_name() -> &'static str {
    "dsl"
}
