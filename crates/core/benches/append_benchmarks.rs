use dobo_core::engine::append::{execute_append, AppendExecutionContext};
use dobo_core::model::{AppendOperation, DatasetRef};
use polars::df;
use uuid::Uuid;
use criterion::{criterion_group, criterion_main, Criterion};

fn append_benchmark(c: &mut Criterion) {
    c.bench_function("append_10k_rows", |b| {
        b.iter(|| {
            let working = df!(
                "_row_id" => &["w1"],
                "_source_dataset" => &["working"],
                "_operation_seq" => &[1i64],
                "_deleted" => &[false],
                "account_code" => &["4000"],
                "amount" => &[100i64]
            )
            .expect("working frame");

            let source = df!(
                "account_code" => vec!["4000"; 10_000],
                "amount" => vec![1i64; 10_000]
            )
            .expect("source frame");

            let operation = AppendOperation {
                source: DatasetRef {
                    dataset_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")
                        .expect("uuid"),
                    dataset_version: None,
                },
                source_selector: None,
                aggregation: None,
            };

            let _ = execute_append(
                &working,
                &source,
                &operation,
                &AppendExecutionContext {
                    operation_seq: 1,
                    ..Default::default()
                },
            );
        });
    });
}

criterion_group!(benches, append_benchmark);
criterion_main!(benches);
