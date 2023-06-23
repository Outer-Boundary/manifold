use crate::{
    models::{login_identity::NewLoginIdentity, users::*},
    types::{error::ErrorResponse, redis::RedisPool},
    util::{
        email::send_multipart_email,
        login_identity::add_login_identity,
        url::full_uri,
        users::{add_user, delete_user, get_user, get_users},
    },
};
use actix_web::{delete, get, http::header, post, web, HttpRequest, HttpResponse};
use sqlx::MySqlPool;
use uuid::Uuid;

pub fn users_scope(cfg: &mut web::ServiceConfig) {
    cfg.service(get_users_route)
        .service(get_user_route)
        .service(add_user_route)
        .service(delete_user_route);
}

#[tracing::instrument(skip(pool))]
#[get("")]
async fn get_users_route(pool: web::Data<MySqlPool>) -> HttpResponse {
    tracing::debug!("Requesting all users...");

    let users = get_users(&pool).await;

    match users {
        Ok(users) => {
            tracing::info!("Returning list of all users.");
            HttpResponse::Ok().json(users)
        }
        Err(err) => {
            tracing::error!("Failed while trying to get a list of all users. {}", err);
            HttpResponse::InternalServerError().json(
                ErrorResponse::new(0, "Error occurred while trying to list all users")
                    .description(err),
            )
        }
    }
}

#[tracing::instrument(skip(pool))]
#[get("/{id}")]
async fn get_user_route(pool: web::Data<MySqlPool>, id: web::Path<Uuid>) -> HttpResponse {
    let user_id = id.into_inner();

    tracing::debug!("Requesting user with id '{}'...", user_id);

    let user = get_user(user_id, &pool).await;

    match user {
        Ok(Some(user)) => {
            tracing::info!("Found user with id '{}'.", user_id);
            HttpResponse::Ok().json(user)
        }
        Ok(None) => {
            tracing::warn!("No user found with id '{}'.", user_id);
            HttpResponse::NotFound().json(ErrorResponse::new(
                0,
                format!("No user with id '{}'", user_id),
            ))
        }
        Err(err) => {
            tracing::error!(
                "Failed while trying to find user with id '{}'. {}",
                user_id,
                err
            );
            HttpResponse::InternalServerError().json(
                ErrorResponse::new(
                    0,
                    format!(
                        "Error occurred while trying to get user with id '{}'",
                        user_id
                    ),
                )
                .description(err),
            )
        }
    }
}

