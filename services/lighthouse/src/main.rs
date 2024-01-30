use std::{borrow::Cow, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    error_handling::HandleErrorLayer,
    extract::ConnectInfo,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    BoxError, Extension, Json, Router,
};
use base64::prelude::*;
use prisma::PrismaClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

/// Defines the available error types.
#[derive(Error, Debug)]
enum LighthouseError {
    #[error("unknown error")]
    Unknown,
    #[error("failed to decode channel key")]
    KeyDecodeError(#[from] base64::DecodeError),
    #[error("failed to query database")]
    QueryError(#[from] prisma::client::queries::QueryError),
}

impl IntoResponse for LighthouseError {
    fn into_response(self) -> Response {
        todo!()
    }
}

/// Defines the context for requests.
struct LighthouseCtx {
    db: RwLock<PrismaClient>,
}

/// The payload for the `report_status` endpoint. Contains the version of the service.
#[derive(Serialize)]
struct Status {
    version: &'static str,
}

async fn report_status() -> (StatusCode, Json<Status>) {
    (
        StatusCode::OK,
        Json(Status {
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

#[derive(Deserialize)]
struct RegisterEndpointPayload {
    key: String,
}

/// This endpoint handles the registration of a new endpoint.
async fn register_endpoint(
    Json(payload): Json<RegisterEndpointPayload>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
) -> Result<(), LighthouseError> {
    let db = ctx.db.write().await;

    // decode key
    let key = BASE64_STANDARD.decode(payload.key.as_bytes())?;

    // upsert channel entry
    db.channel()
        .upsert(
            prisma::channel::key::equals(key.clone()),
            prisma::channel::create(key.clone(), vec![]),
            vec![],
        )
        .exec()
        .await?;

    // connect endpoint
    db.channel_endpoint().create(
        prisma::channel::key::equals(key.clone()),
        addr.ip().to_string(),
        chrono::Utc::now().fixed_offset(),
        vec![],
    );

    Ok(())
}

/// Handles errors from middleware.
async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("Unhandled internal error: {}", error)),
    )
}

#[tokio::main]
async fn main() {
    // initialise tracing
    tracing_subscriber::fmt::init();

    // initialise prisma
    let prisma = prisma::new_client()
        .await
        .expect("failed to create database client");

    // create context
    let ctx = LighthouseCtx { db: prisma.into() };

    // create app router
    let app = Router::new()
        .route("/", get(report_status))
        .route("/endpoints", post(register_endpoint))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(Arc::new(ctx)))
                .into_inner(),
        );

    // create listener and serve
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind listener");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("fatal error while serving requests");
}
