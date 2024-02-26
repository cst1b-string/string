// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use rspc::{Config, Router};
use string_comm::{Socket, DEFAULT_PORT};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod protocol;
mod user_session;

#[tokio::main]
async fn main() {
    // intialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .pretty()
        .init();
    info!(
        "Starting {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    // rspc router for communicating with frontend
    let router = desktop_rspc::build_router();

    // bind to socket
    info!(
        "Launching socket listener on {}:{}",
        "127.0.0.1", DEFAULT_PORT
    );
    let socket = Socket::bind(([127, 0, 0, 1], DEFAULT_PORT).into())
        .await
        .expect("failed to bind socket");

    // create context
    let ctx = Arc::new(RouterCtx {
        socket,
        user_session,
    });

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
