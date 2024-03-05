use std::{
    borrow::Cow,
    net::{AddrParseError, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    BoxError, Extension, Json, Router,
};
use axum_macros::debug_handler;
use lighthouse_prisma::PrismaClient;
use pgp::{composed::SignedPublicKey, Deserializable};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

use string_comm::crypto::Crypto;

/// Defines the available error types.
#[allow(dead_code)]
#[derive(Error, Debug)]
enum LighthouseError {
    #[error("unknown error")]
    Unknown,
    #[error("failed to decode channel key")]
    KeyDecodeError(#[from] base64::DecodeError),
    #[error("failed to query database")]
    QueryError(#[from] lighthouse_prisma::client::queries::QueryError),
    #[error("failed to parse endpoint ip:port pair")]
    EndpointParseError(#[from] AddrParseError),
    #[error("failed to use public key")]
    PubKeyError(#[from] pgp::errors::Error),
    #[error("failed to verify signature")]
    SignatureError,
    #[error("no such ID")]
    InvalidId,
    #[error("invalid fingerprint")]
    InvalidFingerprint(#[from] hex::FromHexError),
}

#[derive(Serialize)]
struct ServerError {
    sorry: String,
    error: String,
}

impl IntoResponse for LighthouseError {
    fn into_response(self) -> Response {
        let mut resp = Json(ServerError {
            sorry: "oopsies :3".to_string(),
            error: format!("{:?}", self), // self.to_string()
        })
        .into_response();
        *resp.status_mut() = StatusCode::BAD_REQUEST;
        resp
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

/// This (debug) endpoint wipes the database so we don't accumulate stuff across tests
#[debug_handler]
async fn wipe_all(Extension(ctx): Extension<Arc<LighthouseCtx>>) -> Result<(), LighthouseError> {
    let db = ctx.db.write().await;

    db.pubkey().delete_many(vec![]).exec().await?;
    db.pending_connection().delete_many(vec![]).exec().await?;
    db.endpoint().delete_many(vec![]).exec().await?;

    let sql_cmd = "ALTER SEQUENCE \"Pubkey_id_seq\" RESTART WITH 1";
    db._execute_raw(prisma_client_rust::Raw::new(sql_cmd, vec![]))
        .exec()
        .await?;
    let sql_cmd1 = "ALTER SEQUENCE \"PendingConnection_id_seq\" RESTART WITH 1";
    db._execute_raw(prisma_client_rust::Raw::new(sql_cmd1, vec![]))
        .exec()
        .await?;

    Ok(())
}

fn verify_data(
    pubkey_str: &str,
    signature_str: &String,
    input: &String,
    timestamp: u32,
) -> Result<(), LighthouseError> {
    let now: u32 = chrono::Utc::now().timestamp() as u32;

    if timestamp < now - 30 || timestamp > now + 30 {
        return Err(LighthouseError::SignatureError);
    }

    let (pubkey, _headers) = SignedPublicKey::from_string(pubkey_str)?;

    let signature = hex::decode(signature_str).map_err(|_| LighthouseError::SignatureError)?;

    let data = format!("{}-{}", input, timestamp);

    Crypto::verify_data_static(&pubkey, &signature, data.as_bytes())
        .map_err(|_| LighthouseError::SignatureError)?;

    Ok(())
}

#[derive(Deserialize)]
struct RegisterEndpointPayload {
    endpoint: String,
    pubkey: String,
    signature: String,
    timestamp: u32,
}

#[derive(Deserialize)]
struct LookupEndpointPayload {
    id: String,
    client: String,
    fingerprint: String,
}

#[derive(Deserialize)]
struct ListConnPayload {
    id: String,
    signature: String,
    timestamp: u32,
}

#[derive(Serialize)]
struct RegisterEndpointResponse {
    id: String,
}

#[derive(Serialize)]
struct LookupEndpointResponse {
    endpoint: String,
}

#[derive(Serialize)]
struct ListConnResponse {
    conns: Vec<(String, String)>,
}

/// This endpoint handles the registration of a new endpoint.
#[debug_handler]
async fn register_endpoint(
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
    Json(payload): Json<RegisterEndpointPayload>,
) -> Result<Response, LighthouseError> {
    let db = ctx.db.write().await;

    let endpoint = SocketAddr::from_str(&payload.endpoint)?;

    verify_data(
        &payload.pubkey,
        &payload.signature,
        &payload.endpoint,
        payload.timestamp,
    )?;

    let existing_rec = db
        .endpoint()
        .find_first(vec![
            lighthouse_prisma::endpoint::ip::equals(endpoint.ip().to_string()),
            lighthouse_prisma::endpoint::port::equals(endpoint.port().into()),
        ])
        .exec()
        .await?;

    if let Some(rec) = existing_rec {
        return Ok(Json(RegisterEndpointResponse { id: rec.id }).into_response());
    }

    let endpoint_rec = db
        .endpoint()
        .create(
            endpoint.ip().to_string(),
            endpoint.port().into(),
            chrono::Utc::now().fixed_offset(),
            vec![],
        )
        .exec()
        .await?;

    db.pubkey()
        .create(
            lighthouse_prisma::endpoint::id::equals(endpoint_rec.id.clone()),
            payload.pubkey,
            vec![],
        )
        .exec()
        .await?;

    Ok(Json(RegisterEndpointResponse {
        id: endpoint_rec.id,
    })
    .into_response())
}

/// This endpoint handles the lookup of a endpoint.
#[debug_handler]
async fn lookup_endpoint(
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
    Json(payload): Json<LookupEndpointPayload>,
) -> Result<Response, LighthouseError> {
    let db = ctx.db.write().await;

    let client = SocketAddr::from_str(&payload.client)?;

    let endpoint_rec = db
        .endpoint()
        .find_first(vec![lighthouse_prisma::endpoint::id::equals(
            payload.id.clone(),
        )])
        .exec()
        .await?
        .ok_or(LighthouseError::InvalidId)?;

    db.pending_connection()
        .create(
            lighthouse_prisma::endpoint::id::equals(payload.id.clone()),
            client.ip().to_string(),
            client.port().into(),
            hex::decode(payload.fingerprint)?,
            vec![],
        )
        .exec()
        .await?;

    Ok(Json(LookupEndpointResponse {
        endpoint: format!("{}:{}", endpoint_rec.ip, endpoint_rec.port),
    })
    .into_response())
}

/// This endpoint handles the listing of all pending connections
#[debug_handler]
async fn list_conns(
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
    Json(payload): Json<ListConnPayload>,
) -> Result<Response, LighthouseError> {
    let db = ctx.db.write().await;

    let pubkey_str = db
        .pubkey()
        .find_first(vec![lighthouse_prisma::pubkey::endpoint_id::equals(
            payload.id.clone(),
        )])
        .exec()
        .await?
        .ok_or(LighthouseError::InvalidId)?
        .pubkey;

    verify_data(
        &pubkey_str,
        &payload.signature,
        &payload.id,
        payload.timestamp,
    )?;

    let conns = db
        .pending_connection()
        .find_many(vec![
            lighthouse_prisma::pending_connection::endpoint_id::equals(payload.id),
        ])
        .exec()
        .await?;

    Ok(Json(ListConnResponse {
        conns: conns
            .iter()
            .map(|rec| {
                (
                    format!("{}:{}", rec.ip, rec.port),
                    hex::encode(rec.fingerprint.clone()),
                )
            })
            .collect(),
    })
    .into_response())
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

fn start_db_cleanup_worker(ctx: Arc<LighthouseCtx>) {
    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
            let ctx = ctx.db.write().await;
            // TODO: flush old entries
            drop(ctx);
        }
    });
}

#[tokio::main]
async fn main() {
    // initialise tracing
    tracing_subscriber::fmt::init();

    // initialise prisma
    let prisma = lighthouse_prisma::new_client()
        .await
        .expect("failed to create database client");

    let ctx = LighthouseCtx { db: prisma.into() };

    let arc_ctx = Arc::new(ctx);

    // create app router
    let app = Router::new()
        .route("/", get(report_status))
        .route("/register", post(register_endpoint))
        .route("/lookup", post(lookup_endpoint))
        .route("/listconns", post(list_conns))
        .route("/wipe", get(wipe_all)) // Testing purposes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(arc_ctx.clone()))
                .into_inner(),
        );

    // create listener and serve
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind listener");

    start_db_cleanup_worker(arc_ctx);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("fatal error while serving requests");
}
