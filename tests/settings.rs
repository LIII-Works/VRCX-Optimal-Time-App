use std::{fs, path::PathBuf};

use chrono::{Local, TimeZone};

use vrcx_optimal_time_app::{
    model::{AppSettings, MissingDataBehavior},
    settings::{
        SETTINGS_SCHEMA_VERSION, SettingsError, default_settings_path, load_settings, save_settings,
    },
};

fn test_path(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "vrcx-optimal-time-app-{test_name}-{}-settings.toml",
        std::process::id()
    ))
}

fn cleanup(path: &std::path::Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("toml.tmp"));
}

#[test]
fn default_settings_path_is_next_to_the_running_executable() {
    let executable = std::env::current_exe().unwrap();
    let expected = executable.parent().unwrap().join("settings.toml");

    assert_eq!(default_settings_path().unwrap(), expected);
}

#[test]
fn missing_settings_file_loads_defaults() {
    let path = test_path("missing-defaults");
    cleanup(&path);

    assert_eq!(load_settings(&path).unwrap(), AppSettings::default());
    assert!(!path.exists());
}

#[test]
fn settings_round_trip_through_versioned_toml_and_atomic_replacement() {
    let path = test_path("round-trip");
    cleanup(&path);
    let mut settings = AppSettings::default();
    settings.analysis.your_user_id = "usr_550e8400-e29b-41d4-a716-446655440000".to_owned();
    settings.analysis.missing_data = MissingDataBehavior::Zero;
    settings.analysis.start_time = Local.with_ymd_and_hms(2024, 1, 1, 8, 30, 0).single();
    settings.analysis.end_time = Local.with_ymd_and_hms(2024, 2, 29, 18, 45, 0).single();
    settings.window.friend_ids_collapsed = true;

    save_settings(&path, &settings).unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.contains(&format!("schema_version = {SETTINGS_SCHEMA_VERSION}")));
    assert!(!path.with_extension("toml.tmp").exists());
    assert_eq!(load_settings(&path).unwrap(), settings);

    settings.window.friend_ids_collapsed = false;
    save_settings(&path, &settings).unwrap();
    assert!(!path.with_extension("toml.tmp").exists());
    assert_eq!(load_settings(&path).unwrap(), settings);
    cleanup(&path);
}

#[test]
fn version_one_migrates_missing_current_fields_to_defaults() {
    let path = test_path("migration");
    cleanup(&path);
    fs::write(
        &path,
        "schema_version = 1\n[analysis]\nyour_user_id = 'usr_550e8400-e29b-41d4-a716-446655440000'\n",
    )
    .unwrap();

    let loaded = load_settings(&path).unwrap();
    assert_eq!(
        loaded.analysis.your_user_id,
        "usr_550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(
        loaded.analysis.bucket_duration,
        AppSettings::default().analysis.bucket_duration
    );
    assert_eq!(loaded.analysis.end_time, None);
    assert_eq!(loaded.window, AppSettings::default().window);
    cleanup(&path);
}

#[test]
fn malformed_toml_and_future_versions_fail_without_overwriting_the_file() {
    for (name, contents) in [
        ("malformed", "schema_version = ["),
        ("future", "schema_version = 99"),
    ] {
        let path = test_path(name);
        cleanup(&path);
        fs::write(&path, contents).unwrap();

        let error = load_settings(&path).unwrap_err();
        match name {
            "malformed" => assert!(matches!(error, SettingsError::MalformedToml { .. })),
            "future" => assert!(matches!(error, SettingsError::FutureSchema { .. })),
            _ => unreachable!(),
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        cleanup(&path);
    }
}

#[test]
fn invalid_persisted_ids_are_not_loaded_as_active_settings() {
    let path = test_path("invalid-ids");
    cleanup(&path);
    fs::write(
        &path,
        "schema_version = 1\n[analysis]\nyour_user_id = 'not-an-id'\nfriend_ids = ['usr_550e8400-e29b-41d4-a716-446655440000', 'bad']\n",
    )
    .unwrap();

    let loaded = load_settings(&path).unwrap();

    assert!(loaded.analysis.your_user_id.is_empty());
    assert_eq!(
        loaded.analysis.friend_ids,
        vec!["usr_550e8400-e29b-41d4-a716-446655440000"]
    );
    cleanup(&path);
}
