//! Selector interpolation for expression sources.

use crate::dsl::context::CompilationContext;
use crate::dsl::error::ValidationError;
use std::collections::HashMap;

const MAX_INTERPOLATION_DEPTH: usize = 10;

/// Expand selector references in an expression source.
///
/// Supports `{{NAME}}` form.
///
/// Accepts either a selectors map (`&HashMap<String, String>`) or a
/// [`CompilationContext`] reference.
pub fn interpolate_selectors<S>(source: &str, selectors: &S) -> Result<String, ValidationError>
where
    S: SelectorLookup + ?Sized,
{
    let mut stack = Vec::new();
    interpolate_recursive(source, selectors, &mut stack, 0)
}

/// Selector source abstraction used by interpolation.
pub trait SelectorLookup {
    fn get_selector_expr(&self, name: &str) -> Option<&str>;
}

impl SelectorLookup for HashMap<String, String> {
    fn get_selector_expr(&self, name: &str) -> Option<&str> {
        self.get(name).map(|value| value.as_str())
    }
}

impl SelectorLookup for CompilationContext {
    fn get_selector_expr(&self, name: &str) -> Option<&str> {
        self.get_selector(name)
    }
}

fn interpolate_recursive<S>(
    source: &str,
    selectors: &S,
    stack: &mut Vec<String>,
    depth: usize,
) -> Result<String, ValidationError>
where
    S: SelectorLookup + ?Sized,
{
    if depth > MAX_INTERPOLATION_DEPTH {
        return Err(ValidationError::MaxInterpolationDepth {
            max_depth: MAX_INTERPOLATION_DEPTH,
        });
    }

    let mut output = String::with_capacity(source.len());
    let mut i = 0usize;
    let mut literal_start = 0usize;
    let mut changed = false;
    let bytes = source.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let start = i + 2;
            if let Some((end, next_index)) = find_selector_end(source, start) {
                output.push_str(&source[literal_start..i]);

                let raw_name = source[start..end].trim();
                if raw_name.is_empty() {
                    return Err(ValidationError::UnresolvedSelectorRef {
                        selector: String::new(),
                    });
                }

                if stack.iter().any(|s| s == raw_name) {
                    let mut cycle = stack.join(" -> ");
                    if !cycle.is_empty() {
                        cycle.push_str(" -> ");
                    }
                    cycle.push_str(raw_name);
                    return Err(ValidationError::CircularSelectorRef { cycle });
                }

                let selector_expr = selectors.get_selector_expr(raw_name).ok_or_else(|| {
                    ValidationError::UnresolvedSelectorRef {
                        selector: raw_name.to_string(),
                    }
                })?;

                stack.push(raw_name.to_string());
                let expanded = interpolate_recursive(selector_expr, selectors, stack, depth + 1)?;
                stack.pop();

                output.push_str(&expanded);
                i = next_index;
                literal_start = i;
                changed = true;
                continue;
            }
        }

        i += 1;
    }

    output.push_str(&source[literal_start..]);

    if changed {
        // A selector can expand into additional selectors.
        interpolate_recursive(&output, selectors, stack, depth + 1)
    } else {
        Ok(output)
    }
}

fn find_selector_end(source: &str, start: usize) -> Option<(usize, usize)> {
    let mut i = start;
    let bytes = source.as_bytes();
    while i + 1 < bytes.len() {
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            return Some((i, i + 2));
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::context::{ColumnType, CompilationContext};
    use std::collections::HashSet;

    #[test]
    fn expands_simple_selector() {
        let mut ctx = CompilationContext::new();
        ctx.add_column("orders.amount", ColumnType::Float);
        ctx.add_selector("A", "orders.amount");

        let expanded = interpolate_selectors("{{A}}", &ctx).unwrap();
        assert_eq!(expanded, "orders.amount");
    }

    #[test]
    fn detects_circular_selector() {
        let mut ctx = CompilationContext::new();
        ctx.add_selector("A", "{{B}}");
        ctx.add_selector("B", "{{A}}");

        let err = interpolate_selectors("{{A}}", &ctx).unwrap_err();
        assert!(matches!(err, ValidationError::CircularSelectorRef { .. }));
    }

    #[test]
    fn expands_double_brace_selector() {
        let mut ctx = CompilationContext::new();
        ctx.add_selector("ACTIVE", "TRUE");

        let expanded = interpolate_selectors("{{ACTIVE}}", &ctx).unwrap();
        assert_eq!(expanded, "TRUE");
    }

    #[test]
    fn collects_nested_selectors() {
        let mut ctx = CompilationContext::new();
        ctx.add_selector("A", "{{B}}");
        ctx.add_selector("B", "{{C}}");
        ctx.add_selector("C", "TRUE");

        let expanded = interpolate_selectors("{{A}}", &ctx).unwrap();
        assert_eq!(expanded, "TRUE");
    }

    #[test]
    fn unresolved_selector_fails() {
        let ctx = CompilationContext::new();
        let err = interpolate_selectors("{{MISSING}}", &ctx).unwrap_err();
        assert!(matches!(err, ValidationError::UnresolvedSelectorRef { .. }));
    }

    #[test]
    fn empty_selector_fails() {
        let ctx = CompilationContext::new();
        let err = interpolate_selectors("{{}}", &ctx).unwrap_err();
        assert!(matches!(err, ValidationError::UnresolvedSelectorRef { .. }));
    }

    #[test]
    fn depth_limit_fails() {
        let mut ctx = CompilationContext::new();
        let mut seen = HashSet::new();
        for idx in 0..15 {
            let current = format!("S{idx}");
            let next = format!("S{}", idx + 1);
            seen.insert(current.clone());
            ctx.add_selector(current, format!("{{{{{next}}}}}"));
        }
        for key in &seen {
            assert!(ctx.get_selector(key).is_some());
        }
        let err = interpolate_selectors("{{S0}}", &ctx).unwrap_err();
        assert!(matches!(err, ValidationError::MaxInterpolationDepth { .. }));
    }

    #[test]
    fn preserves_utf8_content_while_expanding_selector() {
        let mut ctx = CompilationContext::new();
        ctx.add_selector("A", "orders.amount");

        let expanded = interpolate_selectors("café {{A}}", &ctx).unwrap();
        assert_eq!(expanded, "café orders.amount");
    }

    #[test]
    fn single_brace_tokens_are_left_as_literal_text() {
        let mut ctx = CompilationContext::new();
        ctx.add_selector("A", "orders.amount");

        let expanded = interpolate_selectors("{A}", &ctx).unwrap();
        assert_eq!(expanded, "{A}");
    }
}
