//! Defines the RSPC router for the desktop application.

mod message;
mod settings;

use std::{path::Path, sync::Arc};

use message::attach_message_queries;
use rspc::{Config, Router};
use settings::{attach_settings_queries, SettingsContext};
use string_comm::Socket;
use thiserror::Error;

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
    pub async fn from_socket(socket: Socket) -> Result<Self, ContextError> {
        Ok(Self {
            socket,
            cache: cache_prisma::new_client().await?,
            settings_ctx: SettingsContext::from_path("./settings.json").await?,
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

    builder.build()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rspc_router() {
        super::build_router();
    }
}
