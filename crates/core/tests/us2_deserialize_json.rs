mod common;

use dobo_core::model::{
    Calendar, DataSource, Dataset, Expression, Period, Project, Resolver, Run,
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

    let _: Dataset = parse_entity(&root, "dataset");
    let _: Project = parse_entity(&root, "project");
    let _: Run = parse_entity(&root, "run");
    let _: Resolver = parse_entity(&root, "resolver");
    let _: Calendar = parse_entity(&root, "calendar");
    let _: Period = parse_entity(&root, "period");
    let _: DataSource = parse_entity(&root, "datasource");
    let _: Expression = parse_entity(&root, "expression");
}
