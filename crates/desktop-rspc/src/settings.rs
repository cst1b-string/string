//! Defines queries that interact with the application settings and file system.

use std::path::Path;

use rspc::{RouterBuilder, Type};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

use crate::Ctx;

/// The theme of the application.
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, Type)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

/// The settings for the application.
#[derive(Default, Debug, Serialize, Deserialize, Type)]
pub struct Settings {
    pub theme: Theme,
}

/// The context for the settings.
pub struct SettingsContext {
    /// The underlying settings.
    pub settings: RwLock<Settings>,
}

/// An enum of errors that can occur when working with settings.
#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("encountered an IO error")]
    IoError(#[from] std::io::Error),
    #[error("failed to serialize/deserialize settings")]
    SerdeError(#[from] serde_json::Error),
}

impl SettingsContext {
    /// Attempt to load the settings from the given path.
    pub async fn from_data_dir<P: AsRef<Path>>(path: P) -> Result<Self, SettingsError> {
        let path = path.as_ref().to_owned();
        let settings_path = path.join("settings.json");
        info!("- Settings path: {:?}", settings_path);

        // check if file exists, otherwise copy defaults
        if !settings_path.exists() {
            tokio::fs::create_dir_all(path).await?;
            let file = tokio::fs::File::create(&settings_path).await?;
            let settings = Settings::default();
            serde_json::to_writer(
                file.try_into_std().expect("failed to downcast tokio File"),
                &settings,
            )?;
        }

        // read from file
        let content = tokio::fs::read_to_string(&settings_path).await?;
        let settings = serde_json::from_str(&content)?;

        Ok(Self::from_settings(settings))
    }

    /// Create a new settings context from the underlying settings.
    pub fn from_settings(settings: Settings) -> SettingsContext {
        Self {
            settings: RwLock::new(settings),
        }
    }

    /// Save the settings to the given path.
    pub async fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), SettingsError> {
        let path = path.as_ref();
        let file = std::fs::File::create(path)?;

        // write to file
        serde_json::to_writer(file, &*self.settings.read().await)?;

        Ok(())
    }
}

/// Attach the settings queries to the router.
pub fn attach_settings_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
        .query("settings.theme", |t| t(get_settings_theme))
        .mutation("settings.theme", |t| t(update_settings_theme))
}

/// Get the theme from the settings.
async fn get_settings_theme(ctx: Ctx, _: ()) -> Result<Theme, rspc::Error> {
    Ok(ctx.settings_ctx.settings.read().await.theme)
}

/// Update the theme to the settings.
async fn update_settings_theme(ctx: Ctx, theme: Theme) -> Result<(), rspc::Error> {
    ctx.settings_ctx.settings.write().await.theme = theme;
    Ok(())
}
