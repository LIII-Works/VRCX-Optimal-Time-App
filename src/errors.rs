use thiserror::Error;

use crate::{settings::SettingsError, validation::ValidationError};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("application startup failed: {0}")]
    Startup(String),
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

pub type AppResult<T> = Result<T, AppError>;
