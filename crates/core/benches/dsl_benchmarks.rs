use dobo_core::dsl::parse_expression;
use std::time::Instant;

fn main() {
    let expressions = [
        "transactions.amount_local * fx.rate",
        r#"IF(accounts.type = "revenue", transactions.amount_local * -1, transactions.amount_local)"#,
        r#"transactions.source_system = "ERP" AND transactions.amount_local > 1000"#,
        r#"CONCAT(accounts.code, " - ", accounts.name)"#,
        "SUM(transactions.amount_local)",
        "transactions.posting_date >= TODAY() - 30",
    ];

    let start = Instant::now();
    for idx in 0..1000 {
        let source = expressions[idx % expressions.len()];
        parse_expression(source).expect("benchmark expression should parse");
    }
    let elapsed = start.elapsed();
    println!(
        "Parsed 1000 expressions in {} ms",
        elapsed.as_secs_f64() * 1000.0
    );
}
