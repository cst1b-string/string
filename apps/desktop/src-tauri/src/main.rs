// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use desktop_rspc::Context;
use string_comm::{Socket, DEFAULT_PORT};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // intialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();
    info!(
        "Starting {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    // build tauri app
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    // rspc router for communicating with frontend
    let router = desktop_rspc::build_router();

    // bind to socket
    info!(
        "Launching socket listener on {}:{}",
        "127.0.0.1", DEFAULT_PORT
    );
    let socket = Socket::bind(([127, 0, 0, 1], DEFAULT_PORT).into(), "desktop".into())
        .await
        .expect("failed to bind socket");

    // get app data dir and create it if it doesn't exist
    let data_dir = app.handle().path_resolver().app_data_dir().unwrap();
    std::fs::create_dir_all(&data_dir).expect("failed to create app data directory");

    // create context
    info!("Creating application context...");
    let ctx = desktop_rspc::Ctx::new(
        Context::from(socket, data_dir)
            .await
            .expect("failed to create context"),
    );

    // add rspc plugin
    app.handle()
        .plugin(rspc::integrations::tauri::plugin(
            router.into(),
            move || ctx.clone(),
        ))
        .expect("failed to add rspc plugin");

    // start the app
    app.run(|_, event| match event {
        _ => {}
    })
}
