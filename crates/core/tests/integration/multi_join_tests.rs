use dobo_core::engine::join::apply_runtime_joins;
use dobo_core::model::{Expression, RuntimeJoin};
use polars::prelude::IntoLazy;
use uuid::Uuid;

use crate::sample_datasets;

#[test]
fn dual_join_enriches_customers_and_products() {
    let working = sample_datasets::gl_transactions_frame().lazy();

    let joins = vec![
        RuntimeJoin {
            alias: "customers".to_string(),
            dataset_id: Uuid::new_v4(),
            dataset_version: None,
            on: Expression {
                source: "customer_id = customers.id".to_string(),
            },
        },
        RuntimeJoin {
            alias: "products".to_string(),
            dataset_id: Uuid::new_v4(),
            dataset_version: None,
            on: Expression {
                source: "product_id = products.id".to_string(),
            },
        },
    ];

    let result = apply_runtime_joins(
        working,
        &joins,
        "gl",
        &[
            "journal_id".to_string(),
            "customer_id".to_string(),
            "product_id".to_string(),
            "amount_local".to_string(),
            "currency".to_string(),
            "_period".to_string(),
        ],
        |join| {
            if join.alias == "customers" {
                Ok((
                    sample_datasets::customers_frame().lazy(),
                    vec!["id".to_string(), "tier".to_string(), "_period".to_string()],
                ))
            } else {
                Ok((
                    sample_datasets::products_frame().lazy(),
                    vec![
                        "id".to_string(),
                        "category".to_string(),
                        "_period".to_string(),
                    ],
                ))
            }
        },
    )
    .expect("multi join should succeed")
    .collect()
    .expect("collect multi-join result");

    assert!(result.column("tier_customers").is_ok());
    assert!(result.column("category_products").is_ok());

    let tiers = result
        .column("tier_customers")
        .expect("tier column")
        .str()
        .expect("tier strings");
    assert_eq!(tiers.get(1), Some("gold"));
}

#[test]
fn second_join_cannot_reference_previous_join_alias() {
    let working = sample_datasets::gl_transactions_frame().lazy();

    let joins = vec![
        RuntimeJoin {
            alias: "customers".to_string(),
            dataset_id: Uuid::new_v4(),
            dataset_version: None,
            on: Expression {
                source: "customer_id = customers.id".to_string(),
            },
        },
        RuntimeJoin {
            alias: "products".to_string(),
            dataset_id: Uuid::new_v4(),
            dataset_version: None,
            on: Expression {
                source: "customers.tier = products.category".to_string(),
            },
        },
    ];

    let error = match apply_runtime_joins(
        working,
        &joins,
        "gl",
        &[
            "journal_id".to_string(),
            "customer_id".to_string(),
            "product_id".to_string(),
            "amount_local".to_string(),
            "currency".to_string(),
            "_period".to_string(),
        ],
        |join| {
            if join.alias == "customers" {
                Ok((
                    sample_datasets::customers_frame().lazy(),
                    vec!["id".to_string(), "tier".to_string(), "_period".to_string()],
                ))
            } else {
                Ok((
                    sample_datasets::products_frame().lazy(),
                    vec![
                        "id".to_string(),
                        "category".to_string(),
                        "_period".to_string(),
                    ],
                ))
            }
        },
    ) {
        Ok(_) => panic!("cross-join alias reference should fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        dobo_core::engine::join::JoinError::InvalidJoinCondition(_)
    ));
}

#[test]
fn second_join_cannot_reference_previous_join_suffixed_column() {
    let working = sample_datasets::gl_transactions_frame().lazy();

    let joins = vec![
        RuntimeJoin {
            alias: "customers".to_string(),
            dataset_id: Uuid::new_v4(),
            dataset_version: None,
            on: Expression {
                source: "customer_id = customers.id".to_string(),
            },
        },
        RuntimeJoin {
            alias: "products".to_string(),
            dataset_id: Uuid::new_v4(),
            dataset_version: None,
            on: Expression {
                source: "tier_customers = products.category".to_string(),
            },
        },
    ];

    let error = match apply_runtime_joins(
        working,
        &joins,
        "gl",
        &[
            "journal_id".to_string(),
            "customer_id".to_string(),
            "product_id".to_string(),
            "amount_local".to_string(),
            "currency".to_string(),
            "_period".to_string(),
        ],
        |join| {
            if join.alias == "customers" {
                Ok((
                    sample_datasets::customers_frame().lazy(),
                    vec!["id".to_string(), "tier".to_string(), "_period".to_string()],
                ))
            } else {
                Ok((
                    sample_datasets::products_frame().lazy(),
                    vec![
                        "id".to_string(),
                        "category".to_string(),
                        "_period".to_string(),
                    ],
                ))
            }
        },
    ) {
        Ok(_) => panic!("cross-join suffixed reference should fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        dobo_core::engine::join::JoinError::InvalidJoinCondition(_)
    ));
}
