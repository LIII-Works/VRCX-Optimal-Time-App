// Copyright 2022-2024 Michael Ripley
// Derived behavior from vrcx-optimal-time at d28cb95e9d14b630d83e682752959162ee3c86e7.
// Licensed under the MIT license; see upstream_license.txt and THIRD_PARTY_NOTICES.txt.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, Timelike, Utc};
use rusqlite::{Connection, ErrorCode, OpenFlags};
use thiserror::Error;

use crate::{
    model::{AnalysisRequest, MissingDataBehavior, WeeklyGraph},
    validation::{
        ValidationError, parse_user_id, validate_bucket_duration, validate_minimum_activations,
        validate_time_range,
    },
};

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const DAYS_PER_WEEK: usize = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    pub graph: WeeklyGraph,
    pub friend_graphs: Vec<FriendGraph>,
    pub all_event_count: usize,
    pub online_offline_event_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FriendGraph {
    pub friend_id: String,
    pub graph: WeeklyGraph,
}

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("VRCX database was not found at {path}")]
    DatabaseNotFound { path: PathBuf },
    #[error("VRCX database is locked at {path}")]
    DatabaseLocked { path: PathBuf },
    #[error("VRCX database at {path} does not match the expected VRCX schema")]
    SchemaMismatch { path: PathBuf },
    #[error("could not open VRCX database {path}: {source}")]
    OpenDatabase {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("could not configure VRCX database {path}: {source}")]
    ConfigureDatabase {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("could not query VRCX database {path}: {source}")]
    QueryDatabase {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("could not read VRCX database {path}: {source}")]
    DatabaseReadFailed {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("VRCX database {path} contains an invalid timestamp: {value}")]
    InvalidTimestamp { path: PathBuf, value: String },
    #[error("VRCX database {path} contains an unsupported online/offline event type: {value}")]
    InvalidEventType { path: PathBuf, value: String },
    #[error("VRCX database {path} has no usable activity history")]
    NoUsableActivity { path: PathBuf },
}

#[derive(Debug, Clone, Copy)]
struct Interval {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Default)]
struct BucketValue {
    online_count: u32,
    activity_dates: HashSet<chrono::NaiveDate>,
}

pub fn analyze(request: &AnalysisRequest) -> Result<AnalysisResult, AnalyzerError> {
    validate_bucket_duration(request.bucket_duration)?;
    validate_minimum_activations(request.minimum_activations)?;
    validate_time_range(request.start_time, request.end_time)?;
    let normalized_user_id = parse_user_id(&request.your_user_id, 1)?;
    let table_prefix = normalized_user_id.replace(['-', '_'], "");
    let path = request.database_path.clone();
    if !path.is_file() {
        return Err(AnalyzerError::DatabaseNotFound { path });
    }
    let connection = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|source| map_open_error(&path, source))?;
    connection
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
        .map_err(|source| AnalyzerError::ConfigureDatabase {
            path: path.to_path_buf(),
            source,
        })?;

    let all_events = read_all_events(&connection, &path, &table_prefix)?;
    let running_intervals = reconstruct_running_intervals(&all_events, request.uptime_threshold)
        .into_iter()
        .filter_map(|interval| {
            clip_to_range(
                interval,
                request.start_time.as_ref(),
                request.end_time.as_ref(),
            )
        })
        .collect::<Vec<_>>();
    if running_intervals.is_empty() {
        return Err(AnalyzerError::NoUsableActivity { path });
    }

    let buckets_per_day = (24 * 60 * 60 / request.bucket_duration.as_secs()) as usize;
    let mut buckets: [Vec<BucketValue>; DAYS_PER_WEEK] = std::array::from_fn(|_| {
        (0..buckets_per_day)
            .map(|_| BucketValue::default())
            .collect()
    });
    for interval in &running_intervals {
        register_activity(interval, request.bucket_duration, &mut buckets);
    }

    let mut friend_buckets: Vec<(String, [Vec<BucketValue>; DAYS_PER_WEEK])> = request
        .friend_ids
        .iter()
        .cloned()
        .map(|friend_id| {
            let buckets = std::array::from_fn(|_| {
                (0..buckets_per_day)
                    .map(|_| BucketValue::default())
                    .collect()
            });
            (friend_id, buckets)
        })
        .collect();
    for interval in &running_intervals {
        for (_, friend_buckets) in &mut friend_buckets {
            register_activity(interval, request.bucket_duration, friend_buckets);
        }
    }

    let online_events = read_online_offline_events(&connection, &path, &table_prefix)?;
    let mut online_since = HashMap::<String, DateTime<Utc>>::new();
    for (created_at, user_id, event_type) in &online_events {
        if !request.friend_ids.is_empty() && !request.friend_ids.iter().any(|id| id == user_id) {
            continue;
        }
        match event_type.as_str() {
            "Online" => {
                online_since.insert(user_id.clone(), *created_at);
            }
            "Offline" => {
                if let Some(start) = online_since.remove(user_id) {
                    let Some(interval) = clip_to_range(
                        Interval {
                            start,
                            end: *created_at,
                        },
                        request.start_time.as_ref(),
                        request.end_time.as_ref(),
                    ) else {
                        continue;
                    };
                    for interval in
                        clamp_to_running(interval.start, interval.end, &running_intervals)
                    {
                        register_online(&interval, request.bucket_duration, &mut buckets);
                        if let Some((_, friend_buckets)) =
                            friend_buckets.iter_mut().find(|(id, _)| id == user_id)
                        {
                            register_online(&interval, request.bucket_duration, friend_buckets);
                        }
                    }
                }
            }
            value => {
                return Err(AnalyzerError::InvalidEventType {
                    path: path.to_path_buf(),
                    value: value.to_owned(),
                });
            }
        }
    }

    let graph = build_graph(buckets, request);
    let friend_graphs = friend_buckets
        .into_iter()
        .map(|(friend_id, buckets)| FriendGraph {
            friend_id,
            graph: build_graph(buckets, request),
        })
        .collect();

    Ok(AnalysisResult {
        graph,
        friend_graphs,
        all_event_count: all_events.len(),
        online_offline_event_count: online_events.len(),
    })
}

