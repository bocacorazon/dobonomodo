//! Resolver rule evaluation engine.
//!
//! This module provides rule matching, period expansion, and template rendering
//! for resolving logical dataset/table requests into physical locations.
//!
//! # Example
//!
//! ```ignore
//! use dobo_core::resolver::context::ResolutionRequest;
//! use dobo_core::resolver::engine::resolve;
//!
//! let result = resolve(request, resolver, calendar, periods)?;
//! assert!(!result.locations.is_empty());
//! ```
pub mod calendar_matcher;
pub mod context;
pub mod diagnostics;
pub mod engine;
pub mod expander;
pub mod matcher;
pub mod renderer;

/// Resolver submodule identifier.
pub fn module_name() -> &'static str {
    "resolver"
}
