mod common;

use dobo_core::model::{Calendar, DataSource, Dataset, Expression, Period, Project, Resolver, Run};
use serde::de::DeserializeOwned;
use serde_yaml::Value;

fn parse_entity<T: DeserializeOwned>(root: &Value, key: &str) -> T {
    serde_yaml::from_value(root.get(key).cloned().expect("entity key should exist"))
        .expect("entity should deserialize")
}

#[test]
fn yaml_deserializes_required_entities() {
    let fixture = common::read_fixture("entities.yaml");
    let root: Value = serde_yaml::from_str(&fixture).expect("yaml should parse");

    let _: Dataset = parse_entity(&root, "dataset");
    let _: Project = parse_entity(&root, "project");
    let _: Run = parse_entity(&root, "run");
    let _: Resolver = parse_entity(&root, "resolver");
    let _: Calendar = parse_entity(&root, "calendar");
    let _: Period = parse_entity(&root, "period");
    let _: DataSource = parse_entity(&root, "datasource");
    let _: Expression = parse_entity(&root, "expression");
}
