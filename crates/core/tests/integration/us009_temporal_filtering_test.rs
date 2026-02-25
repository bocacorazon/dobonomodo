use dobo_core::engine::error::AppendError;
use dobo_core::engine::temporal::apply_temporal_filter;
use dobo_core::model::TemporalMode;
use polars::df;

#[test]
fn ts16_period_mode_filters_to_run_period() {
    let frame = df!(
        "_period" => &["2025-12", "2026-01", "2026-02"],
        "amount" => &[1i64, 2, 3]
    )
    .expect("frame");

    let filtered = apply_temporal_filter(&frame, Some(TemporalMode::Period), Some("2026-01"), None)
        .expect("period filter");
    assert_eq!(filtered.height(), 1);
}

#[test]
fn ts17_bitemporal_mode_filters_by_as_of_date() {
    let frame = df!(
        "_period_from" => &["2026-01-01", "2026-01-01", "2026-01-20"],
        "_period_to" => &[Some("2026-01-10"), None, None],
        "amount" => &[1i64, 2, 3]
    )
    .expect("frame");

    let filtered = apply_temporal_filter(
        &frame,
        Some(TemporalMode::Bitemporal),
        None,
        Some("2026-01-15"),
    )
    .expect("bitemporal filter");
    assert_eq!(filtered.height(), 1);
}

#[test]
fn ts18_snapshot_mode_appends_all_rows_without_filtering() {
    let frame = df!(
        "amount" => &[1i64, 2, 3]
    )
    .expect("frame");

    let filtered = apply_temporal_filter(
        &frame,
        Some(TemporalMode::Snapshot),
        Some("2026-01"),
        Some("2026-01-15"),
    )
    .expect("snapshot mode");
    assert_eq!(filtered.height(), 3);
}

#[test]
fn ts19_bitemporal_mode_requires_as_of_date() {
    let frame = df!(
        "_period_from" => &["2026-01-01"],
        "_period_to" => &[Option::<&str>::None],
        "amount" => &[1i64]
    )
    .expect("frame");

    let error = apply_temporal_filter(
        &frame,
        Some(TemporalMode::Bitemporal),
        Some("2026-01"),
        None,
    )
    .expect_err("bitemporal mode without as_of_date should fail");
    match error {
        AppendError::ExpressionParseError { error, .. } => {
            assert!(error.contains("as_of_date"));
        }
        other => panic!("expected ExpressionParseError, got {other:?}"),
    }
}
