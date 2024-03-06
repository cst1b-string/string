use rspc::{ErrorCode, RouterBuilder};

use crate::Ctx;

/// Attach the user cache queries to the router.
pub fn attach_user_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder.query("user.list", |t| t(list_users))
}

/// Fetch a list of users from the cache.
pub async fn list_users(ctx: Ctx, _: ()) -> Result<Vec<cache_prisma::user::Data>, rspc::Error> {
    ctx.cache
        .user()
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
