use std::fs;

use vrcx_optimal_time_app::diagnostics::append_log;

#[test]
fn diagnostics_create_log_directory_and_append_technical_message() {
    let root = std::env::temp_dir().join(format!(
        "vrcx-optimal-time-app-diagnostics-{}",
        std::process::id()
    ));
    let log = root.join("logs").join("app.log");
    let _ = fs::remove_dir_all(&root);

    append_log(&log, "database locked").unwrap();

    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("database locked"));
    let _ = fs::remove_dir_all(root);
}
