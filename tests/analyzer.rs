use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use rusqlite::Connection;
use vrcx_optimal_time_app::{
    analyzer::{AnalyzerError, analyze},
    model::{AnalysisRequest, MissingDataBehavior},
};

const USER_ID: &str = "usr_550e8400-e29b-41d4-a716-446655440000";
const FRIEND_A: &str = "usr_650e8400-e29b-41d4-a716-446655440000";
const FRIEND_B: &str = "usr_750e8400-e29b-41d4-a716-446655440000";
static FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn fixture_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "vrcx-optimal-time-app-analyzer-{}-{}.sqlite3",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed),
    ))
}

fn database_snapshot(path: &std::path::Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let sidecars = [
        path.to_owned(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
        PathBuf::from(format!("{}-journal", path.display())),
    ];
    sidecars
        .into_iter()
        .map(|path| {
            let contents = fs::read(&path).ok();
            (path, contents)
        })
        .collect()
}

fn create_fixture(path: &std::path::Path) {
    let _ = fs::remove_file(path);
    let user_table = USER_ID.replace(['-', '_'], "");
    let connection = Connection::open(path).unwrap();
    for suffix in [
        "feed_avatar",
        "feed_gps",
        "feed_online_offline",
        "feed_status",
        "friend_log_history",
    ] {
        connection
            .execute(
                &format!("create table {user_table}_{suffix} (created_at text not null, user_id text, display_name text, type text)"),
                (),
            )
            .unwrap();
    }

    for timestamp in [
        "2024-01-01T00:00:00Z",
        "2024-01-01T00:10:00Z",
        "2024-01-01T00:20:00Z",
        "2024-01-01T00:30:00Z",
    ] {
        connection
            .execute(
                &format!("insert into {user_table}_feed_avatar (created_at) values (?1)"),
                [timestamp],
            )
            .unwrap();
    }
    for (timestamp, event_type) in [
        ("2024-01-01T00:05:00Z", "Online"),
        ("2024-01-01T00:25:00Z", "Offline"),
    ] {
        connection
            .execute(
                &format!("insert into {user_table}_feed_online_offline (created_at, user_id, display_name, type) values (?1, 'usr_friend', 'Friend', ?2)"),
                [timestamp, event_type],
            )
            .unwrap();
    }
}

fn create_partial_bucket_fixture(path: &std::path::Path) {
    let _ = fs::remove_file(path);
    let user_table = USER_ID.replace(['-', '_'], "");
    let connection = Connection::open(path).unwrap();
    for suffix in [
        "feed_avatar",
        "feed_gps",
        "feed_online_offline",
        "feed_status",
        "friend_log_history",
    ] {
        connection
            .execute(
                &format!("create table {user_table}_{suffix} (created_at text not null, user_id text, display_name text, type text)"),
                (),
            )
            .unwrap();
    }
    for timestamp in ["2024-01-01T00:01:00Z", "2024-01-01T00:04:00Z"] {
        connection
            .execute(
                &format!("insert into {user_table}_feed_avatar (created_at) values (?1)"),
                [timestamp],
            )
            .unwrap();
    }
    for (timestamp, event_type) in [
        ("2024-01-01T00:01:30Z", "Online"),
        ("2024-01-01T00:03:00Z", "Offline"),
    ] {
        connection
            .execute(
                &format!("insert into {user_table}_feed_online_offline (created_at, user_id, display_name, type) values (?1, 'usr_friend', 'Friend', ?2)"),
                [timestamp, event_type],
            )
            .unwrap();
    }
}

#[test]
fn analyzer_returns_weekly_buckets_without_mutating_its_database() {
    let path = fixture_path();
    create_fixture(&path);
    let before = database_snapshot(&path);
    let mut request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        friend_ids: Vec::new(),
        database_path: path.clone(),
        uptime_threshold: Duration::from_secs(15 * 60),
        bucket_duration: Duration::from_secs(10 * 60),
        normalize: false,
        start_time: None::<DateTime<Local>>,
        end_time: None::<DateTime<Local>>,
        minimum_activations: 1,
        missing_data: MissingDataBehavior::Gap,
    };

    let result = analyze(&request).unwrap();

    assert_eq!(result.graph.weekdays.len(), 7);
    assert_eq!(result.graph.weekdays[0].len(), 144);
    assert!(result.graph.weekdays.iter().flatten().any(Option::is_some));

    request.minimum_activations = 2;
    let filtered = analyze(&request).unwrap();
    assert!(
        filtered
            .graph
            .weekdays
            .iter()
            .flatten()
            .all(Option::is_none)
    );

    assert_eq!(database_snapshot(&path), before);
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_accepts_valid_subminute_bucket_durations_without_panicking() {
    let path = fixture_path();
    create_fixture(&path);
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        friend_ids: Vec::new(),
        database_path: path.clone(),
        uptime_threshold: Duration::from_secs(15 * 60),
        bucket_duration: Duration::from_secs(30),
        normalize: false,
        start_time: None::<DateTime<Local>>,
        end_time: None::<DateTime<Local>>,
        minimum_activations: 1,
        missing_data: MissingDataBehavior::Gap,
    };

    let result = analyze(&request).unwrap();

    assert_eq!(result.graph.weekdays[0].len(), 2_880);
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_assigns_partial_activity_to_the_bucket_containing_interval_start() {
    let path = fixture_path();
    create_partial_bucket_fixture(&path);
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        friend_ids: Vec::new(),
        database_path: path.clone(),
        uptime_threshold: Duration::from_secs(15 * 60),
        bucket_duration: Duration::from_secs(10 * 60),
        normalize: true,
        start_time: None::<DateTime<Local>>,
        end_time: None::<DateTime<Local>>,
        minimum_activations: 1,
        missing_data: MissingDataBehavior::Gap,
    };

    let result = analyze(&request).unwrap();

    let local_start = DateTime::parse_from_rfc3339("2024-01-01T00:01:00Z")
        .unwrap()
        .with_timezone(&Local);
    let weekday = local_start.weekday().num_days_from_monday() as usize;
    let bucket = (local_start.hour() * 60 + local_start.minute()) / 10;
    assert_eq!(result.graph.weekdays[weekday][bucket as usize], Some(1.0));
    assert!(result.graph.weekdays[weekday][bucket as usize + 1].is_none());
    assert!(
        result
            .graph
            .weekdays
            .iter()
            .flatten()
            .flatten()
            .all(|value| value.is_finite())
    );
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_preserves_online_state_across_start_time_boundary() {
    let path = fixture_path();
    create_fixture(&path);
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        friend_ids: Vec::new(),
        database_path: path.clone(),
        uptime_threshold: Duration::from_secs(15 * 60),
        bucket_duration: Duration::from_secs(10 * 60),
        start_time: Some(
            "2024-01-01T00:15:00Z"
                .parse::<DateTime<chrono::Utc>>()
                .unwrap()
                .with_timezone(&Local),
        ),
        ..AnalysisRequest::default()
    };

    let result = analyze(&request).unwrap();
    let local_start = "2024-01-01T00:15:00Z"
        .parse::<DateTime<chrono::Utc>>()
        .unwrap()
        .with_timezone(&Local);
    let weekday = local_start.weekday().num_days_from_monday() as usize;
    let bucket = (local_start.hour() * 60 + local_start.minute()) as usize / 10;
    assert_eq!(result.graph.weekdays[weekday][bucket], Some(1.0));

    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_clips_running_activity_at_end_time() {
    let path = fixture_path();
    create_fixture(&path);
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        database_path: path.clone(),
        uptime_threshold: Duration::from_secs(15 * 60),
        bucket_duration: Duration::from_secs(10 * 60),
        end_time: Some(
            "2024-01-01T00:15:00Z"
                .parse::<DateTime<chrono::Utc>>()
                .unwrap()
                .with_timezone(&Local),
        ),
        ..AnalysisRequest::default()
    };

    let result = analyze(&request).unwrap();
    let start_local = "2024-01-01T00:00:00Z"
        .parse::<DateTime<chrono::Utc>>()
        .unwrap()
        .with_timezone(&Local);
    let end_local = "2024-01-01T00:15:00Z"
        .parse::<DateTime<chrono::Utc>>()
        .unwrap()
        .with_timezone(&Local);
    let weekday = start_local.weekday().num_days_from_monday() as usize;
    let start_bucket = (start_local.hour() * 60 + start_local.minute()) as usize / 10;
    let end_bucket = (end_local.hour() * 60 + end_local.minute()) as usize / 10;

    assert!(result.graph.weekdays[weekday][start_bucket].is_some());
    assert!(result.graph.weekdays[weekday][end_bucket].is_some());
    assert!(result.graph.weekdays[weekday][end_bucket + 1].is_none());
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_clips_both_bounds_and_friend_graphs_to_same_interval() {
    let path = fixture_path();
    create_fixture(&path);
    let table = USER_ID.replace(['-', '_'], "");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            &format!(
                "insert into {table}_feed_online_offline (created_at, user_id, display_name, type) values
                 ('2024-01-01T00:05:00Z', ?1, 'Friend A', 'Online'),
                 ('2024-01-01T00:10:00Z', ?1, 'Friend A', 'Offline'),
                 ('2024-01-01T00:15:00Z', ?2, 'Friend B', 'Online'),
                 ('2024-01-01T00:20:00Z', ?2, 'Friend B', 'Offline')"
            ),
            [FRIEND_A, FRIEND_B],
        )
        .unwrap();
    drop(connection);

    let start = Local.with_ymd_and_hms(2024, 1, 1, 8, 12, 0).single();
    let end = Local.with_ymd_and_hms(2024, 1, 1, 8, 18, 0).single();
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        friend_ids: vec![FRIEND_A.to_owned(), FRIEND_B.to_owned()],
        database_path: path.clone(),
        start_time: start,
        end_time: end,
        ..AnalysisRequest::default()
    };

    let result = analyze(&request).unwrap();
    let bucket_index = |value: DateTime<Local>| (value.hour() * 60 + value.minute()) as usize / 10;
    let friend_a_bucket = bucket_index(
        Local
            .with_ymd_and_hms(2024, 1, 1, 8, 5, 0)
            .single()
            .unwrap(),
    );
    let friend_b_bucket = bucket_index(
        Local
            .with_ymd_and_hms(2024, 1, 1, 8, 15, 0)
            .single()
            .unwrap(),
    );
    let weekday = start.unwrap().weekday().num_days_from_monday() as usize;

    assert!(
        result.friend_graphs[0]
            .graph
            .weekdays
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    assert_eq!(
        result.friend_graphs[1].graph.weekdays[weekday][friend_b_bucket],
        Some(1.0)
    );
    assert!(result.graph.weekdays[weekday][friend_a_bucket].is_none());
    assert!(result.graph.weekdays[weekday][friend_b_bucket].is_some());
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_rejects_range_with_no_activity_after_clipping() {
    let path = fixture_path();
    create_fixture(&path);
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        database_path: path.clone(),
        start_time: Local.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).single(),
        end_time: Local.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).single(),
        ..AnalysisRequest::default()
    };

    assert!(matches!(
        analyze(&request),
        Err(AnalyzerError::NoUsableActivity { .. })
    ));
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_rejects_empty_start_and_end_interval() {
    let path = fixture_path();
    create_fixture(&path);
    let bound = Local.with_ymd_and_hms(2024, 1, 1, 8, 15, 0).single();
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        database_path: path.clone(),
        start_time: bound,
        end_time: bound,
        ..AnalysisRequest::default()
    };

    assert!(matches!(
        analyze(&request),
        Err(AnalyzerError::NoUsableActivity { .. })
    ));
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_translates_missing_and_schema_mismatch_databases() {
    let missing = fixture_path();
    let missing_request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        database_path: missing.clone(),
        ..AnalysisRequest::default()
    };
    assert!(matches!(
        analyze(&missing_request),
        Err(AnalyzerError::DatabaseNotFound { .. })
    ));

    let schema_path = fixture_path();
    Connection::open(&schema_path).unwrap();
    let schema_request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        database_path: schema_path.clone(),
        ..AnalysisRequest::default()
    };
    assert!(matches!(
        analyze(&schema_request),
        Err(AnalyzerError::SchemaMismatch { .. })
    ));
    let _ = fs::remove_file(schema_path);
}

