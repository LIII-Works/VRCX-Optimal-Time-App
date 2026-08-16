use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use vrcx_optimal_time_app::{
    model::AnalysisRequest,
    refresh::{RefreshCoordinator, RefreshReason},
};

fn request() -> AnalysisRequest {
    AnalysisRequest {
        database_path: PathBuf::from("fixture.sqlite3"),
        ..AnalysisRequest::default()
    }
}

#[test]
fn edits_inside_debounce_window_coalesce_and_newer_generation_wins() {
    let start = Instant::now();
    let mut coordinator = RefreshCoordinator::default();

    let first = coordinator.request(request(), RefreshReason::Launch, start);
    coordinator.request(
        request(),
        RefreshReason::ControlChanged,
        start + Duration::from_millis(100),
    );
    let third = coordinator.request(
        request(),
        RefreshReason::ControlChanged,
        start + Duration::from_millis(200),
    );

    assert_eq!(first, 1);
    assert_eq!(third, 3);
    assert!(
        coordinator
            .poll(start + Duration::from_millis(299))
            .is_none()
    );
    let job = coordinator
        .poll(start + Duration::from_millis(500))
        .unwrap();
    assert_eq!(job.generation, third);
    assert_eq!(job.reason, RefreshReason::ControlChanged);
    assert!(!coordinator.is_current(first));
    assert!(coordinator.is_current(third));

    let fourth = coordinator.request(
        request(),
        RefreshReason::DatabaseRefresh,
        start + Duration::from_millis(600),
    );
    assert_eq!(fourth, 4);
    assert!(
        coordinator
            .poll(start + Duration::from_millis(899))
            .is_none()
    );
    assert_eq!(
        coordinator
            .poll(start + Duration::from_millis(900))
            .unwrap()
            .generation,
        4
    );
}
