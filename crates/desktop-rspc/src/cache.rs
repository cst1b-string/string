use rspc::RouterBuilder;

use crate::Ctx;

/// Attach the cache queries to the router.
pub fn attach_cache_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
}
