use crate::Ctx;
use rspc::{ErrorCode, RouterBuilder, Type};
use serde::Deserialize;

/// Attach the user cache queries to the router.
pub fn attach_user_queries<TMeta: Send>(
    builder: RouterBuilder<Ctx, TMeta>,
) -> RouterBuilder<Ctx, TMeta> {
    builder
        .query("user.list", |t| t(list_users))
        .query("user.user", |t| t(get_user))
        .mutate("user.update_user_details", |t| t(update_user_details))
}

/// Fetch a list of users from the cache.
pub async fn get_user(
    ctx: Ctx,
    user_id: Vec<u8>,
) -> Result<Option<cache_prisma::user::Data>, rspc::Error> {
    ctx.cache
        .user()
        .find_unique(vec![cache_prisma::user::id::equals(user_id)])
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

#[derive(Debug, Type, Deserialize)]
pub struct UpdateUserDetails {
    username: String,
    biography: String,
    user_id: Vec<u8>,
}

/// Update user_detils
pub async fn list_users(
    ctx: Ctx,
    args: UpdateUserDetails,
) -> Result<Vec<cache_prisma::user::Data>, rspc::Error> {
    ctx.cache
        .user()
        .update(
            cache_prisma::user::id::equals(args.user_id),
            vec![
                cache_prisma::user::username::equals(args.username),
                cache_prisma::user::biography::equals(args.biography),
            ],
        )
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