fn build_graph(
    buckets: [Vec<BucketValue>; DAYS_PER_WEEK],
    request: &AnalysisRequest,
) -> WeeklyGraph {
    WeeklyGraph {
        bucket_duration: request.bucket_duration,
        weekdays: buckets.map(|day| {
            day.into_iter()
                .map(|bucket| {
                    if bucket.online_count < request.minimum_activations
                        || (request.normalize && bucket.activity_dates.is_empty())
                    {
                        match request.missing_data {
                            MissingDataBehavior::Gap => None,
                            MissingDataBehavior::Zero => Some(0.0),
                        }
                    } else if request.normalize {
                        Some(bucket.online_count as f64 / bucket.activity_dates.len() as f64)
                    } else {
                        Some(bucket.online_count as f64)
                    }
                })
                .collect()
        }),
    }
}

fn read_all_events(
    connection: &Connection,
    path: &Path,
    prefix: &str,
) -> Result<Vec<DateTime<Utc>>, AnalyzerError> {
    let statement = format!(
        "select created_at from {prefix}_feed_avatar union select created_at from {prefix}_feed_gps union select created_at from {prefix}_feed_online_offline union select created_at from {prefix}_feed_status union select created_at from {prefix}_friend_log_history order by created_at asc"
    );
    let mut query = connection
        .prepare(&statement)
        .map_err(|source| map_query_error(path, source))?;
    let values = query
        .query_map((), |row| row.get::<_, String>(0))
        .map_err(|source| map_query_error(path, source))?;
    values
        .map(|value| {
            let value = value.map_err(|source| AnalyzerError::QueryDatabase {
                path: path.to_path_buf(),
                source,
            })?;
            value
                .parse::<DateTime<Utc>>()
                .map_err(|_| AnalyzerError::InvalidTimestamp {
                    path: path.to_path_buf(),
                    value,
                })
        })
        .collect()
}

fn read_online_offline_events(
    connection: &Connection,
    path: &Path,
    prefix: &str,
) -> Result<Vec<(DateTime<Utc>, String, String)>, AnalyzerError> {
    let statement = format!(
        "select created_at, user_id, type from {prefix}_feed_online_offline order by created_at asc"
    );
    let mut query = connection
        .prepare(&statement)
        .map_err(|source| map_query_error(path, source))?;
    let values = query
        .query_map((), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|source| map_query_error(path, source))?;
    values
        .map(|value| {
            let (timestamp, user_id, event_type) =
                value.map_err(|source| AnalyzerError::QueryDatabase {
                    path: path.to_path_buf(),
                    source,
                })?;
            let created_at = timestamp.parse::<DateTime<Utc>>().map_err(|_| {
                AnalyzerError::InvalidTimestamp {
                    path: path.to_path_buf(),
                    value: timestamp,
                }
            })?;
            Ok((created_at, user_id, event_type))
        })
        .collect()
}

fn map_open_error(path: &Path, source: rusqlite::Error) -> AnalyzerError {
    if is_locked(&source) {
        AnalyzerError::DatabaseLocked {
            path: path.to_owned(),
        }
    } else {
        AnalyzerError::DatabaseReadFailed {
            path: path.to_owned(),
            source,
        }
    }
}

fn map_query_error(path: &Path, source: rusqlite::Error) -> AnalyzerError {
    if is_schema_mismatch(&source) {
        AnalyzerError::SchemaMismatch {
            path: path.to_owned(),
        }
    } else if is_locked(&source) {
        AnalyzerError::DatabaseLocked {
            path: path.to_owned(),
        }
    } else {
        AnalyzerError::DatabaseReadFailed {
            path: path.to_owned(),
            source,
        }
    }
}

