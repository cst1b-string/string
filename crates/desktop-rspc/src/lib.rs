//! Defines the RSPC router for the desktop application.

mod channel;
mod message;
mod settings;
mod user;

use std::{path::Path, sync::Arc};

use channel::attach_channel_queries;
use message::attach_message_queries;
use rspc::{Config, Router};
use settings::{attach_settings_queries, SettingsContext};
use string_comm::Socket;
use thiserror::Error;
use tracing::info;
use user::attach_user_queries;

/// The context type for the router.
pub struct Context {
    /// The communication socket.
    pub socket: Socket,
    /// The Prisma client for the cache.
    pub cache: cache_prisma::PrismaClient,
    /// The settings for the application.
    pub settings_ctx: settings::SettingsContext,
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
    pub async fn from<P: AsRef<Path>>(socket: Socket, data_dir: P) -> Result<Self, ContextError> {
        info!("- Data directory: {:?}", data_dir.as_ref());

        // create sqlite path
        let sqlite_path = format!(
            "file://{}",
            data_dir
                .as_ref()
                .join("cache.sqlite")
                .to_str()
                .expect("invalid path")
        );
        info!("- SQLite path: {:?}", sqlite_path);

        // create settings path
        let settings_path = data_dir.as_ref().join("settings.json");
        info!("- Settings path: {:?}", settings_path);

        Ok(Self {
            socket,
            cache: cache_prisma::new_client_with_url(&sqlite_path).await?,
            settings_ctx: SettingsContext::from_path(settings_path).await?,
        })
    }
}

/// Thread-safe reference to the context.
pub type Ctx = Arc<Context>;

/// Build a router without exporting any bindings.
pub fn build_router() -> Router<Ctx> {
    build_router_with::<String>(None)
}

/// Build a router with the given bindings file.
pub fn build_router_with_bindings<P: AsRef<Path>>(bindings: P) -> Router<Ctx> {
    build_router_with(Some(bindings))
}

/// Internal function to build a router with optional bindings.
fn build_router_with<P: AsRef<Path>>(bindings: Option<P>) -> Router<Ctx> {
    let config = match bindings {
        Some(path) => Config::new().export_ts_bindings(path.as_ref()),
        None => Config::new(),
    };
    let builder = Router::<Ctx>::new().config(config);

    // attach queries
    let builder = attach_settings_queries(builder);
    let builder = attach_message_queries(builder);
    let builder = attach_channel_queries(builder);
    let builder = attach_user_queries(builder);

    builder.build()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rspc_router() {
        super::build_router();
    }
}
