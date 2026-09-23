#![allow(clippy::unwrap_used)]

use crate::mock_time;
use crate::model::Model;
use crate::test::util::new_empty_model;

// 14:44 20 Mar 2023 Berlin
const TIMESTAMP_2023: i64 = 1679319865208;

#[test]
fn today_basic() {
    let mut model = new_empty_model();
    model._set("A1", "=TODAY()");
    model._set("A2", "=TEXT(A1, \"yyyy/m/d\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"11/8/2022");
    assert_eq!(model._get_text("A2"), *"2022/11/8");
}

#[test]
fn today_with_wrong_tz() {
    let model = Model::new_empty("model", "en", "Wrong Timezone", "en");
    assert!(model.is_err());
}

#[test]
fn now_basic_utc() {
    mock_time::set_mock_time(TIMESTAMP_2023);
    let mut model = Model::new_empty("model", "en", "UTC", "en").unwrap();
    model._set("A1", "=TODAY()");
    model._set("A2", "=NOW()");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3/20/2023");
    // 45005.572511574
    assert_eq!(model._get_text("A2"), *"3/20/2023, 1:44 PM");
}

#[test]
fn now_basic_europe_berlin() {
    mock_time::set_mock_time(TIMESTAMP_2023);
    let mut model = Model::new_empty("model", "en", "Europe/Berlin", "en").unwrap();
    model._set("A1", "=TODAY()");
    model._set("A2", "=NOW()");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3/20/2023");
    // This is UTC + 1 hour: 45005.572511574 + 1/24
    assert_eq!(model._get_text("A2"), *"3/20/2023, 2:44 PM");
}

#[test]
fn today_with_timezone() {
    mock_time::set_mock_time(TIMESTAMP_2023);
    let mut model = Model::new_empty("model", "en", "UTC", "en").unwrap();
    model._set("A1", "=TODAY(\"Pacific/Kiritimati\")");
    model._set("A2", "=NOW(\"Europe/Berlin\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3/21/2023");
    // 45005.572511574
    assert_eq!(model._get_text("A2"), *"3/20/2023, 2:44 PM");
    model.evaluate();
}

#[test]
fn today_with_invalid_timezone() {
    mock_time::set_mock_time(TIMESTAMP_2023);
    let mut model = Model::new_empty("model", "en", "UTC", "en").unwrap();
    model._set("A1", "=TODAY(\"Invalid/Timezone\")");
    model._set("A2", "=NOW(\"Invalid/Timezone\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#VALUE!");
    assert_eq!(model._get_text("A2"), *"#VALUE!");
}

#[test]
fn captured_host_clock_is_local_validated_and_consistent_with_named_timezones() {
    let mut model = Model::new_empty("clock", "en", "UTC", "en").unwrap();
    for (instant, offset, local, utc) in [
        (
            "2025-01-01T07:59:59Z",
            -28800,
            "2024-12-31 23:59:59",
            "2025-01-01 07:59:59",
        ),
        (
            "1900-02-28T12:00:00Z",
            0,
            "1900-02-28 12:00:00",
            "1900-02-28 12:00:00",
        ),
    ] {
        let millis = chrono::DateTime::parse_from_rfc3339(instant)
            .unwrap()
            .timestamp_millis();
        model.set_calculation_clock(millis, offset).unwrap();
        model._set("A1", "=TEXT(NOW(),\"yyyy-mm-dd hh:mm:ss\")");
        model._set("A2", "=TEXT(NOW(\"UTC\"),\"yyyy-mm-dd hh:mm:ss\")");
        model._set("A3", "=NOW()-NOW()");
        model.evaluate();
        assert_eq!(model._get_text("A1"), local);
        assert_eq!(model._get_text("A2"), utc);
        assert_eq!(
            model.get_cell_value_by_ref("Sheet1!A3").unwrap(),
            crate::cell::CellValue::Number(0.0)
        );
        assert!(model.set_calculation_clock(i64::MAX, 0).is_err());
        assert!(model.set_calculation_clock(millis, 86400).is_err());
        model.evaluate();
        assert_eq!(
            model._get_text("A1"),
            local,
            "Invalid input must retain the last valid clock"
        );
    }
}
