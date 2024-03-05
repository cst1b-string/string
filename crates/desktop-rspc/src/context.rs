use std::path::Path;

use string_comm::Socket;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

use crate::{
    account::AccountContext,
    settings::{self, SettingsContext},
};

/// The context type for the router.
#[derive(Debug)]
pub struct Context {
    /// The communication socket.
    pub socket: RwLock<StatefulSocket>,
    /// The Prisma client for the cache.
    pub cache: cache_prisma::PrismaClient,
    /// The settings for the application.
    pub settings_ctx: settings::SettingsContext,
    /// The account context.
    pub account_ctx: AccountContext,
}

/// Wrapper type for the socket to account for pre-login users.
#[derive(Debug)]
pub enum StatefulSocket {
    /// The socket is active.
    Active(Socket),
    /// The socket is inactive.
    Inactive,
}

/// An enum of errors that can occur when working with the context.
#[derive(Error, Debug)]
pub enum ContextError {
    #[error("failed to create the Prisma client")]
    NewClientError(#[from] cache_prisma::client::NewClientError),
    #[error("failed to create the settings context")]
    SettingsContextError(#[from] settings::SettingsError),
}

impl Context {
    /// Create a new context with the given socket.
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self, ContextError> {
        info!("- Data directory: {:?}", data_dir.as_ref());

        // create sqlite path
        let sqlite_path = format!(
            "file:{}",
            data_dir
                .as_ref()
                .join("cache.sqlite")
                .to_str()
                .expect("invalid path")
        )
        .replace('\\', "/");

        info!("- SQLite path: {:?}", sqlite_path);

        // create settings path
        let settings_path = data_dir.as_ref().join("settings.json");
        info!("- Settings path: {:?}", settings_path);

        Ok(Self {
            socket: StatefulSocket::Inactive.into(),
            cache: cache_prisma::new_client_with_url(&sqlite_path).await?,
            account_ctx: AccountContext::from_data_dir(&data_dir),
            settings_ctx: SettingsContext::from_data_dir(&data_dir).await?,
        })
    }
}
