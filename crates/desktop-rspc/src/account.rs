use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use pgp::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::CompressionAlgorithm,
    Deserializable, KeyType, SecretKeyParamsBuilder, SignedSecretKey,
};
use rspc::{RouterBuilder, Type};
use serde::Deserialize;
use smallvec::smallvec;
use tokio::sync::RwLock;
use tracing::info;

use crate::{context::ContextError, Ctx};

/// The account context.
#[derive(Debug)]
pub struct AccountContext {
    /// The directory where the keys are stored.
    pub key_dir: PathBuf,
    /// The fingerprint of the active user.
    pub fingerprint: RwLock<Option<Vec<u8>>>,
}

impl AccountContext {
    /// Create a new account context with the given data path.
    pub fn from_data_dir<P: AsRef<Path>>(path: P) -> Self {
        Self {
            key_dir: path.as_ref().join("keys"),
            fingerprint: None.into(),
        }
    }
}

/// Attach the channel cache queries to the router.
pub fn attach_account_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
        .query("account.fingerprint", |t| t(get_fingerprint))
        .mutation("account.login", |t| t(login_account))
        .mutation("account.create", |t| t(create_account))
}

#[derive(Debug, Type, Deserialize)]
struct LoginArgs {
    username: String,
}

/// Test if the user has a private key.
#[tracing::instrument]
async fn login_account(ctx: Ctx, args: LoginArgs) -> Result<(), rspc::Error> {
    // find keys in key dir, abort if missing
    let key_path = ctx
        .account_ctx
        .key_dir
        .join(format!("{}.asc", args.username));
    if !key_path.exists() {
        return Err(rspc::Error::new(
            rspc::ErrorCode::NotFound,
            "No private key found".to_string(),
        ));
    }

    // open file
    let mut key_file = File::open(key_path).map_err(|e| {
        rspc::Error::with_cause(
            rspc::ErrorCode::InternalServerError,
            "Cannot access key file".to_string(),
            e,
        )
    })?;

    // load, prepare socket
    let (secret_key, _) = SignedSecretKey::from_armor_single(&mut key_file).map_err(|err| {
        rspc::Error::with_cause(
            rspc::ErrorCode::InternalServerError,
            "Failed to read key file".to_string(),
            err,
        )
    })?;

    ctx.setup_socket(args.username, secret_key)
        .await
        .map_err(|err| {
            rspc::Error::with_cause(
                rspc::ErrorCode::InternalServerError,
                match &err {
                    ContextError::NewClientError(_) => "failed to create prisma client",
                    ContextError::SettingsContextError(_) => "failed to initialise settings",
                    ContextError::LighthouseContextError(_) => "failed to initialise lighthouse",
                    ContextError::PrismaError(_) => "encountered prisma query error",
                    ContextError::SocketActive => "socket already active",
                    ContextError::SocketError(_) => "error setting up socket",
                }
                .to_string(),
                err,
            )
        })?;

    Ok(())
}

#[derive(Debug, Type, Deserialize)]
struct CreateAccountArgs {
    /// The username of the account.
    username: String,
    /// The passphrase for the key.
    passphrase: String,
}

/// Test if the user has a private key.
#[tracing::instrument]
async fn create_account(ctx: Ctx, args: CreateAccountArgs) -> Result<(), rspc::Error> {
    info!("Creating user account...");

    // check for existing key
    let key_path = ctx
        .account_ctx
        .key_dir
        .join(format!("{}.asc", args.username));
    if key_path.exists() {
        return Err(rspc::Error::new(
            rspc::ErrorCode::Conflict,
            "Local user already exists".to_string(),
        ));
    }

    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
        .key_type(KeyType::Rsa(2048))
        .can_certify(false)
        .can_sign(true)
        .primary_user_id(args.username.clone())
        .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256])
        .preferred_hash_algorithms(smallvec![HashAlgorithm::SHA2_256])
        .preferred_compression_algorithms(smallvec![CompressionAlgorithm::ZLIB]);

    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");
    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");
    let passwd_fn = || args.passphrase;

    let secret_key = secret_key
        .sign(passwd_fn)
        .expect("must be able to sign its own metadata");

    // write key to file
    let mut file = File::create(key_path).expect("Error opening privkey file");
    file.write_all(
        secret_key
            .to_armored_string(None)
            .expect("Error generating armored string")
            .as_bytes(),
    )
    .map_err(|e| {
        rspc::Error::with_cause(
            rspc::ErrorCode::InternalServerError,
            "Error writing key to file".to_string(),
            e,
        )
    })?;

    ctx.setup_socket(args.username, secret_key)
        .await
        .map_err(|err| {
            rspc::Error::with_cause(
                rspc::ErrorCode::InternalServerError,
                "failed to set up socket".to_string(),
                err,
            )
        })?;

    Ok(())
}

/// Get the fingerprint of the active user.
async fn get_fingerprint(ctx: Ctx, _: ()) -> Result<String, rspc::Error> {
    match *ctx.account_ctx.fingerprint.read().await {
        Some(ref fingerprint) => Ok(format!("{:x?}", fingerprint)),
        None => Err(rspc::Error::new(
            rspc::ErrorCode::NotFound,
            "No active user".to_string(),
        )),
    }
}