#[tracing::instrument(skip(pool, redis, request))]
#[post("")]
async fn add_user_route(
    pool: web::Data<MySqlPool>,
    redis: web::Data<RedisPool>,
    request: HttpRequest,
    new_user: web::Json<NewUser>,
) -> HttpResponse {
    tracing::debug!("Creating new user...");

    // Create the user
    let result = add_user(new_user.clone(), &pool).await;

    match result {
        Ok(user_id) => {
            // Get the user's details
            let user = get_user(user_id, &pool).await;

            match user {
                Ok(Some(user)) => {
                    // Create their login identity
                    let result =
                        add_login_identity(user_id, new_user.clone().identity, &pool).await;

                    match result {
                        Ok(_) => {
                            let redis_conn = redis.get().await;

                            match redis_conn {
                                Ok(mut redis_conn) => match new_user.clone().identity {
                                    NewLoginIdentity::EmailPassword(li) => {
                                        let result = send_multipart_email(
                                            "Manifold Account Verification".to_string(),
                                            user_id,
                                            li.email,
                                            user.username.clone(),
                                            "verification_email.html",
                                            &mut redis_conn,
                                        )
                                        .await;

                                        match result {
                                            Ok(_) => {
                                                tracing::info!(
                                                    "Created new user with id '{}'.",
                                                    user_id
                                                );
                                                HttpResponse::Created()
                                                    .append_header((
                                                        header::LOCATION,
                                                        format!(
                                                            "{}/{}",
                                                            full_uri(&request),
                                                            user_id
                                                        ),
                                                    ))
                                                    .json(user)
                                            }
                                            Err(err) => {
                                                tracing::error!(
                                                        "Error occurred while trying to send verification email to user with id '{}'. {}",
                                                        user_id,
                                                        err
                                                    );
                                                HttpResponse::InternalServerError().json(
                                                        ErrorResponse::new(
                                                            0,
                                                            format!(
                                                                "Error occurred while trying to send verification email to user with id '{}'",
                                                                user_id
                                                            ),
                                                        )
                                                        .description(err),
                                                    )
                                            }
                                        }
                                    }
                                },
                                Err(err) => {
                                    tracing::error!(
                                        "Error occurred while trying to get a connection from the redis pool. {}",
                                        err
                                    );
                                    HttpResponse::InternalServerError().json(
                                        ErrorResponse::new(
                                            0,
                                            "Error occurred while trying to get a connection from the redis pool",
                                        )
                                        .description(err),
                                    )
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!(
                                "Error occurred while trying to add login identity for newly created user with id '{}'. {}",
                                user_id,
                                err
                            );

                            // Try to delete the created user if the server failed to also create their login identity.
                            let _ = delete_user(user_id, &pool).await;

                            HttpResponse::InternalServerError().json(
                                ErrorResponse::new(
                                    0,
                                    format!(
                                        "Error occurred while trying to add login identity for newly created user with id '{}'",
                                        user_id
                                    ),
                                )
                                .description(err),
                            )
                        }
                    }
                }
                Ok(None) => {
                    tracing::error!("Could not find newly created user with id '{}'.", user_id);
                    HttpResponse::InternalServerError().json(ErrorResponse::new(
                        0,
                        format!("Could not find newly created user with id '{}'", user_id),
                    ))
                }
                Err(err) => {
                    tracing::error!(
                        "Error occurred while trying to get newly created user with id '{}'. {}",
                        user_id,
                        err
                    );
                    HttpResponse::InternalServerError().json(
                        ErrorResponse::new(
                            0,
                            format!(
                            "Error occurred while trying to get newly created user with id '{}'",
                            user_id
                        ),
                        )
                        .description(err),
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!("Failed while trying to create new user. {}", err);
            HttpResponse::InternalServerError().json(
                ErrorResponse::new(0, "Error occurred while trying to create new user")
                    .description(err),
            )
        }
    }
}

#[tracing::instrument(skip(pool))]
#[delete("/{id}")]
async fn delete_user_route(pool: web::Data<MySqlPool>, id: web::Path<Uuid>) -> HttpResponse {
    let user_id = id.into_inner();

    tracing::debug!("Deleting user with id '{}'...", user_id);

    let user = get_user(user_id, &pool).await;

    match user {
        Ok(Some(_)) => {
            let result = delete_user(user_id, &pool).await;

            match result {
                Ok(_) => {
                    tracing::info!("Deleted user with id '{}'.", user_id);
                    HttpResponse::NoContent().finish()
                }
                Err(err) => {
                    tracing::error!(
                        "Failed while trying to delete user with id '{}'. {}",
                        user_id,
                        err
                    );
                    HttpResponse::InternalServerError().json(
                        ErrorResponse::new(
                            0,
                            format!("Unable to delete user with id '{}'", user_id),
                        )
                        .description(err),
                    )
                }
            }
        }
        Ok(None) => {
            tracing::warn!("Trying to delete non-existent user with id '{}'.", user_id);
            HttpResponse::NotFound().json(ErrorResponse::new(
                0,
                format!("Trying to delete non-existent user with id '{}'", user_id),
            ))
        }
        Err(err) => {
            tracing::error!(
                "Failed while trying to delete user with id '{}'. {}",
                user_id,
                err
            );
            HttpResponse::InternalServerError().json(
                ErrorResponse::new(0, format!("Unable to delete user with id '{}'", user_id))
                    .description(err),
            )
        }
    }
}