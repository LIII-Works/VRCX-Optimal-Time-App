use vrcx_optimal_time_app::{
    VrcxOptimalTimeApp,
    app::{APP_TITLE, native_window_options},
    model::{
        AnalysisRequest, AnalysisSettings, AppSettings, AppStatus, WeeklyGraph, WindowSettings,
    },
    settings::{default_settings_path, load_settings},
};

#[test]
fn shell_defaults_and_window_configuration_are_typed_and_stable() {
    let app = VrcxOptimalTimeApp::default();
    let _settings = AppSettings {
        analysis: AnalysisSettings::default(),
        window: WindowSettings::default(),
    };
    let _request = AnalysisRequest::default();
    let _graph = WeeklyGraph::default();
    let _status = AppStatus::default();

    assert_eq!(app.status, AppStatus::Idle);
    assert!(app.weekly_graph.is_none());
    assert_eq!(app.settings.analysis.uptime_threshold.as_secs(), 300);
    assert_eq!(app.settings.analysis.bucket_duration.as_secs(), 600);
    assert_eq!(app.settings.analysis.minimum_activations, 1);
    let expected_window_size = default_settings_path()
        .ok()
        .and_then(|path| load_settings(&path).ok())
        .map_or([1280.0, 720.0], |settings| settings.window.size);
    assert_eq!(app.settings.window.size, expected_window_size);

    assert_eq!(APP_TITLE, "VRCX Optimal Time");

    let options = native_window_options();
    assert_eq!(
        options.viewport.inner_size,
        Some(eframe::egui::vec2(
            expected_window_size[0],
            expected_window_size[1],
        ))
    );
}
