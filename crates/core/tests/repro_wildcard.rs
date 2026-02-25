use dobo_core::dsl::aggregation::{parse_aggregation, AggregateFunction};

#[test]
fn test_parse_count_wildcard() {
    let parsed = parse_aggregation("COUNT(*)").unwrap();
    assert_eq!(parsed.input_column, "*");
    assert_eq!(parsed.function, AggregateFunction::Count);
}
