// Resolution context types
// Defines ResolutionRequest and ResolutionContext structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::model::{Calendar, Period};
use crate::resolver::calendar_matcher::CalendarMatcher;
use crate::resolver::diagnostics::ResolverSource;

/// Input to the resolution engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionRequest {
    pub dataset_id: String,
    pub table_name: String,
    pub period_id: Uuid,
    pub project_id: Option<String>,
}

/// Enriched context for rule evaluation and template rendering
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionContext {
    pub dataset_id: String,
    pub table_name: String,
    pub period: Period,
    pub period_level: String,
    pub resolver_source: Option<ResolverSource>,
    pub additional_context: HashMap<String, String>,
}

/// Build ResolutionContext from request and loaded metadata
pub fn build_context(
    request: &ResolutionRequest,
    period: Period,
    calendar: &Calendar,
) -> Result<ResolutionContext, String> {
    let mut matcher = CalendarMatcher::new(calendar);
    build_context_with_matcher(request, period, &mut matcher)
}

/// Build ResolutionContext from request and loaded metadata using a reusable matcher
pub fn build_context_with_matcher(
    request: &ResolutionRequest,
    period: Period,
    matcher: &mut CalendarMatcher,
) -> Result<ResolutionContext, String> {
    // Find period level from calendar
    let period_level = find_period_level(&period, matcher)?;

    Ok(ResolutionContext {
        dataset_id: request.dataset_id.clone(),
        table_name: request.table_name.clone(),
        period,
        period_level,
        resolver_source: None,
        additional_context: HashMap::new(),
    })
}

/// Find the level name for a period
fn find_period_level(period: &Period, matcher: &mut CalendarMatcher) -> Result<String, String> {
    matcher
        .find_level_strict(&period.identifier)?
        .ok_or_else(|| {
            format!(
                "cannot determine level for period '{}' using calendar identifier patterns",
                period.identifier,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CalendarStatus, LevelDef, PeriodStatus};

    #[test]
    fn test_build_context() {
        let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();

        let request = ResolutionRequest {
            dataset_id: "sales".to_string(),
            table_name: "transactions".to_string(),
            period_id,
            project_id: None,
        };

        let period = Period {
            id: period_id,
            identifier: "2024-Q1".to_string(),
            name: "Q1 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-03-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        };

        let calendar = Calendar {
            id: calendar_id,
            name: "Test".to_string(),
            description: None,
            status: CalendarStatus::Active,
            is_default: true,
            levels: vec![LevelDef {
                name: "quarter".to_string(),
                parent_level: None,
                identifier_pattern: Some(r"^\d{4}-Q[1-4]$".to_string()),
                date_rules: vec![],
            }],
            created_at: None,
            updated_at: None,
        };

        let context = build_context(&request, period, &calendar).unwrap();
        assert_eq!(context.dataset_id, "sales");
        assert_eq!(context.table_name, "transactions");
        assert_eq!(context.period_level, "quarter");
    }

    #[test]
    fn test_build_context_fails_when_no_calendar_pattern_matches() {
        let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();

        let request = ResolutionRequest {
            dataset_id: "sales".to_string(),
            table_name: "transactions".to_string(),
            period_id,
            project_id: None,
        };

        let period = Period {
            id: period_id,
            identifier: "2024-Q1".to_string(),
            name: "Q1 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-03-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        };

        let calendar = Calendar {
            id: calendar_id,
            name: "Test".to_string(),
            description: None,
            status: CalendarStatus::Active,
            is_default: true,
            levels: vec![LevelDef {
                name: "month".to_string(),
                parent_level: None,
                identifier_pattern: Some(r"^\d{4}-(0[1-9]|1[0-2])$".to_string()),
                date_rules: vec![],
            }],
            created_at: None,
            updated_at: None,
        };

        let error = build_context(&request, period, &calendar).unwrap_err();
        assert!(error.contains("cannot determine level"));
    }

    #[test]
    fn test_build_context_fails_for_invalid_level_pattern() {
        let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();

        let request = ResolutionRequest {
            dataset_id: "sales".to_string(),
            table_name: "transactions".to_string(),
            period_id,
            project_id: None,
        };

        let period = Period {
            id: period_id,
            identifier: "2024-Q1".to_string(),
            name: "Q1 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-03-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        };

        let calendar = Calendar {
            id: calendar_id,
            name: "Test".to_string(),
            description: None,
            status: CalendarStatus::Active,
            is_default: true,
            levels: vec![LevelDef {
                name: "quarter".to_string(),
                parent_level: None,
                identifier_pattern: Some("[".to_string()),
                date_rules: vec![],
            }],
            created_at: None,
            updated_at: None,
        };

        let error = build_context(&request, period, &calendar).unwrap_err();
        assert!(error.contains("invalid identifier_pattern"));
        assert!(error.contains("quarter"));
    }
}
