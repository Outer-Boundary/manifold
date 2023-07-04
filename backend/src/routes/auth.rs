use crate::{
    models::login_identity::ClientLoginIdentity,
    types::{error::ErrorResponse, redis::RedisPool},
    util::auth::{
        login::{login_user, logout_user},
        login_identity::verify_login_identity,
        session::get_user_id_from_session,
    },
};
use actix_web::{post, web, HttpResponse};
use sqlx::MySqlPool;

pub fn auth_scope(cfg: &mut web::ServiceConfig) {
    cfg.service(verify_route)
        .service(login_route)
        .service(logout_route);
}

#[tracing::instrument(skip(db_pool, redis, token))]
#[post("/verify")]
async fn verify_route(
    db_pool: web::Data<MySqlPool>,
    redis: web::Data<RedisPool>,
    token: web::Json<String>,
) -> HttpResponse {
    tracing::debug!("Verifying login identity...");

    let result = verify_login_identity(token.into_inner(), &db_pool, &redis).await;

    match result {
        Ok(user_id) => {
            tracing::info!(
                "Successfully verified login identity for user with id '{}'.",
                user_id
            );
            HttpResponse::NoContent().finish()
        }
        Err(err) => {
            tracing::error!("Failed while trying to verify login identity. {}", err);
            HttpResponse::InternalServerError().json(
                ErrorResponse::new(0, "Failed while trying to verify login identity")
                    .description(err),
            )
        }
    }
}

#[tracing::instrument(skip(login_identity, db_pool, session))]
#[post("/login")]
async fn login_route(
    login_identity: web::Json<ClientLoginIdentity>,
    db_pool: web::Data<MySqlPool>,
    session: actix_session::Session,
) -> HttpResponse {
    tracing::debug!("Logging in user...");

    let login_result = login_user(login_identity.into_inner(), &db_pool, &session).await;

    match login_result {
        Ok((Some(user_id), true)) => {
            tracing::info!("Successfully logged in user with id '{}'.", user_id);
            HttpResponse::NoContent().finish()
        }
        Ok((user_id, false)) => {
            if let Some(user_id) = user_id {
                tracing::warn!("Failed login attempt for user with id '{}'.", user_id);
            } else {
                tracing::warn!("Failed login attempt.");
            }

            HttpResponse::Unauthorized().finish()
        }
        Err(err) => {
            tracing::error!("Failed while trying to login user. {}", err);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::new(0, "Failed while trying to login user").description(err))
        }
        // The below arm should not be possible. We should only return true if we are able to find a matching user
        // for the login identity. This means that we will always have the user id to return if the login is successful.
        Ok((None, true)) => {
            tracing::error!("Failed while trying to login user. Unexpected value returned.");
            HttpResponse::InternalServerError().json(
                ErrorResponse::new(0, "Failed while trying to login user")
                    .description("Unexpected value returned"),
            )
        }
    }
}

#[tracing::instrument(skip(session))]
#[post("/logout")]
async fn logout_route(session: actix_session::Session) -> HttpResponse {
    tracing::debug!("Logging out user...");

    let user_id = get_user_id_from_session(&session);

    let result = logout_user(&session);

    match result {
        Ok(_) => {
            if let Ok(Some(user_id)) = user_id {
                tracing::info!("Successfully logged out user with id '{}'.", user_id)
            };
            HttpResponse::NoContent().finish()
        }
        Err(err) => {
            tracing::error!("Failed while trying to logout user. {}", err);
            HttpResponse::InternalServerError()
                .json(ErrorResponse::new(0, "Failed while trying to logout user").description(err))
        }
    }
}
