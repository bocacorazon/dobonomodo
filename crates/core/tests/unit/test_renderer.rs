use dobo_core::resolver::renderer::render_template;
use std::collections::HashMap;

#[test]
fn test_renderer_preserves_preencoded_url_value() {
    let mut context = HashMap::new();
    context.insert("period_id".to_string(), "2024%2FQ1".to_string());

    let rendered = render_template("/data/{period_id}/facts.parquet", &context).unwrap();
    assert_eq!(rendered, "/data/2024%2FQ1/facts.parquet");
}

#[test]
fn test_renderer_rejects_empty_token() {
    let context = HashMap::new();
    let error = render_template("/data/{}/facts.parquet", &context).unwrap_err();
    assert!(error.contains("{}"));
}

#[test]
fn test_renderer_special_characters() {
    let mut context = HashMap::new();
    context.insert("table_name".to_string(), "sales-data_v2".to_string());

    let rendered = render_template("/data/{table_name}.parquet", &context).unwrap();
    assert_eq!(rendered, "/data/sales-data_v2.parquet");
}
