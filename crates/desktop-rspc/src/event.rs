use futures::Stream;
use rspc::{RouterBuilder, Type};
use serde::{Deserialize, Serialize};
use string_comm::maybe_break;
use string_protocol::packet::v1::packet::PacketType;

use crate::Ctx;

#[derive(Serialize, Deserialize, Type)]
pub enum Event {
    Tick,
    NotConnected,
    MessageReceived {
        author: String,
        channel_id: String,
        content: String,
    },
}

/// Attach the message cache queries to the router.
pub fn attach_event_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder.subscription("event", |t| t(event_stream))
}

/// Create a stream of events.
fn event_stream(ctx: Ctx, _: ()) -> impl Stream<Item = Event> {
    async_stream::stream! {
        let mut unified_inbound_rx = ctx.inbound_app_rx.write().await;
        let unified_inbound_rx = match *unified_inbound_rx {
            Some(ref mut rx) => rx,
            None => {
                yield Event::NotConnected;
                return;
            }
        };
        loop {
            let (_, packet) = match unified_inbound_rx.recv().await {
                Some(packet) => packet,
                None => break,
            };

            // match on packet type
            let packet_type = maybe_break!(packet.packet_type);
            let message = match packet_type {
                PacketType::PktMessage(message) => message,
                _ => unreachable!("unexpected packet type - yichen lied")
            };

            yield Event::MessageReceived {
                author: message.username,
                channel_id: message.channel_id,
                content: message.content,
            };
        }
    }
}
