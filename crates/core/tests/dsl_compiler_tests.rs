//! Unit and contract tests for DSL compiler.

use dobo_core::dsl::*;
use polars::df;
use polars::prelude::DataType;
use polars::prelude::{AnyValue, IntoLazy};

fn build_context(allow_aggregates: bool) -> CompilationContext {
    let mut ctx = CompilationContext::new()
        .with_aggregates(allow_aggregates)
        .with_today(chrono::NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid fixed today date"));
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_column("transactions.count", ColumnType::Integer);
    ctx.add_column("transactions.flag", ColumnType::Boolean);
    ctx.add_column("transactions.name", ColumnType::String);
    ctx.add_column("transactions.date", ColumnType::Date);
    ctx.add_selector("ACTIVE", "transactions.flag = TRUE");
    ctx
}

fn compile_source(source: &str, ctx: &CompilationContext) -> CompiledExpression {
    let ast = parse_expression(source).expect("parse should succeed");
    compile_expression(&ast, ctx).expect("compile should succeed")
}

#[test]
fn test_compile_literals() {
    let ctx = build_context(false);
    for source in [r#"42"#, r#""hello""#, "TRUE", "NULL"] {
        let compiled = compile_source(source, &ctx);
        let debug = format!("{:?}", compiled.as_expr());
        assert!(!debug.is_empty());
    }
}

#[test]
fn test_compile_column_reference() {
    let ctx = build_context(false);
    let compiled = compile_source("transactions.amount", &ctx);
    let debug = format!("{:?}", compiled.as_expr());
    assert!(debug.contains("transactions.amount"));
}

#[test]
fn test_compile_arithmetic_operators() {
    let ctx = build_context(false);
    for source in [
        "transactions.amount + 1",
        "transactions.amount - 1",
        "transactions.amount * 2",
        "transactions.amount / 2",
    ] {
        let compiled = compile_source(source, &ctx);
        assert!(!format!("{:?}", compiled.as_expr()).is_empty());
    }
}

#[test]
fn test_compile_comparison_operators() {
    let ctx = build_context(false);
    for source in [
        "transactions.amount = 1",
        "transactions.amount <> 1",
        "transactions.amount < 1",
        "transactions.amount <= 1",
        "transactions.amount > 1",
        "transactions.amount >= 1",
    ] {
        let compiled = compile_source(source, &ctx);
        assert_eq!(compiled.return_type(), ExprType::Boolean);
    }
}

#[test]
fn test_compile_logical_operators() {
    let ctx = build_context(false);
    for source in ["TRUE AND FALSE", "TRUE OR FALSE", "NOT TRUE"] {
        let compiled = compile_source(source, &ctx);
        assert_eq!(compiled.return_type(), ExprType::Boolean);
    }
}

#[test]
fn test_compile_arithmetic_functions() {
    let ctx = build_context(false);
    for source in [
        "ABS(-5)",
        "ROUND(transactions.amount, 2)",
        "FLOOR(transactions.amount)",
        "CEIL(transactions.amount)",
        "MOD(transactions.amount, 2)",
        "MIN(transactions.amount, 10)",
        "MAX(transactions.amount, 10)",
    ] {
        let compiled = compile_source(source, &ctx);
        assert_eq!(compiled.return_type(), ExprType::Number);
    }
}

#[test]
fn test_compile_string_functions() {
    let ctx = build_context(false);
    for source in [
        r#"CONCAT(transactions.name, "x")"#,
        "UPPER(transactions.name)",
        "LOWER(transactions.name)",
        "TRIM(transactions.name)",
        "LEFT(transactions.name, 2)",
        "RIGHT(transactions.name, 2)",
        "LEN(transactions.name)",
        r#"CONTAINS(transactions.name, "a")"#,
        r#"REPLACE(transactions.name, "a", "b")"#,
    ] {
        let compiled = compile_source(source, &ctx);
        assert!(!format!("{:?}", compiled.as_expr()).is_empty());
    }
}

#[test]
fn test_compile_conditional_functions() {
    let ctx = build_context(false);
    for source in [
        "IF(transactions.amount > 10, 1, 0)",
        "AND(transactions.flag, TRUE)",
        "OR(transactions.flag, FALSE)",
        "NOT(transactions.flag)",
        "ISNULL(NULL)",
        "COALESCE(NULL, transactions.amount, 0)",
    ] {
        let compiled = compile_source(source, &ctx);
        assert!(!format!("{:?}", compiled.as_expr()).is_empty());
    }
}

#[test]
fn test_compile_date_functions() {
    let mut ctx = build_context(false);
    ctx.add_selector("TODAY_REF", "TODAY()");
    for source in [
        r#"DATE("2026-01-01")"#,
        "TODAY()",
        "YEAR(transactions.date)",
        "MONTH(transactions.date)",
        "DAY(transactions.date)",
        "DATEDIFF(transactions.date, transactions.date)",
        "DATEADD(transactions.date, 1)",
    ] {
        let compiled = compile_source(source, &ctx);
        assert!(!format!("{:?}", compiled.as_expr()).is_empty());
    }
}

#[test]
fn test_compile_aggregate_functions() {
    let ctx = build_context(true);
    for source in [
        "SUM(transactions.amount)",
        "COUNT(transactions.amount)",
        "COUNT_ALL()",
        "AVG(transactions.amount)",
        "MIN_AGG(transactions.amount)",
        "MAX_AGG(transactions.amount)",
    ] {
        let compiled = compile_source(source, &ctx);
        assert_eq!(compiled.return_type(), ExprType::Number);
    }
}

#[test]
fn test_contract_polars_compatibility() {
    let ctx = build_context(false);
    let compiled = compile_source("transactions.amount * 1.1", &ctx);
    let df = df! {
        "transactions.amount" => [10.0, 20.0, 30.0],
        "transactions.count" => [1, 2, 3],
        "transactions.flag" => [true, false, true],
        "transactions.name" => ["a", "b", "c"],
        "transactions.date" => ["2026-01-01", "2026-01-02", "2026-01-03"],
    }
    .expect("dataframe should build")
    .lazy();

    let selected = df.select([compiled.into_expr()]).collect();
    assert!(selected.is_ok());
}

#[test]
fn test_compile_with_interpolation_end_to_end() {
    let mut ctx = build_context(false);
    ctx.add_selector("HIGH_AMOUNT", "transactions.amount > 10");
    let compiled = compile_with_interpolation("{HIGH_AMOUNT} AND transactions.flag = TRUE", &ctx)
        .expect("full compile should succeed");
    assert_eq!(compiled.return_type(), ExprType::Boolean);
}

#[test]
fn test_compile_null_literal_produces_null_value() {
    let ctx = build_context(false);
    let compiled = compile_source("NULL", &ctx);
    let out = df! {"seed" => [1i64]}
        .expect("dataframe should build")
        .lazy()
        .select([compiled.into_expr().alias("value")])
        .collect()
        .expect("query should execute");

    let value = out
        .column("value")
        .expect("value column should exist")
        .get(0)
        .expect("row should exist");
    assert!(matches!(value, AnyValue::Null));
}

#[test]
fn test_compile_divide_by_zero_produces_null() {
    let ctx = build_context(false);
    let compiled = compile_source("transactions.amount / transactions.count", &ctx);
    let out = df! {
        "transactions.amount" => [10.0, 10.0],
        "transactions.count" => [0i64, 2i64],
        "transactions.flag" => [true, false],
        "transactions.name" => ["a", "b"],
        "transactions.date" => ["2026-01-01", "2026-01-02"],
    }
    .expect("dataframe should build")
    .lazy()
    .select([compiled.into_expr().alias("value")])
    .collect()
    .expect("query should execute");

    let first = out
        .column("value")
        .expect("value column should exist")
        .get(0)
        .expect("first row should exist");
    assert!(matches!(first, AnyValue::Null));

    let second = out
        .column("value")
        .expect("value column should exist")
        .get(1)
        .expect("second row should exist");
    assert_eq!(second, AnyValue::Float64(5.0));
}

#[test]
fn test_compile_functions_have_behavior() {
    let mut ctx = build_context(false)
        .with_today(chrono::NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid fixed today date"));
    ctx.add_column("s.value", ColumnType::String);
    ctx.add_column("s.number", ColumnType::Float);

    let input = df! {
        "s.value" => ["abc"],
        "s.number" => [12.6f64],
    }
    .expect("dataframe should build")
    .lazy();

    let year = compile_source(r#"YEAR(DATE("2026-01-31"))"#, &ctx).into_expr();
    let month = compile_source(r#"MONTH(DATE("2026-01-31"))"#, &ctx).into_expr();
    let day = compile_source(r#"DAY(DATE("2026-01-31"))"#, &ctx).into_expr();
    let datediff =
        compile_source(r#"DATEDIFF(DATE("2026-01-31"), DATE("2026-01-01"))"#, &ctx).into_expr();
    let contains = compile_source(r#"CONTAINS(s.value, "bc")"#, &ctx).into_expr();
    let len = compile_source("LEN(s.value)", &ctx).into_expr();
    let round = compile_source("ROUND(s.number, 0)", &ctx).into_expr();
    let floor = compile_source("FLOOR(s.number)", &ctx).into_expr();
    let ceil = compile_source("CEIL(s.number)", &ctx).into_expr();
    let today = compile_source("TODAY()", &ctx).into_expr();
    let concat = compile_source(r#"CONCAT(s.value, "-x")"#, &ctx).into_expr();

    let out = input
        .select([
            year.alias("year"),
            month.alias("month"),
            day.alias("day"),
            datediff.alias("datediff"),
            contains.alias("contains"),
            len.alias("len"),
            round.alias("round"),
            floor.alias("floor"),
            ceil.alias("ceil"),
            today.alias("today"),
            concat.alias("concat"),
        ])
        .collect()
        .expect("query should execute");

    assert_eq!(
        out.column("year").expect("year").get(0).expect("value"),
        AnyValue::Int32(2026)
    );
    assert_eq!(
        out.column("month").expect("month").get(0).expect("value"),
        AnyValue::Int8(1)
    );
    assert_eq!(
        out.column("day").expect("day").get(0).expect("value"),
        AnyValue::Int8(31)
    );
    assert_eq!(
        out.column("datediff")
            .expect("datediff")
            .get(0)
            .expect("value"),
        AnyValue::Int64(30)
    );
    assert_eq!(
        out.column("contains")
            .expect("contains")
            .get(0)
            .expect("value"),
        AnyValue::Boolean(true)
    );
    assert_eq!(
        out.column("len").expect("len").get(0).expect("value"),
        AnyValue::UInt32(3)
    );
    assert_eq!(
        out.column("round").expect("round").get(0).expect("value"),
        AnyValue::Float64(13.0)
    );
    assert_eq!(
        out.column("floor").expect("floor").get(0).expect("value"),
        AnyValue::Float64(12.0)
    );
    assert_eq!(
        out.column("ceil").expect("ceil").get(0).expect("value"),
        AnyValue::Float64(13.0)
    );
    let today_string = out
        .column("today")
        .expect("today")
        .cast(&DataType::String)
        .expect("cast to string should succeed")
        .get(0)
        .expect("value")
        .to_string();
    assert!(today_string.contains("2026-01-15"));
    assert_eq!(
        out.column("concat").expect("concat").get(0).expect("value"),
        AnyValue::String("abc-x")
    );
}

#[test]
fn test_trim_removes_tabs_and_newlines() {
    let mut ctx = build_context(false);
    ctx.add_column("s.value", ColumnType::String);

    let out = df! {
        "s.value" => ["\t abc \n"],
    }
    .expect("dataframe should build")
    .lazy()
    .select([compile_source("TRIM(s.value)", &ctx)
        .into_expr()
        .alias("trimmed")])
    .collect()
    .expect("query should execute");

    assert_eq!(
        out.column("trimmed")
            .expect("trimmed")
            .get(0)
            .expect("value"),
        AnyValue::String("abc")
    );
}
