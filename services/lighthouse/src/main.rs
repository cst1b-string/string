use std::{
    borrow::Cow,
    net::{AddrParseError, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::{
    error_handling::HandleErrorLayer,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    BoxError, Extension, Json, Router,
};
use axum_macros::debug_handler;
use lighthouse_prisma::PrismaClient;
use lighthouse_protocol::{
    GetNodeAddrPayload, GetNodeAddrResponse, ListPotentialPeersPayload, ListPotentialPeersResponse,
    RegisterNodeAddrPayload, RegisterNodeAddrResponse, Sign,
};

use serde::Serialize;
use thiserror::Error;
use tokio::{net::TcpListener, sync::RwLock};
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

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
async fn wipe_node_entries(
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
) -> Result<(), LighthouseError> {
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

/// This endpoint handles the registration of a new endpoint.
#[debug_handler]
async fn register_node_addr(
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
    Json(payload): Json<RegisterNodeAddrPayload>,
) -> Result<Response, LighthouseError> {
    let db = ctx.db.write().await;

    // verify the payload
    payload.verify()?;

    // verify_data(
    //     &payload.public_key,
    //     &payload.signature,
    //     &payload.addr,
    //     payload.timestamp,
    // )?;

    let existing_rec = db
        .endpoint()
        .find_first(vec![
            lighthouse_prisma::endpoint::ip::equals(payload.addr.ip().to_string()),
            lighthouse_prisma::endpoint::port::equals(payload.addr.port().into()),
        ])
        .exec()
        .await?;

    if matches!(existing_rec, Some(_)) {
        return Ok(Json(RegisterNodeAddrResponse {}).into_response());
    }

    // create an endpoint record
    let endpoint = db
        .endpoint()
        .create(
            payload.addr.ip().to_string(),
            payload.addr.port().into(),
            chrono::Utc::now().fixed_offset(),
            vec![],
        )
        .exec()
        .await?;

    // store the pubkey
    db.pubkey()
        .create(
            lighthouse_prisma::endpoint::id::equals(endpoint.id.clone()),
            payload.public_key,
            vec![],
        )
        .exec()
        .await?;

    Ok(Json(RegisterNodeAddrResponse {}).into_response())
}

/// This endpoint handles the lookup of a endpoint.
#[debug_handler]
async fn get_node_addr(
    Path(fingerprint): Path<String>,
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
    Json(payload): Json<GetNodeAddrPayload>,
) -> Result<Response, LighthouseError> {
    let db = ctx.db.write().await;

    // find the endpoint
    let endpoint = db
        .endpoint()
        .find_first(vec![lighthouse_prisma::endpoint::id::equals(
            fingerprint.clone(),
        )])
        .exec()
        .await?
        .ok_or(LighthouseError::InvalidId)?;

    // store an entry in the pending connections table
    db.pending_connection()
        .create(
            lighthouse_prisma::endpoint::id::equals(fingerprint.clone()),
            payload.addr.ip().to_string(),
            payload.addr.port().into(),
            hex::decode(&fingerprint)?,
            vec![],
        )
        .exec()
        .await?;

    Ok(Json(GetNodeAddrResponse {
        addr: SocketAddr::new(
            endpoint
                .ip
                .parse()
                .expect("stored information was not an IP address"),
            endpoint
                .port
                .try_into()
                .expect("stored information was not a port"),
        ),
    })
    .into_response())
}

/// This endpoint handles the listing of all pending connections
#[debug_handler]
async fn list_potential_peers(
    Extension(ctx): Extension<Arc<LighthouseCtx>>,
    Json(payload): Json<ListPotentialPeersPayload>,
) -> Result<Response, LighthouseError> {
    let db = ctx.db.write().await;

    // verify the payload
    payload.verify()?;

    let conns = db
        .pending_connection()
        .find_many(vec![
            lighthouse_prisma::pending_connection::endpoint_id::equals(payload.fingerprint.clone()),
        ])
        .exec()
        .await?;

    Ok(Json(ListPotentialPeersResponse {
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
    let ctx = Arc::new(ctx);

    // create app router
    let app = Router::new()
        .route("/", get(report_status))
        .route("/nodes", post(register_node_addr))
        .route("/nodes/:fingerprint", get(get_node_addr))
        .route("/peers", get(list_potential_peers))
        .route("/nodes", delete(wipe_node_entries)) // Testing purposes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(ctx.clone()))
                .into_inner(),
        );

    // create listener and serve
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind listener");

    start_db_cleanup_worker(ctx);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("fatal error while serving requests");
}
