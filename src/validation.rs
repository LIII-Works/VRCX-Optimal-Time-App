use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, Local, LocalResult, NaiveDate, TimeZone};
use thiserror::Error;

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid VRChat user ID on line {line}; expected usr_ followed by a UUID")]
    InvalidUserId { line: usize },
    #[error("bucket duration of {seconds} seconds must divide 86400 seconds evenly")]
    InvalidBucketDuration { seconds: u64 },
    #[error("minimum activations must be at least 1")]
    InvalidMinimumActivations,
    #[error("invalid local date or time")]
    InvalidDateTime,
    #[error("analysis start time must not be later than end time")]
    InvalidTimeRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeParts {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendIdParseReport {
    pub ids: Vec<String>,
    pub invalid_lines: Vec<usize>,
}

pub fn parse_user_id(input: &str, line: usize) -> Result<String, ValidationError> {
    if !is_user_id(input) {
        return Err(ValidationError::InvalidUserId { line });
    }

    Ok(input.to_ascii_lowercase())
}

pub fn parse_friend_ids(input: &str) -> FriendIdParseReport {
    if input.is_empty() {
        return FriendIdParseReport {
            ids: Vec::new(),
            invalid_lines: Vec::new(),
        };
    }

    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    let mut invalid_lines = Vec::new();

    for (index, raw_line) in input.split('\n').enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        match parse_user_id(line, index + 1) {
            Ok(id) if seen.insert(id.clone()) => ids.push(id),
            Ok(_) => {}
            Err(_) => invalid_lines.push(index + 1),
        }
    }

    FriendIdParseReport { ids, invalid_lines }
}

pub fn validate_bucket_duration(duration: Duration) -> Result<(), ValidationError> {
    let seconds = duration.as_secs();
    if seconds == 0 || duration.subsec_nanos() != 0 || !SECONDS_PER_DAY.is_multiple_of(seconds) {
        return Err(ValidationError::InvalidBucketDuration { seconds });
    }

    Ok(())
}

pub fn validate_minimum_activations(value: u32) -> Result<(), ValidationError> {
    (value > 0)
        .then_some(())
        .ok_or(ValidationError::InvalidMinimumActivations)
}

pub fn local_datetime_from_parts(parts: DateTimeParts) -> Result<DateTime<Local>, ValidationError> {
    NaiveDate::from_ymd_opt(parts.year, parts.month, parts.day)
        .and_then(|date| date.and_hms_opt(parts.hour, parts.minute, 0))
        .and_then(|naive| match Local.from_local_datetime(&naive) {
            LocalResult::Single(value) => Some(value),
            LocalResult::None | LocalResult::Ambiguous(_, _) => None,
        })
        .ok_or(ValidationError::InvalidDateTime)
}

pub fn validate_time_range(
    start: Option<DateTime<Local>>,
    end: Option<DateTime<Local>>,
) -> Result<(), ValidationError> {
    if let (Some(start), Some(end)) = (start, end)
        && start > end
    {
        return Err(ValidationError::InvalidTimeRange);
    }

    Ok(())
}

fn is_user_id(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() == 40
        && bytes.starts_with(b"usr_")
        && bytes[4..].iter().enumerate().all(|(index, byte)| {
            let index = index + 4;
            if [12, 17, 22, 27].contains(&index) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
