//! Defines the RSPC router for the desktop application.

mod account;
mod channel;
mod context;
mod event;
mod settings;
mod user;

use std::{path::Path, sync::Arc};

use account::attach_account_queries;
use channel::attach_channel_queries;
pub use context::Context;
use event::attach_event_queries;
use rspc::{Config, Router};
use settings::attach_settings_queries;
use user::attach_user_queries;

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
    let builder = attach_account_queries(builder);
    let builder = attach_channel_queries(builder);
    let builder = attach_event_queries(builder);
    let builder = attach_settings_queries(builder);
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
