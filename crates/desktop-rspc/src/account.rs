use std::path::{Path, PathBuf};

use rspc::{RouterBuilder, Type};
use serde::Deserialize;

use crate::Ctx;

/// The account context.
pub struct AccountContext {
    /// The directory where the keys are stored.
    key_dir: PathBuf,
    /// TODO: not particularly secure
    cached_password: Option<String>,
}

impl AccountContext {
    /// Create a new account context with the given data path.
    pub fn from_data_dir<P: AsRef<Path>>(path: P) -> Self {
        Self {
            key_dir: path.as_ref().join("keys"),
            cached_password: None,
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

/// Test if the user has a private key.
fn login_account(ctx: Ctx, passphrase: String) -> Result<bool, rspc::Error> {
    Ok(false)
}

#[derive(Debug, Type, Deserialize)]
struct CreateAccountArgs {
    username: String,
    passphrase: String,
}

/// Test if the user has a private key.
fn create_account(ctx: Ctx, args: CreateAccountArgs) -> Result<bool, rspc::Error> {
    Ok(false)
}
