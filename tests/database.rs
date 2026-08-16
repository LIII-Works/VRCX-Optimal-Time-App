use std::path::Path;

use vrcx_optimal_time_app::database::{database_path_from_app_data, resolve_database_path};

#[test]
fn default_database_path_matches_vrcx_appdata_layout() {
    assert_eq!(
        database_path_from_app_data(Path::new(r"C:\Users\test\AppData\Roaming")),
        Path::new(r"C:\Users\test\AppData\Roaming\VRCX\VRCX.sqlite3")
    );
}

#[test]
fn custom_database_path_wins_without_environment_discovery() {
    let custom = Path::new(r"D:\data\copy.sqlite3");
    assert_eq!(resolve_database_path(Some(custom)).unwrap(), custom);
}
