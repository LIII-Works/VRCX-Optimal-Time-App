use std::{
    fs,
    path::{Path, PathBuf},
};

use vrcx_optimal_time_app::{
    VrcxOptimalTimeApp,
    analyzer::analyze,
    app::{APP_TITLE, native_window_options},
    model::AnalysisRequest,
};

const SELF_TEST_USER_ID: &str = "usr_550e8400-e29b-41d4-a716-446655440000";
const SELF_TEST_EXPECTED: &str = include_str!("../tests/fixtures/expected.json");

fn self_test_summary(path: &Path) -> Result<String, String> {
    let before = snapshot_database(path);
    let result = analyze(&AnalysisRequest {
        your_user_id: SELF_TEST_USER_ID.to_owned(),
        friend_ids: Vec::new(),
        database_path: path.to_owned(),
        ..AnalysisRequest::default()
    })
    .map_err(|error| error.to_string())?;
    let populated_buckets = result
        .graph
        .weekdays
        .iter()
        .flatten()
        .filter(|value| value.is_some())
        .count();
    let value_sum: f64 = result.graph.weekdays.iter().flatten().flatten().sum();
    if snapshot_database(path) != before {
        return Err("analysis changed the SQLite database or sidecar files".to_owned());
    }
    Ok(format!(
        "{{\"all_event_count\":{},\"online_offline_event_count\":{},\"weekday_count\":7,\"bucket_count\":{},\"populated_buckets\":{},\"value_sum\":{value_sum:.6}}}",
        result.all_event_count,
        result.online_offline_event_count,
        result.graph.weekdays[0].len(),
        populated_buckets,
    ))
}

fn snapshot_database(path: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    [
        path.to_owned(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
        PathBuf::from(format!("{}-journal", path.display())),
    ]
    .into_iter()
    .map(|candidate| (candidate.clone(), fs::read(candidate).ok()))
    .collect()
}

fn run_self_test(path: &Path) -> Result<(), String> {
    let actual = self_test_summary(path)?;
    let expected = SELF_TEST_EXPECTED.trim();
    if actual != expected {
        return Err(format!(
            "fixture mismatch: expected {expected}, got {actual}"
        ));
    }
    println!("self-test passed: {actual}");
    Ok(())
}

fn cli_mode() -> Option<i32> {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new("--self-test")) {
        return None;
    }
    let Some(path) = arguments.next() else {
        eprintln!("usage: VRCXOptimalTimeApp.exe --self-test <sqlite-path>");
        return Some(2);
    };
    if arguments.next().is_some() {
        eprintln!("usage: VRCXOptimalTimeApp.exe --self-test <sqlite-path>");
        return Some(2);
    }
    match run_self_test(Path::new(&path)) {
        Ok(()) => Some(0),
        Err(error) => {
            eprintln!("self-test failed: {error}");
            Some(1)
        }
    }
}

fn main() -> eframe::Result {
    if let Some(code) = cli_mode() {
        std::process::exit(code);
    }
    eframe::run_native(
        APP_TITLE,
        native_window_options(),
        Box::new(|_creation_context| Ok(Box::new(VrcxOptimalTimeApp::default()))),
    )
}
