use std::time::Duration;

use chrono::{Datelike, Local, TimeZone, Timelike};

use vrcx_optimal_time_app::validation::{
    DateTimeParts, ValidationError, local_datetime_from_parts, parse_friend_ids, parse_user_id,
    validate_bucket_duration, validate_minimum_activations, validate_time_range,
};

const UPPER_ID: &str = "usr_550E8400-E29B-41D4-A716-446655440000";
const LOWER_ID: &str = "usr_550e8400-e29b-41d4-a716-446655440000";

#[test]
fn user_ids_require_the_usr_prefix_and_uuid_shape_and_normalize_to_lowercase() {
    assert_eq!(parse_user_id(UPPER_ID, 1).unwrap(), LOWER_ID);

    for invalid in [
        "",
        "550e8400-e29b-41d4-a716-446655440000",
        "usr_not-a-uuid",
        "usr_550e8400-e29b-41d4-a716-446655440000\n",
    ] {
        assert!(matches!(
            parse_user_id(invalid, 1),
            Err(ValidationError::InvalidUserId { line: 1, .. })
        ));
    }
}

#[test]
fn friend_paste_preserves_first_seen_valid_ids_and_reports_one_based_invalid_lines() {
    let pasted =
        format!("{UPPER_ID}\n{LOWER_ID}\nusr_bad\n\nusr_123e4567-e89b-12d3-a456-426614174000");

    let report = parse_friend_ids(&pasted);
    assert_eq!(report.invalid_lines, vec![3, 4]);
    assert_eq!(
        report.ids,
        vec![
            LOWER_ID.to_owned(),
            "usr_123e4567-e89b-12d3-a456-426614174000".to_owned(),
        ]
    );

    let valid = format!("{UPPER_ID}\n{LOWER_ID}\nusr_123e4567-e89b-12d3-a456-426614174000");
    assert_eq!(
        parse_friend_ids(&valid).ids,
        vec![
            LOWER_ID.to_owned(),
            "usr_123e4567-e89b-12d3-a456-426614174000".to_owned(),
        ]
    );
}

#[test]
fn empty_friend_filter_means_no_filter_not_an_invalid_id() {
    let report = parse_friend_ids("");

    assert!(report.ids.is_empty());
    assert!(report.invalid_lines.is_empty());
}

#[test]
fn bucket_duration_must_evenly_divide_a_day() {
    assert!(validate_bucket_duration(Duration::from_secs(600)).is_ok());
    assert!(matches!(
        validate_bucket_duration(Duration::from_secs(601)),
        Err(ValidationError::InvalidBucketDuration { .. })
    ));
}

#[test]
fn minimum_activations_must_be_positive() {
    assert!(validate_minimum_activations(1).is_ok());
    assert_eq!(
        validate_minimum_activations(0),
        Err(ValidationError::InvalidMinimumActivations)
    );
}

#[test]
fn numeric_parts_build_a_local_datetime_without_text_parsing() {
    let value = local_datetime_from_parts(DateTimeParts {
        year: 2024,
        month: 2,
        day: 29,
        hour: 23,
        minute: 58,
    })
    .unwrap();

    assert_eq!(value.year(), 2024);
    assert_eq!(value.month(), 2);
    assert_eq!(value.day(), 29);
    assert_eq!(value.hour(), 23);
    assert_eq!(value.minute(), 58);
}

#[test]
fn impossible_calendar_dates_are_rejected() {
    for parts in [
        DateTimeParts {
            year: 2023,
            month: 2,
            day: 29,
            hour: 12,
            minute: 0,
        },
        DateTimeParts {
            year: 2024,
            month: 4,
            day: 31,
            hour: 12,
            minute: 0,
        },
        DateTimeParts {
            year: 2024,
            month: 1,
            day: 1,
            hour: 24,
            minute: 0,
        },
        DateTimeParts {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 60,
        },
    ] {
        assert!(matches!(
            local_datetime_from_parts(parts),
            Err(ValidationError::InvalidDateTime)
        ));
    }
}

#[test]
fn time_range_requires_start_not_later_than_end() {
    let start = Local.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).single();
    let end = Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single();

    assert_eq!(
        validate_time_range(start, end),
        Err(ValidationError::InvalidTimeRange)
    );
    assert!(validate_time_range(start, None).is_ok());
    assert!(validate_time_range(None, end).is_ok());
}
