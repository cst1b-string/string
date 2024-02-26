use std::path::{Path, PathBuf};

use rspc::{Config, Router};

/// The context type for the router.
pub struct Context {}

/// Build a router without exporting any bindings.
pub fn build_router() -> Router<Context> {
    build_router_with::<String>(None)
}

/// Build a router with the given bindings file.
pub fn build_router_with_bindings<P: AsRef<Path>>(bindings: P) -> Router<Context> {
    build_router_with(Some(bindings))
}

/// Internal function to build a router with optional bindings.
fn build_router_with<P: AsRef<Path>>(bindings: Option<P>) -> Router<Context> {
    let config = match bindings {
        Some(path) => Config::new().export_ts_bindings(path.as_ref()),
        None => Config::new(),
    };
    Router::<Context>::new()
        .query("version", |t| t(|ctx, input: ()| env!("CARGO_PKG_VERSION")))
        .config(config)
        .build()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rspc_router() {
        super::build_router();
    }
}
