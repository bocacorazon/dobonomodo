// Diagnostic types for resolution tracing
// Defines ResolutionResult, ResolutionDiagnostic, and related types

use serde::{Deserialize, Serialize};

use crate::model::ResolvedLocation;

/// Result of a resolution operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolutionResult {
    pub locations: Vec<ResolvedLocation>,
    pub diagnostic: ResolutionDiagnostic,
}

/// Diagnostic information for troubleshooting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionDiagnostic {
    pub resolver_id: String,
    pub resolver_source: ResolverSource,
    pub evaluated_rules: Vec<RuleDiagnostic>,
    pub outcome: DiagnosticOutcome,
    pub expanded_periods: Vec<String>,
}

/// Per-rule evaluation details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleDiagnostic {
    pub rule_name: String,
    pub matched: bool,
    pub reason: String,
    pub evaluated_expression: Option<String>,
}

/// How the resolver was selected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolverSource {
    ProjectOverride,
    DatasetReference,
    SystemDefault,
}

/// Final outcome status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOutcome {
    Success,
    NoMatchingRule,
    PeriodExpansionFailure,
    TemplateRenderError,
}

impl ResolutionDiagnostic {
    /// Create a new diagnostic builder
    pub fn new(resolver_id: String, resolver_source: ResolverSource) -> Self {
        Self {
            resolver_id,
            resolver_source,
            evaluated_rules: Vec::new(),
            outcome: DiagnosticOutcome::Success,
            expanded_periods: Vec::new(),
        }
    }

    /// Add a rule evaluation to the diagnostic
    pub fn add_rule_diagnostic(&mut self, diagnostic: RuleDiagnostic) {
        self.evaluated_rules.push(diagnostic);
    }

    /// Set the outcome
    pub fn set_outcome(&mut self, outcome: DiagnosticOutcome) {
        self.outcome = outcome;
    }

    /// Set expanded periods
    pub fn set_expanded_periods(&mut self, periods: Vec<String>) {
        self.expanded_periods = periods;
    }
}

impl RuleDiagnostic {
    /// Create diagnostic for a matched rule
    pub fn matched(rule_name: String, reason: String, expression: Option<String>) -> Self {
        Self {
            rule_name,
            matched: true,
            reason,
            evaluated_expression: expression,
        }
    }

    /// Create diagnostic for a non-matched rule
    pub fn not_matched(rule_name: String, reason: String, expression: Option<String>) -> Self {
        Self {
            rule_name,
            matched: false,
            reason,
            evaluated_expression: expression,
        }
    }

    /// Create diagnostic for a skipped rule (earlier rule matched)
    pub fn skipped(rule_name: String) -> Self {
        Self {
            rule_name,
            matched: false,
            reason: format_rule_reason(None, false, true, None),
            evaluated_expression: None,
        }
    }
}

/// Build a no-match diagnostic with the canonical outcome.
pub fn build_no_match_diagnostic(mut diagnostic: ResolutionDiagnostic) -> ResolutionDiagnostic {
    diagnostic.outcome = DiagnosticOutcome::NoMatchingRule;
    diagnostic
}

/// Format a stable per-rule reason string for diagnostics.
pub fn format_rule_reason(
    expression: Option<&str>,
    matched: bool,
    skipped: bool,
    error: Option<&str>,
) -> String {
    if skipped {
        return "earlier rule already matched (rule not evaluated)".to_string();
    }
    if let Some(error) = error {
        return match expression {
            Some(expr) => format!("when: {} evaluation failed: {}", expr, error),
            None => format!("rule evaluation failed: {}", error),
        };
    }

    match (expression, matched) {
        (Some(expr), true) => format!("when: {} evaluated to true", expr),
        (Some(expr), false) => format!("when: {} evaluated to false", expr),
        (None, true) => "no when condition (unconditional match)".to_string(),
        (None, false) => "rule did not match".to_string(),
    }
}

/// Determine final diagnostic outcome for a resolution request.
pub fn determine_outcome(
    rule_matched: bool,
    period_expansion_failed: bool,
    template_render_failed: bool,
) -> DiagnosticOutcome {
    if period_expansion_failed {
        DiagnosticOutcome::PeriodExpansionFailure
    } else if template_render_failed {
        DiagnosticOutcome::TemplateRenderError
    } else if rule_matched {
        DiagnosticOutcome::Success
    } else {
        DiagnosticOutcome::NoMatchingRule
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_builder() {
        let mut diagnostic = ResolutionDiagnostic::new(
            "test_resolver".to_string(),
            ResolverSource::DatasetReference,
        );

        diagnostic.add_rule_diagnostic(RuleDiagnostic::matched(
            "rule1".to_string(),
            "matched".to_string(),
            Some("period >= '2024-Q1'".to_string()),
        ));

        diagnostic.set_outcome(DiagnosticOutcome::Success);
        diagnostic.set_expanded_periods(vec!["2024-01".to_string(), "2024-02".to_string()]);

        assert_eq!(diagnostic.resolver_id, "test_resolver");
        assert_eq!(diagnostic.evaluated_rules.len(), 1);
        assert_eq!(diagnostic.expanded_periods.len(), 2);
        assert_eq!(diagnostic.outcome, DiagnosticOutcome::Success);
    }

    #[test]
    fn test_build_no_match_diagnostic() {
        let diagnostic = ResolutionDiagnostic::new(
            "test_resolver".to_string(),
            ResolverSource::DatasetReference,
        );
        let built = build_no_match_diagnostic(diagnostic);
        assert_eq!(built.outcome, DiagnosticOutcome::NoMatchingRule);
    }

    #[test]
    fn test_format_rule_reason_variants() {
        assert_eq!(
            format_rule_reason(Some("table == 'sales'"), true, false, None),
            "when: table == 'sales' evaluated to true"
        );
        assert_eq!(
            format_rule_reason(Some("table == 'sales'"), false, false, None),
            "when: table == 'sales' evaluated to false"
        );
        assert_eq!(
            format_rule_reason(None, true, false, None),
            "no when condition (unconditional match)"
        );
        assert_eq!(
            format_rule_reason(None, false, true, None),
            "earlier rule already matched (rule not evaluated)"
        );
        assert!(
            format_rule_reason(Some("period >="), false, false, Some("unexpected end"))
                .contains("evaluation failed")
        );
    }

    #[test]
    fn test_determine_outcome() {
        assert_eq!(
            determine_outcome(true, false, false),
            DiagnosticOutcome::Success
        );
        assert_eq!(
            determine_outcome(false, false, false),
            DiagnosticOutcome::NoMatchingRule
        );
        assert_eq!(
            determine_outcome(true, true, false),
            DiagnosticOutcome::PeriodExpansionFailure
        );
        assert_eq!(
            determine_outcome(true, false, true),
            DiagnosticOutcome::TemplateRenderError
        );
    }
}
