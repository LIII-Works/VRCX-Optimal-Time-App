use std::{
    env,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DatabasePathError {
    #[error("APPDATA is unavailable; cannot determine the default VRCX database path")]
    AppDataUnavailable,
}

pub fn default_database_path() -> Result<PathBuf, DatabasePathError> {
    let app_data = env::var_os("APPDATA").ok_or(DatabasePathError::AppDataUnavailable)?;
    Ok(database_path_from_app_data(Path::new(&app_data)))
}

pub fn database_path_from_app_data(app_data: &Path) -> PathBuf {
    app_data.join("VRCX").join("VRCX.sqlite3")
}

pub fn resolve_database_path(custom_path: Option<&Path>) -> Result<PathBuf, DatabasePathError> {
    custom_path
        .map(Path::to_path_buf)
        .map(Ok)
        .unwrap_or_else(default_database_path)
}
