#![feature(test)]

extern crate test;

use dobo_core::dsl::parse_expression;
use test::Bencher;

const EXPRESSIONS: [&str; 6] = [
    "transactions.amount_local * fx.rate",
    r#"IF(accounts.type = "revenue", transactions.amount_local * -1, transactions.amount_local)"#,
    r#"transactions.source_system = "ERP" AND transactions.amount_local > 1000"#,
    r#"CONCAT(accounts.code, " - ", accounts.name)"#,
    "SUM(transactions.amount_local)",
    "transactions.posting_date >= TODAY() - 30",
];

#[bench]
fn bench_parse_expression(b: &mut Bencher) {
    let mut idx: usize = 0;
    b.iter(|| {
        let source = EXPRESSIONS[idx % EXPRESSIONS.len()];
        idx = idx.wrapping_add(1);
        parse_expression(source).expect("benchmark expression should parse");
    });
}
