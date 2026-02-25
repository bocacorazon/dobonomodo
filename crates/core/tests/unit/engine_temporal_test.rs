use dobo_core::engine::temporal::apply_temporal_filter;
use dobo_core::model::TemporalMode;
use polars::df;

#[test]
fn apply_temporal_filter_period_mode() {
    let frame = df!(
        "_period" => &["2026-01", "2026-02"],
        "amount" => &[1i64, 2]
    )
    .expect("frame");

    let result = apply_temporal_filter(&frame, Some(TemporalMode::Period), Some("2026-01"), None)
        .expect("period filter should succeed");
    assert_eq!(result.height(), 1);
}

#[test]
fn apply_temporal_filter_bitemporal_mode() {
    let frame = df!(
        "_period_from" => &["2026-01-01", "2026-01-20"],
        "_period_to" => &[Some("2026-01-10"), None],
        "amount" => &[1i64, 2]
    )
    .expect("frame");

    let result = apply_temporal_filter(
        &frame,
        Some(TemporalMode::Bitemporal),
        None,
        Some("2026-01-15"),
    )
    .expect("bitemporal filter should succeed");
    assert_eq!(result.height(), 0);
}

#[test]
fn apply_temporal_filter_bitemporal_mode_supports_legacy_columns() {
    let frame = df!(
        "valid_from" => &["2026-01-01", "2026-01-20"],
        "valid_to" => &[Some("2026-01-10"), None],
        "amount" => &[1i64, 2]
    )
    .expect("frame");

    let result = apply_temporal_filter(
        &frame,
        Some(TemporalMode::Bitemporal),
        None,
        Some("2026-01-15"),
    )
    .expect("legacy bitemporal filter should succeed");
    assert_eq!(result.height(), 0);
}

#[test]
fn apply_temporal_filter_snapshot_mode() {
    let frame = df!(
        "amount" => &[1i64, 2, 3]
    )
    .expect("frame");

    let result = apply_temporal_filter(
        &frame,
        Some(TemporalMode::Snapshot),
        Some("2026-01"),
        Some("2026-01-15"),
    )
    .expect("snapshot filter should succeed");
    assert_eq!(result.height(), 3);
}
