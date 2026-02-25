use dobo_core::dsl::aggregation::{parse_aggregation, AggregateFunction};

#[test]
fn parse_aggregation_supports_required_functions() {
    let cases = [
        ("SUM(amount)", AggregateFunction::Sum, "amount"),
        ("COUNT(*)", AggregateFunction::Count, "*"),
        ("AVG(amount)", AggregateFunction::Avg, "amount"),
        ("MIN_AGG(amount)", AggregateFunction::MinAgg, "amount"),
        ("MAX_AGG(amount)", AggregateFunction::MaxAgg, "amount"),
    ];

    for (raw, function, input) in cases {
        let parsed = parse_aggregation(raw).expect("aggregation should parse");
        assert_eq!(parsed.function, function);
        assert_eq!(parsed.input_column, input);
    }
}

#[test]
fn parse_aggregation_rejects_invalid_function() {
    let parsed = parse_aggregation("MEDIAN(amount)");
    assert!(parsed.is_err());
}

#[test]
fn parse_aggregation_rejects_trailing_or_leading_garbage() {
    assert!(parse_aggregation("SUM(amount) junk").is_err());
    assert!(parse_aggregation("junk SUM(amount)").is_err());
}

#[test]
fn parse_aggregation_rejects_wildcard_for_non_count_functions() {
    for expression in ["SUM(*)", "AVG(*)", "MIN_AGG(*)", "MAX_AGG(*)"] {
        assert!(parse_aggregation(expression).is_err());
    }
}
