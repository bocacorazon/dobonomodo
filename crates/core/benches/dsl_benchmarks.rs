use criterion::{criterion_group, criterion_main, Criterion};
use dobo_core::dsl::parse_expression;

const EXPRESSIONS: [&str; 6] = [
    "transactions.amount_local * fx.rate",
    r#"IF(accounts.type = "revenue", transactions.amount_local * -1, transactions.amount_local)"#,
    r#"transactions.source_system = "ERP" AND transactions.amount_local > 1000"#,
    r#"CONCAT(accounts.code, " - ", accounts.name)"#,
    "SUM(transactions.amount_local)",
    "transactions.posting_date >= TODAY() - 30",
];

fn bench_parse_expression(c: &mut Criterion) {
    let mut idx: usize = 0;
    c.bench_function("parse_expression", |b| {
        b.iter(|| {
            let source = EXPRESSIONS[idx % EXPRESSIONS.len()];
            idx = idx.wrapping_add(1);
            parse_expression(source).expect("benchmark expression should parse");
        });
    });
}

criterion_group!(benches, bench_parse_expression);
criterion_main!(benches);
