use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    model::{AnalysisSettings, AppSettings, WindowSettings},
    validation::{parse_friend_ids, parse_user_id},
};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
const SETTINGS_DIRECTORY: &str = "VRCX Optimal Time App";
const SETTINGS_FILE_NAME: &str = "settings.toml";

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("LOCALAPPDATA is unavailable; cannot determine the settings path")]
    LocalAppDataUnavailable,
    #[error("could not read settings file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("settings file {path} contains invalid TOML: {source}")]
    MalformedToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(
        "settings file {path} uses unsupported future schema version {found}; this app supports up to {supported}"
    )]
    FutureSchema {
        path: PathBuf,
        found: u32,
        supported: u32,
    },
    #[error("could not serialize settings: {source}")]
    Serialize {
        #[source]
        source: toml::ser::Error,
    },
    #[error("could not create settings directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not write temporary settings file {path}: {source}")]
    WriteTemporary {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not replace settings file {path}: {source}")]
    Replace {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Deserialize)]
struct PersistedSettings {
    schema_version: u32,
    #[serde(default)]
    analysis: AnalysisSettings,
    #[serde(default)]
    window: WindowSettings,
}

#[derive(Serialize)]
struct SettingsForSave<'a> {
    schema_version: u32,
    analysis: &'a AnalysisSettings,
    window: &'a WindowSettings,
}

pub fn default_settings_path() -> Result<PathBuf, SettingsError> {
    let local_app_data =
        env::var_os("LOCALAPPDATA").ok_or(SettingsError::LocalAppDataUnavailable)?;
    Ok(PathBuf::from(local_app_data)
        .join(SETTINGS_DIRECTORY)
        .join(SETTINGS_FILE_NAME))
}

pub fn load_settings(path: &Path) -> Result<AppSettings, SettingsError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(AppSettings::default()),
        Err(source) => {
            return Err(SettingsError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    let persisted: PersistedSettings =
        toml::from_str(&contents).map_err(|source| SettingsError::MalformedToml {
            path: path.to_owned(),
            source,
        })?;

    if persisted.schema_version > SETTINGS_SCHEMA_VERSION {
        return Err(SettingsError::FutureSchema {
            path: path.to_owned(),
            found: persisted.schema_version,
            supported: SETTINGS_SCHEMA_VERSION,
        });
    }

    Ok(sanitize_settings(AppSettings {
        analysis: persisted.analysis,
        window: persisted.window,
    }))
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    let settings = sanitize_settings(settings.clone());
    let serialized = toml::to_string_pretty(&SettingsForSave {
        schema_version: SETTINGS_SCHEMA_VERSION,
        analysis: &settings.analysis,
        window: &settings.window,
    })
    .map_err(|source| SettingsError::Serialize { source })?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| SettingsError::CreateDirectory {
        path: parent.to_owned(),
        source,
    })?;

    let temporary_path = path.with_extension("toml.tmp");
    fs::write(&temporary_path, serialized).map_err(|source| SettingsError::WriteTemporary {
        path: temporary_path.clone(),
        source,
    })?;
    fs::rename(&temporary_path, path).map_err(|source| SettingsError::Replace {
        path: path.to_owned(),
        source,
    })
}

fn sanitize_settings(mut settings: AppSettings) -> AppSettings {
    if let Ok(user_id) = parse_user_id(&settings.analysis.your_user_id, 1) {
        settings.analysis.your_user_id = user_id;
    } else {
        settings.analysis.your_user_id.clear();
    }

    let report = parse_friend_ids(&settings.analysis.friend_ids.join("\n"));
    settings.analysis.friend_ids = report.ids;
    settings
}
