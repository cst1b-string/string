// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use comm::{Socket, DEFAULT_PORT};
use log::info;
use rspc::{Config, Router};

mod protocol;

/// Context passed to the router during operations.
struct RouterCtx {
    socket: Socket,
}

/// Thread-safe reference to the router context.
type Ctx = Arc<RouterCtx>;

#[tokio::main]
async fn main() {
    // intialize logging
    env_logger::init();
    info!(
        "Starting {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    // rspc router for communicating with frontend
    let router = Router::<Ctx>::new()
        // version query
        .query("version", |t| {
            t(|_: Ctx, _: ()| env!("CARGO_PKG_VERSION").to_string())
        })
        .config(Config::new().export_ts_bindings("../../src/bindings.ts".to_string()))
        .build();

    // bind to socket
    log::info!(
        "Launching socket listener on {}:{}",
        "127.0.0.1",
        DEFAULT_PORT
    );
    let socket = Socket::bind(([127, 0, 0, 1], DEFAULT_PORT).into())
        .await
        .expect("failed to bind socket");

    // create context
    let ctx = Arc::new(RouterCtx { socket });

    // build tauri app
    tauri::Builder::default()
        // rspc plugin - for communicating with frontend
        .plugin(rspc::integrations::tauri::plugin(
            router.into(),
            move || ctx.clone(),
        ))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
