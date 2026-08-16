use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use chrono::Local;

pub fn default_log_path() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA").map(|root| {
        PathBuf::from(root)
            .join("VRCX Optimal Time App")
            .join("logs")
            .join("app.log")
    })
}

pub fn append_log(path: &Path, message: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{} {message}", Local::now().to_rfc3339())
}
