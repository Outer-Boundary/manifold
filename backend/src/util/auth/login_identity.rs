use crate::{models::login_identity::*, types::redis::RedisPool};
use color_eyre::{eyre::eyre, Result};
use futures::prelude::*;
use sqlx::MySqlPool;
use uuid::Uuid;

use super::{password::hash_password, tokens::verify_confirmation_token};

#[tracing::instrument(skip(db_pool))]
pub async fn get_login_identity(
    user_id: Uuid,
    li_type: LoginIdentityType,
    db_pool: &MySqlPool,
) -> Result<Option<LoginIdentity>> {
    let li = match li_type {
        LoginIdentityType::Email => {
            let li = sqlx::query_as!(
                LIEmail,
                "SELECT user_id AS `user_id: Uuid`, email, password_hash, salt, created_at, updated_at FROM login_identity__email WHERE user_id = ?",
                user_id
            )
            .fetch_optional(db_pool)
            .await?;

            li.map(LoginIdentity::Email)
        }
    };

    Ok(li)
}

#[tracing::instrument(skip(db_pool))]
pub async fn get_login_identities(
    user_id: Uuid,
    db_pool: &MySqlPool,
) -> Result<Vec<LoginIdentity>> {
    let all_li: Vec<LoginIdentity> = futures::stream::iter(LoginIdentityType::all())
        .then(|li_type| async move { get_login_identity(user_id, li_type.clone(), db_pool).await })
        .try_collect::<Vec<Option<LoginIdentity>>>()
        .await?
        .into_iter()
        .flatten()
        .collect();

    Ok(all_li)
}

#[tracing::instrument(skip(db_pool))]
pub async fn add_login_identity(
    user_id: Uuid,
    new_li: NewLoginIdentity,
    db_pool: &MySqlPool,
) -> Result<()> {
    match new_li {
        NewLoginIdentity::Email(li) => {
            let (password_hash, salt) = hash_password(li.password).await?;
            let salt = hex::encode(salt.as_bytes());

            sqlx::query_as!(
                LIEmail,
                "INSERT INTO login_identity__email (user_id, email, password_hash, salt) VALUES (?, ?, ?, ?)",
                user_id,
                li.email,
                password_hash,
                salt
            )
            .fetch_optional(db_pool)
            .await?;
        }
    };

    Ok(())
}

#[tracing::instrument(skip(db_pool))]
pub async fn delete_login_identity(
    user_id: Uuid,
    li_type: LoginIdentityType,
    db_pool: &MySqlPool,
) -> Result<()> {
    match li_type {
        LoginIdentityType::Email => {
            sqlx::query!(
                "DELETE FROM login_identity__email WHERE user_id = ?",
                user_id
            )
            .execute(db_pool)
            .await?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip(db_pool))]
pub async fn delete_all_login_identities(user_id: Uuid, db_pool: &MySqlPool) -> Result<()> {
    futures::stream::iter(LoginIdentityType::all())
        .then(
            |li_type| async move { delete_login_identity(user_id, li_type.clone(), db_pool).await },
        )
        .try_collect()
        .await?;

    Ok(())
}

#[tracing::instrument(skip(db_pool, redis))]
pub async fn verify_login_identity(
    token: String,
    db_pool: &MySqlPool,
    redis: &RedisPool,
) -> Result<Uuid> {
    let token = verify_confirmation_token(token, redis, false).await?;

    let li_type_claim = token
        .claims
        .get_claim("li_type")
        .ok_or(eyre!("No login identity type claim in confirmation token"))?;

    let li_type_str = li_type_claim.as_str().ok_or(eyre!(
        "Login identity claim data type was not type 'string'"
    ))?;

    let li_type = serde_json::from_str::<LoginIdentityType>(li_type_str)?;

    set_login_identity_verified(token.user_id, li_type, db_pool).await?;

    Ok(token.user_id)
}

#[tracing::instrument(skip(db_pool))]
pub async fn set_login_identity_verified(
    user_id: Uuid,
    li_type: LoginIdentityType,
    db_pool: &MySqlPool,
) -> Result<()> {
    match li_type {
        LoginIdentityType::Email => {
            sqlx::query!(
                "UPDATE login_identity__email SET verified = true WHERE user_id = ?",
                user_id
            )
            .execute(db_pool)
            .await?;
        }
    }

    Ok(())
}
