use std::path::{Path, PathBuf};

use pgp::{
    types::{KeyTrait, SecretKeyTrait},
    SignedSecretKey,
};
use string_comm::{Socket, DEFAULT_PORT};
use string_protocol::ProtocolPacket;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

use crate::{
    account::AccountContext,
    lighthouse::{LighthouseContext, LighthouseError},
    settings::{self, SettingsContext, SettingsError},
};

/// The context type for the router.
#[derive(Debug)]
pub struct Context {
    /// The communication socket.
    pub socket: RwLock<StatefulSocket>,
    /// The context data directory.
    pub data_dir: PathBuf,
    /// List of inbound application channels.
    pub inbound_channels: RwLock<Vec<mpsc::Receiver<ProtocolPacket>>>,
    /// The Prisma client for the cache.
    pub cache: cache_prisma::PrismaClient,
    /// The settings for the application.
    pub settings_ctx: settings::SettingsContext,
    /// The account context.
    pub account_ctx: AccountContext,
    /// The lighthouse context.
    pub lighthouse_ctx: LighthouseContext,
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
    /// Failed to create the Prisma client.
    #[error("failed to create the Prisma client")]
    NewClientError(#[from] cache_prisma::client::NewClientError),
    /// An error occurred while working with the settings context.
    #[error("failed to create the settings context")]
    SettingsContextError(#[from] SettingsError),
    /// An error occurred while working with the Lighthouse context.
    #[error("failed to create the lighthouse context")]
    LighthouseContextError(#[from] LighthouseError),
    /// An error occurred while working with the Prisma client.
    #[error("encountered prisma error")]
    PrismaError(#[from] cache_prisma::client::QueryError),
    /// The socket is already active.
    #[error("socket already active")]
    SocketActive,
    /// Socket error.
    #[error("socket error")]
    SocketError(#[from] string_comm::socket::SocketError),
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

        Ok(Self {
            socket: StatefulSocket::Inactive.into(),
            data_dir: data_dir.as_ref().to_owned(),
            cache: cache_prisma::new_client_with_url(&sqlite_path).await?,
            account_ctx: AccountContext::from_data_dir(&data_dir),
            settings_ctx: SettingsContext::from_data_dir(&data_dir).await?,
            lighthouse_ctx: LighthouseContext::from_data_dir(&data_dir).await?,
            inbound_channels: RwLock::new(Vec::new()),
        })
    }

    /// Setup the socket for the context.
    pub async fn setup_socket(&self, secret_key: SignedSecretKey) -> Result<(), ContextError> {
        // check if socket is active
        let mut socket = self.socket.write().await;
        if matches!(*socket, StatefulSocket::Active(_)) {
            return Err(ContextError::SocketActive);
        }

        // store user fingerprint
        let mut fingerprint = self.account_ctx.fingerprint.write().await;
        *fingerprint = Some(secret_key.public_key().fingerprint());

        // create new socket
        *socket = StatefulSocket::Active(
            Socket::bind(([0, 0, 0, 0], DEFAULT_PORT).into(), secret_key).await?,
        );

        // look for initial peers
        let peers = self.cache.peer().find_many(vec![]).exec().await?;
        info!("Attempting to establish a connection with the following peers:");
        for peer in peers {
            info!("- Peer: {:?}", peer);
            self.lighthouse_ctx.get_node_address(peer.id).await?;
        }

        Ok(())
    }
}
