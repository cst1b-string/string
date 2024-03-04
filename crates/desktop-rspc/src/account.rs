use std::{
    fs::File,
    path::{Path, PathBuf},
};

use pgp::{Deserializable, SignedSecretKey};
use rspc::{RouterBuilder, Type};
use serde::Deserialize;
use string_comm::Socket;

use crate::{context::StatefulSocket, Ctx};

/// The account context.
pub struct AccountContext {
    /// The directory where the keys are stored.
    key_dir: PathBuf,
}

impl AccountContext {
    /// Create a new account context with the given data path.
    pub fn from_data_dir<P: AsRef<Path>>(path: P) -> Self {
        Self {
            key_dir: path.as_ref().join("keys"),
        }
    }
}

/// Attach the channel cache queries to the router.
pub fn attach_crypto_queries<TMeta: Send>(
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
    ///
    username: String,
    passphrase: String,
}

/// Test if the user has a private key.
fn create_account(ctx: Ctx, args: CreateAccountArgs) -> Result<bool, rspc::Error> {
    Ok(false)
}