fn is_locked(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked,
                ..
            },
            _,
        )
    )
}

fn is_schema_mismatch(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(_, Some(message))
            if message.contains("no such table") || message.contains("no such column")
    )
}

fn reconstruct_running_intervals(events: &[DateTime<Utc>], threshold: Duration) -> Vec<Interval> {
    let threshold = ChronoDuration::from_std(threshold).unwrap_or(ChronoDuration::MAX);
    let mut intervals = Vec::new();
    let mut run_start = None;
    for pair in events.windows(2) {
        let duration = pair[1].signed_duration_since(pair[0]);
        if duration >= ChronoDuration::zero() && duration <= threshold {
            run_start.get_or_insert(pair[0]);
        } else if let Some(start) = run_start.take() {
            intervals.push(Interval {
                start,
                end: pair[0],
            });
        }
    }
    if let (Some(start), Some(end)) = (run_start, events.last().copied())
        && end > start
    {
        intervals.push(Interval { start, end });
    }
    intervals
}

fn clamp_to_running(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    running: &[Interval],
) -> Vec<Interval> {
    if end <= start {
        return Vec::new();
    }
    running
        .iter()
        .filter_map(|interval| {
            let start = start.max(interval.start);
            let end = end.min(interval.end);
            (end > start).then_some(Interval { start, end })
        })
        .collect()
}

fn clip_to_range(
    interval: Interval,
    start_time: Option<&DateTime<Local>>,
    end_time: Option<&DateTime<Local>>,
) -> Option<Interval> {
    let start = start_time.map_or(interval.start, |value| {
        interval.start.max(value.with_timezone(&Utc))
    });
    let end = end_time.map_or(interval.end, |value| {
        interval.end.min(value.with_timezone(&Utc))
    });
    (end > start).then_some(Interval { start, end })
}

fn register_activity(
    interval: &Interval,
    duration: Duration,
    buckets: &mut [Vec<BucketValue>; DAYS_PER_WEEK],
) {
    let end = interval.end.with_timezone(&Local);
    let start = interval.start.with_timezone(&Local);
    let step = ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::minutes(1));
    let mut current = floor_local_bucket(start, duration);

    if current != start {
        let next = current + step;
        if next.with_timezone(&Utc) - start.with_timezone(&Utc)
            > ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::minutes(1)) / 2
        {
            visit_activity_bucket(current, duration, buckets);
        }
        current = next;
    }

    while current < end {
        visit_activity_bucket(current, duration, buckets);
        current += step;
    }
}

fn visit_activity_bucket(
    bucket: chrono::DateTime<Local>,
    duration: Duration,
    buckets: &mut [Vec<BucketValue>; DAYS_PER_WEEK],
) {
    let weekday = bucket.weekday().num_days_from_monday() as usize;
    let seconds = u64::from(bucket.hour()) * 3_600
        + u64::from(bucket.minute()) * 60
        + u64::from(bucket.second());
    let index = (seconds / duration.as_secs()) as usize;
    if let Some(value) = buckets[weekday].get_mut(index) {
        value.activity_dates.insert(bucket.date_naive());
    }
}

fn register_online(
    interval: &Interval,
    duration: Duration,
    buckets: &mut [Vec<BucketValue>; DAYS_PER_WEEK],
) {
    for_bucket_in_interval(interval, duration, |weekday, index, _| {
        buckets[weekday][index].online_count += 1;
    });
}

fn for_bucket_in_interval(
    interval: &Interval,
    duration: Duration,
    mut visit: impl FnMut(usize, usize, chrono::NaiveDate),
) {
    let seconds = duration.as_secs();
    let mut current = floor_local_bucket(interval.start.with_timezone(&Local), duration);
    let step = ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::minutes(1));
    while current.with_timezone(&Utc) < interval.end {
        let weekday = current.weekday().num_days_from_monday() as usize;
        let bucket_seconds = u64::from(current.hour()) * 60 * 60
            + u64::from(current.minute()) * 60
            + u64::from(current.second());
        let index = (bucket_seconds / seconds) as usize;
        visit(weekday, index, current.date_naive());
        current += step;
    }
}

fn floor_local_bucket(
    mut current: chrono::DateTime<Local>,
    duration: Duration,
) -> chrono::DateTime<Local> {
    let seconds = duration.as_secs();
    let seconds_since_midnight = u64::from(current.hour()) * 3_600
        + u64::from(current.minute()) * 60
        + u64::from(current.second());
    current -= ChronoDuration::seconds((seconds_since_midnight % seconds) as i64);
    current - ChronoDuration::nanoseconds(i64::from(current.nanosecond()))
}
