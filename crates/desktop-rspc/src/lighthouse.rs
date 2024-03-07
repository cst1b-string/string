use std::{collections::HashMap, net::SocketAddr, path::Path};

use pgp::SignedSecretKey;
use serde::{Deserialize, Serialize};
use string_comm::DEFAULT_PORT;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug)]
pub struct LighthouseContext {
    settings: RwLock<LighthouseSettings>,
}

impl LighthouseContext {
    /// Create a new lighthouse context with the given data path.
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
    /// List potential peers.
    pub async fn list_potential_peers(
        &self,
        secret_key: SignedSecretKey,
    ) -> Result<HashMap<String, SocketAddr>, LighthouseError> {
        let settings = self.settings.read().await;
        let results =
            lighthouse_client::list_potential_peers(&settings.endpoint, &secret_key).await?;
        Ok(results)
    }

    pub async fn get_node_address<F: AsRef<[u8]>>(
        &self,
        fingerprint: F,
    ) -> Result<SocketAddr, LighthouseError> {
        let settings = self.settings.read().await;
        let results = lighthouse_client::get_node_address(
            &settings.endpoint,
            None,
            DEFAULT_PORT,
            fingerprint,
        )
        .await?;
        Ok(results)
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
