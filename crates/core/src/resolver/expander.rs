// Period expander - expands periods using calendar hierarchy
// Implements period expansion logic for resolving to finer data levels

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::model::{Calendar, Period};
use crate::resolver::calendar_matcher::CalendarMatcher;

/// Resolver submodule identifier.
pub fn module_name() -> &'static str {
    "expander"
}

/// Expand a requested period to the target data level.
pub fn expand_period(
    requested_period: &Period,
    requested_level: &str,
    target_level: &str,
    calendar: &Calendar,
    periods: &[Period],
) -> Result<Vec<Period>, String> {
    if target_level == "any" || requested_level == target_level {
        return Ok(vec![requested_period.clone()]);
    }

    if !level_exists(calendar, target_level) {
        return Err(format!(
            "target level '{}' does not exist in calendar",
            target_level
        ));
    }

    if !is_descendant_level(calendar, requested_level, target_level) {
        return Err(format!(
            "cannot expand from level '{}' to non-descendant level '{}'",
            requested_level, target_level
        ));
    }

    let periods_by_id: HashMap<Uuid, &Period> = periods.iter().map(|p| (p.id, p)).collect();
    let mut matcher = CalendarMatcher::new(calendar);
    let mut children_by_parent: HashMap<Uuid, Vec<&Period>> = HashMap::new();
    for period in periods {
        if let Some(parent_id) = period.parent_id {
            children_by_parent
                .entry(parent_id)
                .or_default()
                .push(period);
        }
    }
    for children in children_by_parent.values_mut() {
        children.sort_by(|a, b| {
            a.sequence
                .cmp(&b.sequence)
                .then_with(|| a.identifier.cmp(&b.identifier))
        });
    }

    let mut result = Vec::new();
    let mut visited = HashSet::new();
    collect_descendants_at_level(
        requested_period.id,
        target_level,
        &mut matcher,
        &periods_by_id,
        &children_by_parent,
        &mut visited,
        &mut result,
    )?;

    if result.is_empty() {
        return Err(format!(
            "no descendant periods found for '{}' at target level '{}'",
            requested_period.identifier, target_level
        ));
    }

    Ok(result)
}

fn collect_descendants_at_level(
    parent_id: Uuid,
    target_level: &str,
    matcher: &mut CalendarMatcher,
    periods_by_id: &HashMap<Uuid, &Period>,
    children_by_parent: &HashMap<Uuid, Vec<&Period>>,
    visited: &mut HashSet<Uuid>,
    result: &mut Vec<Period>,
) -> Result<(), String> {
    if !visited.insert(parent_id) {
        return Err(format!(
            "cycle detected while expanding period tree at {}",
            parent_id
        ));
    }

    let Some(children) = children_by_parent.get(&parent_id) else {
        visited.remove(&parent_id);
        return Ok(());
    };

    for child in children {
        let level = infer_period_level(child, matcher);
        if level.as_deref() == Some(target_level) {
            result.push((*child).clone());
            continue;
        }

        if periods_by_id.contains_key(&child.id) {
            collect_descendants_at_level(
                child.id,
                target_level,
                matcher,
                periods_by_id,
                children_by_parent,
                visited,
                result,
            )?;
        }
    }

    visited.remove(&parent_id);
    Ok(())
}

fn level_exists(calendar: &Calendar, level_name: &str) -> bool {
    calendar.levels.iter().any(|level| level.name == level_name)
}

fn is_descendant_level(calendar: &Calendar, ancestor: &str, descendant: &str) -> bool {
    if ancestor == descendant {
        return true;
    }

    let level_map: HashMap<&str, Option<&str>> = calendar
        .levels
        .iter()
        .map(|level| (level.name.as_str(), level.parent_level.as_deref()))
        .collect();

    let mut current = Some(descendant);
    let mut visited = HashSet::new();
    while let Some(level_name) = current {
        if !visited.insert(level_name) {
            return false;
        }

        if let Some(parent_level) = level_map.get(level_name).and_then(|p| *p) {
            if parent_level == ancestor {
                return true;
            }
            current = Some(parent_level);
        } else {
            current = None;
        }
    }

    false
}

fn infer_period_level(period: &Period, matcher: &mut CalendarMatcher) -> Option<String> {
    matcher.find_level_lenient(&period.identifier)
}
