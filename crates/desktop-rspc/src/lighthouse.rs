use std::{net::SocketAddr, path::Path};

use pgp::SignedSecretKey;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

pub struct LighthouseContext {
    settings: RwLock<LighthouseSettings>,
}

impl LighthouseContext {
    pub async fn from_data_dir<P: AsRef<Path>>(path: P) -> Result<Self, LighthouseError> {
        let path = path.as_ref().to_owned();
        let settings_path = path.join("lighthouse.json");
        info!("- Lighthouse path: {:?}", settings_path);

        // check if file exists, otherwise copy defaults
        if !settings_path.exists() {
            tokio::fs::create_dir_all(path).await?;
            let file = tokio::fs::File::create(&settings_path).await?;
            let settings = LighthouseSettings::default();
            serde_json::to_writer(
                file.try_into_std().expect("failed to downcast tokio File"),
                &settings,
            )?;
        }

        // read from file
        let file = tokio::fs::File::open(&settings_path).await?;
        let settings: LighthouseSettings =
            serde_json::from_reader(file.try_into_std().expect("failed to downcast tokio File"))?;

        Ok(Self {
            settings: settings.into(),
        })
    }
}

impl LighthouseContext {
    pub async fn list_potential_peers(
        &self,
        secret_key: SignedSecretKey,
    ) -> Result<Vec<SocketAddr>, LighthouseError> {
        let settings = self.settings.read().await;
        lighthouse_client::list_potential_peers(&settings.endpoint, &secret_key).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LighthouseSettings {
    endpoint: String,
}

impl Default for LighthouseSettings {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:5050".to_string(),
        }
    }
}

/// An enum of errors that can occur when working with settings.
#[derive(Error, Debug)]
pub enum LighthouseError {
    #[error("encountered an IO error")]
    IoError(#[from] std::io::Error),
    #[error("failed to serialize/deserialize settings")]
    SerdeError(#[from] serde_json::Error),
    #[error("encountered a client error")]
    ClientError(#[from] lighthouse_client::LighthouseClientError),
}
