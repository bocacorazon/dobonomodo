mod common;

use dobo_core::model::{
    Calendar, ColumnDef, DataSource, Dataset, Expression, LookupDef, OperationInstance, Period,
    Project, ProjectSnapshot, ResolutionRule, ResolutionStrategy, ResolvedLocation, Resolver,
    ResolverSnapshot, Run, TableRef,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

fn parse_entity<T: DeserializeOwned>(root: &Value, key: &str) -> T {
    serde_json::from_value(root.get(key).cloned().expect("entity key should exist"))
        .expect("entity should deserialize")
}

#[test]
fn json_deserializes_required_entities() {
    let fixture = common::read_fixture("entities.json");
    let root: Value = serde_json::from_str(&fixture).expect("json should parse");

    let dataset: Dataset = parse_entity(&root, "dataset");
    let project: Project = parse_entity(&root, "project");
    let run: Run = parse_entity(&root, "run");
    let resolver: Resolver = parse_entity(&root, "resolver");
    let calendar: Calendar = parse_entity(&root, "calendar");
    let period: Period = parse_entity(&root, "period");
    let datasource: DataSource = parse_entity(&root, "datasource");
    let expression: Expression = parse_entity(&root, "expression");
    let column_def: ColumnDef = parse_entity(&root, "column_def");
    let table_ref: TableRef = parse_entity(&root, "table_ref");
    let lookup_def: LookupDef = parse_entity(&root, "lookup_def");
    let operation_instance: OperationInstance = parse_entity(&root, "operation_instance");
    let resolver_snapshot: ResolverSnapshot = parse_entity(&root, "resolver_snapshot");
    let project_snapshot: ProjectSnapshot = parse_entity(&root, "project_snapshot");
    let resolution_rule: ResolutionRule = parse_entity(&root, "resolution_rule");
    let resolution_strategy: ResolutionStrategy = parse_entity(&root, "resolution_strategy");
    let resolved_location: ResolvedLocation = parse_entity(&root, "resolved_location");

    assert_eq!(dataset.main_table.name, "transactions");
    assert_eq!(project.operations.len(), 1);
    assert_eq!(run.project_snapshot.resolver_snapshots.len(), 1);
    assert_eq!(resolver.rules.len(), 1);
    assert_eq!(calendar.name, "Gregorian");
    assert_eq!(period.identifier, "2026-01");
    assert_eq!(datasource.name, "warehouse");
    assert_eq!(expression.source, "transactions.amount");
    assert_eq!(column_def.name, "amount");
    assert_eq!(table_ref.columns.len(), 1);
    assert_eq!(lookup_def.join_conditions.len(), 1);
    assert_eq!(operation_instance.order, 2);
    assert_eq!(resolver_snapshot.resolver_version, 1);
    assert_eq!(project_snapshot.resolver_snapshots.len(), 1);
    assert_eq!(resolution_rule.name, "from-catalog");
    assert!(matches!(
        resolution_rule.strategy,
        ResolutionStrategy::Catalog { .. }
    ));
    assert!(matches!(
        resolution_strategy,
        ResolutionStrategy::Catalog { .. }
    ));
    assert!(resolved_location.catalog_response.is_some());
}
