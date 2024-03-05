use rspc::{ErrorCode, RouterBuilder, Type};
use serde::Deserialize;

use crate::{context::StatefulSocket, Ctx};

/// Attach the channel cache queries to the router.
pub fn attach_channel_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
        .query("channel.list", |t| t(list_channels))
        .query("channel.messages", |t| t(get_channel_messages))
        .mutation("channel.create", |t| t(create_channel))
        .mutation("channel.send", |t| t(send_message))
}

/// Fetch a list of channels from the cache.
pub async fn list_channels(
    ctx: Ctx,
    _: (),
) -> Result<Vec<cache_prisma::channel::Data>, rspc::Error> {
    ctx.cache
        .channel()
        .find_many(vec![])
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

/// Get messages from a channel.
pub async fn get_channel_messages(
    ctx: Ctx,
    channel_id: i32,
) -> Result<Vec<cache_prisma::message::Data>, rspc::Error> {
    ctx.cache
        .message()
        .find_many(vec![cache_prisma::message::channel_id::equals(channel_id)])
        .order_by(cache_prisma::message::timestamp::order(
            cache_prisma::client::Direction::Asc,
        ))
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

#[derive(Debug, Type, Deserialize)]
pub struct CreateChannelArgs {
    title: String,
    network_id: i32,
}

pub async fn create_channel(
    ctx: Ctx,
    CreateChannelArgs { title, network_id }: CreateChannelArgs,
) -> Result<cache_prisma::channel::Data, rspc::Error> {
    ctx.cache
        .channel()
        .create(title, cache_prisma::network::id::equals(network_id), vec![])
        .exec()
        .await
        .map_err(|err| {
            rspc::Error::with_cause(
                ErrorCode::InternalServerError,
                "failed to create channel".into(),
                err,
            )
        })
}

/// Send a message to the network.
#[derive(Debug, Type, Deserialize)]
pub struct SendMessageArgs {
    channel_id: i32,
    content: String,
}

/// Send a message to the network.
async fn send_message(ctx: Ctx, args: SendMessageArgs) -> Result<(), rspc::Error> {
    let fingerprint = {
        let fingerprint = ctx.account_ctx.fingerprint.read().await;
        match fingerprint.as_ref() {
            None => {
                return Err(rspc::Error::new(
                    ErrorCode::Unauthorized,
                    "not logged in".to_string(),
                ));
            }
            Some(fingerprint) => fingerprint,
        }
        .clone()
    };

    // TODO: send on socket
    let socket = ctx.socket.read().await;
    let socket = match *socket {
        StatefulSocket::Active(ref socket) => socket,
        StatefulSocket::Inactive => {
            return Err(rspc::Error::new(
                ErrorCode::Unauthorized,
                "not logged in".to_string(),
            ));
        }
    };

    // push message to cache - maybe wait for response from socket?
    ctx.cache
        .message()
        .create(
            args.content,
            cache_prisma::user::id::equals(fingerprint.clone()),
            cache_prisma::channel::id::equals(args.channel_id),
            vec![],
        )
        .exec()
        .await
        .map_err(|err| {
            rspc::Error::with_cause(
                ErrorCode::InternalServerError,
                "failed to send message".into(),
                err,
            )
        })?;
    Ok(())
}
