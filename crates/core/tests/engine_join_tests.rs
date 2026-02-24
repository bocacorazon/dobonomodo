//! Unit tests for RuntimeJoin resolution and execution
//! Tests cover dataset version resolution, resolver precedence, and join mechanics

#[path = "fixtures/in_memory_metadata_store.rs"]
mod in_memory_metadata_store;

use dobo_core::engine::join::*;
use dobo_core::model::{ColumnDef, ColumnType, Dataset, DatasetStatus, TableRef, TemporalMode};
use std::collections::BTreeMap;
use uuid::Uuid;

use in_memory_metadata_store::InMemoryMetadataStore;

fn make_dataset(
    id: Uuid,
    version: i32,
    status: DatasetStatus,
    resolver_id: Option<&str>,
) -> Dataset {
    Dataset {
        id,
        name: "test_dataset".to_string(),
        description: None,
        owner: "test_owner".to_string(),
        version,
        status,
        resolver_id: resolver_id.map(|s| s.to_string()),
        main_table: TableRef {
            name: "main".to_string(),
            temporal_mode: Some(TemporalMode::Bitemporal),
            columns: vec![ColumnDef {
                name: "id".to_string(),
                column_type: ColumnType::Integer,
                nullable: Some(false),
                description: None,
            }],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn test_pinned_version_resolution() {
    // T012: Dataset version resolution with pinned version
    let dataset_id = Uuid::new_v4();
    let dataset_v3 = make_dataset(dataset_id, 3, DatasetStatus::Active, Some("s3-resolver"));
    let store = InMemoryMetadataStore::new().with_dataset(dataset_v3);

    let result = resolve_dataset_version(&dataset_id, Some(3), &store);
    assert!(result.is_ok());
    let (resolved_dataset, resolved_version) = result.unwrap();
    assert_eq!(resolved_version, 3);
    assert_eq!(resolved_dataset.id, dataset_id);
}

#[test]
fn test_latest_active_version_resolution() {
    // T013: Dataset version resolution with latest active
    let dataset_id = Uuid::new_v4();
    let dataset_v10 = make_dataset(dataset_id, 10, DatasetStatus::Active, Some("s3-resolver"));
    let store = InMemoryMetadataStore::new().with_dataset(dataset_v10);

    let result = resolve_dataset_version(&dataset_id, None, &store);
    assert!(result.is_ok());
    let (resolved_dataset, resolved_version) = result.unwrap();
    assert_eq!(resolved_version, 10);
    assert_eq!(resolved_dataset.status, DatasetStatus::Active);
}

#[test]
fn test_dataset_not_found_error() {
    let dataset_id = Uuid::new_v4();
    let store = InMemoryMetadataStore::new();
    let result = resolve_dataset_version(&dataset_id, None, &store);
    assert!(result.is_err());
    match result {
        Err(JoinError::DatasetNotFound(id)) => assert_eq!(id, dataset_id),
        _ => panic!("Expected DatasetNotFound error"),
    }
}

#[test]
fn test_dataset_disabled_error() {
    let dataset_id = Uuid::new_v4();
    let disabled_dataset = make_dataset(dataset_id, 2, DatasetStatus::Disabled, None);
    let store = InMemoryMetadataStore::new().with_dataset(disabled_dataset);

    let result = resolve_dataset_version(&dataset_id, None, &store);
    assert!(result.is_err());
    match result {
        Err(JoinError::DatasetDisabled(id)) => assert_eq!(id, dataset_id),
        _ => panic!("Expected DatasetDisabled error"),
    }
}

#[test]
fn test_version_not_found_error() {
    let dataset_id = Uuid::new_v4();
    let store = InMemoryMetadataStore::new().with_dataset(make_dataset(
        dataset_id,
        1,
        DatasetStatus::Active,
        Some("resolver"),
    ));
    let result = resolve_dataset_version(&dataset_id, Some(99), &store);
    assert!(result.is_err());
    match result {
        Err(JoinError::VersionNotFound {
            dataset_id: id,
            version,
        }) => {
            assert_eq!(id, dataset_id);
            assert_eq!(version, 99);
        }
        _ => panic!("Expected VersionNotFound error"),
    }
}

#[test]
fn test_resolver_precedence_project_override() {
    // Project override takes precedence over all
    let dataset_id = Uuid::new_v4();
    let mut project_overrides = BTreeMap::new();
    project_overrides.insert(dataset_id, "test-resolver".to_string());

    let resolver_id = resolve_resolver_id(
        &dataset_id,
        Some("dataset-resolver"),
        &project_overrides,
        "system-default",
    );

    assert_eq!(resolver_id, "test-resolver");
}

#[test]
fn test_resolver_precedence_dataset_fallback() {
    // Dataset resolver_id when no project override
    let dataset_id = Uuid::new_v4();
    let project_overrides = BTreeMap::new();

    let resolver_id = resolve_resolver_id(
        &dataset_id,
        Some("dataset-resolver"),
        &project_overrides,
        "system-default",
    );

    assert_eq!(resolver_id, "dataset-resolver");
}

#[test]
fn test_resolver_precedence_system_default() {
    // System default when no project override and no dataset resolver_id
    let dataset_id = Uuid::new_v4();
    let project_overrides = BTreeMap::new();

    let resolver_id = resolve_resolver_id(&dataset_id, None, &project_overrides, "system-default");

    assert_eq!(resolver_id, "system-default");
}
