//! Defines queries that interact with the local SQLite message cache.

use cache_prisma::client::chrono::{TimeZone, Utc};
use rspc::{ErrorCode, RouterBuilder, Type};
use serde::Deserialize;

use crate::Ctx;

/// Attach the message cache queries to the router.
pub fn attach_message_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
        .query("message.list", |t| t(list_messages))
        .mutation("message.send", |t| t(send_message))
}

/// Arguments for the list messages query.
#[derive(Debug, Deserialize, Type)]
pub struct ListMessageArgs {
    /// Return messages after this timestamp.
    pub after: Option<i32>,
}

/// Fetch a list of messages from the message cache.
pub async fn list_messages(
    ctx: Ctx,
    input: ListMessageArgs,
) -> Result<Vec<cache_prisma::message::Data>, rspc::Error> {
    ctx.cache
        .message()
        .find_many(vec![cache_prisma::message::timestamp::gt(
            Utc.timestamp_opt(input.after.unwrap_or(0) as i64, 0)
                .unwrap()
                .into(),
        )])
        .take(100)
        .exec()
        .await
        .map_err(|err| {
            rspc::Error::with_cause(
                ErrorCode::InternalServerError,
                "failed to fetch from cache".into(),
                err,
            )
        })
}

/// Send a message to the network.
async fn send_message(ctx: Ctx, message: String) -> Result<(), rspc::Error> {
    // TODO: send the message
    Ok(())
}
