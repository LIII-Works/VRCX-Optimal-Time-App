use std::time::Duration;

use vrcx_optimal_time_app::validation::{
    ValidationError, parse_friend_ids, parse_user_id, validate_bucket_duration,
    validate_minimum_activations,
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
