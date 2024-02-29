use futures::Stream;
use rspc::{RouterBuilder, Type};
use serde::{Deserialize, Serialize};

use crate::Ctx;

#[derive(Serialize, Deserialize, Type)]
pub enum Event {
    Tick,
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
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            yield Event::Tick;
        }
    }
}
