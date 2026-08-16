use std::{path::PathBuf, time::Duration};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub analysis: AnalysisSettings,
    #[serde(default)]
    pub window: WindowSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisSettings {
    #[serde(default)]
    pub your_user_id: String,
    #[serde(default)]
    pub friend_ids: Vec<String>,
    #[serde(default)]
    pub database_path: Option<PathBuf>,
    #[serde(default = "default_uptime_threshold")]
    pub uptime_threshold: Duration,
    #[serde(default = "default_bucket_duration")]
    pub bucket_duration: Duration,
    #[serde(default)]
    pub normalize: bool,
    #[serde(default)]
    pub start_time: Option<DateTime<Local>>,
    #[serde(default = "default_minimum_activations")]
    pub minimum_activations: u32,
    #[serde(default)]
    pub missing_data: MissingDataBehavior,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            your_user_id: String::new(),
            friend_ids: Vec::new(),
            database_path: None,
            uptime_threshold: Duration::from_secs(300),
            bucket_duration: Duration::from_secs(600),
            normalize: false,
            start_time: None,
            minimum_activations: 1,
            missing_data: MissingDataBehavior::Gap,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowSettings {
    #[serde(default = "default_window_size")]
    pub size: [f32; 2],
    #[serde(default)]
    pub position: Option<[f32; 2]>,
    #[serde(default)]
    pub friend_ids_collapsed: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            size: [1280.0, 720.0],
            position: None,
            friend_ids_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisRequest {
    pub your_user_id: String,
    pub friend_ids: Vec<String>,
    pub database_path: PathBuf,
    pub uptime_threshold: Duration,
    pub bucket_duration: Duration,
    pub normalize: bool,
    pub start_time: Option<DateTime<Local>>,
    pub minimum_activations: u32,
    pub missing_data: MissingDataBehavior,
}

impl Default for AnalysisRequest {
    fn default() -> Self {
        let settings = AnalysisSettings::default();

        Self {
            your_user_id: settings.your_user_id,
            friend_ids: settings.friend_ids,
            database_path: PathBuf::new(),
            uptime_threshold: settings.uptime_threshold,
            bucket_duration: settings.bucket_duration,
            normalize: settings.normalize,
            start_time: settings.start_time,
            minimum_activations: settings.minimum_activations,
            missing_data: settings.missing_data,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeeklyGraph {
    pub bucket_duration: Duration,
    pub weekdays: [Vec<Option<f64>>; 7],
}

impl Default for WeeklyGraph {
    fn default() -> Self {
        Self {
            bucket_duration: AnalysisSettings::default().bucket_duration,
            weekdays: std::array::from_fn(|_| Vec::new()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AppStatus {
    #[default]
    Idle,
    Calculating,
    Updated,
    Empty,
    Warning(String),
    Error(String),
}

impl AppStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Idle => "Enter a valid VRCX user ID to begin analysis.",
            Self::Calculating => "Loading database and calculating availability...",
            Self::Updated => "Availability data is current.",
            Self::Empty => {
                "Warning: not enough matching friend activity was found; capture more VRCX history."
            }
            Self::Warning(message) => message,
            Self::Error(message) => message,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingDataBehavior {
    #[default]
    Gap,
    Zero,
}

fn default_uptime_threshold() -> Duration {
    Duration::from_secs(300)
}

fn default_bucket_duration() -> Duration {
    Duration::from_secs(600)
}

fn default_minimum_activations() -> u32 {
    1
}

fn default_window_size() -> [f32; 2] {
    [1280.0, 720.0]
}
