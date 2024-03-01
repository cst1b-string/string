use rspc::{RouterBuilder, Type};
use serde::Deserialize;

use crate::Ctx;

/// Attach the channel cache queries to the router.
pub fn attach_crypto_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
        .query("crypto.loadKey", |t| t(load_private_key))
        .mutation("crypto.generateKey", |t| t(generate_private_key))
}

/// Test if the user has a private key.
fn load_private_key(ctx: Ctx, passphrase: String) -> Result<bool, rspc::Error> {
    Ok(false)
}

#[derive(Debug, Type, Deserialize)]
struct GenerateKeyArgs {
    username: String,
    passphrase: String,
}

/// Test if the user has a private key.
fn generate_private_key(ctx: Ctx, args: GenerateKeyArgs) -> Result<bool, rspc::Error> {
    Ok(false)
}
