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
//! # Example
//!
//! ```ignore
//! use dobo_core::dsl::{parse_expression, validate_expression, compile_expression};
//!
//! let ast = parse_expression("transactions.amount * 1.1")?;
//! let typed_ast = validate_expression(&ast, &schema, &context)?;
//! let polars_expr = compile_expression(&typed_ast)?;
//! ```

pub mod ast;
pub mod context;
pub mod error;
pub mod parser;
pub mod types;

// Re-export main types and functions
pub use ast::*;
pub use context::*;
pub use error::*;
pub use parser::{parse_expression, parse_expression_with_span, Span};
pub use types::*;