#[test]
fn analyzer_translates_corrupt_database_to_read_failure() {
    let path = fixture_path();
    fs::write(&path, b"not a sqlite database").unwrap();
    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        database_path: path.clone(),
        ..AnalysisRequest::default()
    };

    assert!(matches!(
        analyze(&request),
        Err(AnalyzerError::DatabaseReadFailed { .. })
    ));
    let _ = fs::remove_file(path);
}

#[test]
fn analyzer_returns_individual_graphs_for_selected_friends() {
    let path = fixture_path();
    create_fixture(&path);
    let table = USER_ID.replace(['-', '_'], "");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            &format!(
                "insert into {table}_feed_online_offline (created_at, user_id, display_name, type) values
                 ('2024-01-01T00:05:00Z', ?1, 'Friend A', 'Online'),
                 ('2024-01-01T00:10:00Z', ?1, 'Friend A', 'Offline'),
                 ('2024-01-01T00:15:00Z', ?2, 'Friend B', 'Online'),
                 ('2024-01-01T00:20:00Z', ?2, 'Friend B', 'Offline')"
            ),
            [FRIEND_A, FRIEND_B],
        )
        .unwrap();
    drop(connection);

    let request = AnalysisRequest {
        your_user_id: USER_ID.to_owned(),
        friend_ids: vec![FRIEND_A.to_owned(), FRIEND_B.to_owned()],
        database_path: path.clone(),
        uptime_threshold: Duration::from_secs(15 * 60),
        bucket_duration: Duration::from_secs(10 * 60),
        ..AnalysisRequest::default()
    };
    let result = analyze(&request).unwrap();

    assert_eq!(
        result
            .friend_graphs
            .iter()
            .map(|friend| friend.friend_id.as_str())
            .collect::<Vec<_>>(),
        vec![FRIEND_A, FRIEND_B]
    );
    let friend_a_local = "2024-01-01T00:05:00Z"
        .parse::<DateTime<chrono::Utc>>()
        .unwrap()
        .with_timezone(&Local);
    let friend_b_local = "2024-01-01T00:15:00Z"
        .parse::<DateTime<chrono::Utc>>()
        .unwrap()
        .with_timezone(&Local);
    let bucket_index = |value: DateTime<Local>| (value.hour() * 60 + value.minute()) as usize / 10;
    assert_eq!(
        result.friend_graphs[0].graph.weekdays
            [friend_a_local.weekday().num_days_from_monday() as usize]
            [bucket_index(friend_a_local)],
        Some(1.0)
    );
    assert_eq!(
        result.friend_graphs[1].graph.weekdays
            [friend_b_local.weekday().num_days_from_monday() as usize]
            [bucket_index(friend_b_local)],
        Some(1.0)
    );
    let _ = fs::remove_file(path);
}
