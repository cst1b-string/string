use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use pgp::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::{CompressionAlgorithm, KeyTrait, SecretKeyTrait},
    Deserializable, KeyType, SecretKeyParamsBuilder, SignedSecretKey,
};
use rspc::{RouterBuilder, Type};
use serde::Deserialize;
use smallvec::smallvec;
use string_comm::Socket;
use tokio::sync::Mutex;

use crate::{context::StatefulSocket, Ctx};

/// The account context.
pub struct AccountContext {
    /// The directory where the keys are stored.
    key_dir: PathBuf,
    /// The fingerprint of the active user.
    fingerprint: Mutex<Option<Vec<u8>>>,
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
        .query("account.login", |t| t(login_account))
        .mutation("account.create", |t| t(create_account))
}

#[derive(Debug, Type, Deserialize)]
struct LoginArgs;

/// Test if the user has a private key.
async fn login_account(ctx: Ctx, _: LoginArgs) -> Result<bool, rspc::Error> {
    // find keys in key dir, abort if missing
    let key_path = ctx.account_ctx.key_dir.join("private.key");
    if !key_path.exists() {
        return Ok(false);
    }

    // open file
    let mut key_file = File::open(key_path).map_err(|e| {
        rspc::Error::with_cause(
            rspc::ErrorCode::InternalServerError,
            "missing".to_string(),
            e,
        )
    })?;

    // load, prepare socket
    let (secret_key, _) = SignedSecretKey::from_armor_single(&mut key_file).map_err(|err| {
        rspc::Error::with_cause(
            rspc::ErrorCode::InternalServerError,
            "failed to read key".to_string(),
            err,
        )
    })?;

    // check if socket is active
    let mut socket = ctx.socket.lock().await;
    if matches!(*socket, StatefulSocket::Active(_)) {
        return Ok(true);
    }

    // store user fingerprint
    let mut fingerprint = ctx.account_ctx.fingerprint.lock().await;
    *fingerprint = Some(secret_key.public_key().fingerprint());

    // create new socket
    *socket = StatefulSocket::Active(
        Socket::bind(([0, 0, 0, 0], 40000).into(), secret_key)
            .await
            .map_err(|err| {
                rspc::Error::with_cause(
                    rspc::ErrorCode::InternalServerError,
                    "failed to start socket server".to_string(),
                    err,
                )
            })?,
    );

    Ok(false)
}

#[derive(Debug, Type, Deserialize)]
struct CreateAccountArgs {
    /// The username of the account.
    username: String,
    /// The passphrase for the key.
    passphrase: String,
}

/// Test if the user has a private key.
async fn create_account(ctx: Ctx, args: CreateAccountArgs) -> Result<bool, rspc::Error> {
    let key_path = ctx.account_ctx.key_dir.join("private.key");
    if !key_path.exists() {
        return Ok(false);
    }

    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
        .key_type(KeyType::Rsa(2048))
        .can_certify(false)
        .can_sign(true)
        .primary_user_id(args.username)
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
    .expect("Error writing privkey");

    // check if socket is active
    let mut socket = ctx.socket.lock().await;
    if matches!(*socket, StatefulSocket::Active(_)) {
        return Ok(true);
    }

    // store user fingerprint
    let mut fingerprint = ctx.account_ctx.fingerprint.lock().await;
    *fingerprint = Some(secret_key.public_key().fingerprint());

    // create new socket
    *socket = StatefulSocket::Active(
        Socket::bind(([0, 0, 0, 0], 40000).into(), secret_key)
            .await
            .map_err(|err| {
                rspc::Error::with_cause(
                    rspc::ErrorCode::InternalServerError,
                    "failed to start socket server".to_string(),
                    err,
                )
            })?,
    );

    Ok(false)
}
