use std::collections::HashMap;

use color_eyre::{eyre::eyre, Result};
use deadpool_redis::redis::AsyncCommands;
use pasetors::{
    claims::{Claims, ClaimsValidationRules},
    keys::SymmetricKey,
    local,
    token::UntrustedToken,
    version4::V4,
    Local,
};
use rand::{rngs::OsRng, RngCore};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    types::{redis::RedisPool, tokens::ConfirmationToken},
    util::configuration::{get_config, SecretConfiguration},
};

const SESSION_KEY_PREFIX: &str = "MANIFOLD";
const PASSWORD_CHANGE_KEY_PREFIX: &str = "PWD_CHG";

pub struct ConfirmationTokenBuilder<'a> {
    secret_config: SecretConfiguration,
    user_id: Uuid,
    redis: &'a RedisPool,
    session_key_prefix: String,
    ttl: chrono::Duration,
    claims: HashMap<String, String>,
}

impl<'a> ConfirmationTokenBuilder<'a> {
    #[tracing::instrument(skip(redis))]
    pub fn create(user_id: Uuid, redis: &'a RedisPool) -> Result<Self> {
        let config = get_config()?;

        Ok(ConfirmationTokenBuilder {
            secret_config: config.secret.clone(),
            user_id,
            redis,
            session_key_prefix: SESSION_KEY_PREFIX.to_string(),
            ttl: chrono::Duration::minutes(config.secret.token_expiration),
            claims: HashMap::new(),
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn for_password_change(&mut self) -> &mut Self {
        self.session_key_prefix =
            format!("{}_{}", self.session_key_prefix, PASSWORD_CHANGE_KEY_PREFIX);
        self.ttl = chrono::Duration::minutes(60);
        self
    }

    #[tracing::instrument(skip(self, value))]
    pub fn add_claim(&mut self, claim: String, value: impl Serialize) -> Result<&mut Self> {
        self.claims.insert(claim, serde_json::to_string(&value)?);
        Ok(self)
    }

    #[tracing::instrument(skip(self))]
    pub async fn issue_confirmation_token(&mut self) -> Result<(String, i64)> {
        let session_key: String = {
            let mut buff = [0u8; 128];
            OsRng.fill_bytes(&mut buff);
            hex::encode(buff)
        };

        let redis_key = format!("{}_{}", self.session_key_prefix, session_key);

        let mut redis_conn = self.redis.get().await?;

        redis_conn.set(redis_key.clone(), String::new()).await?;

        let current_date_time = chrono::Utc::now();
        let dt = current_date_time + self.ttl;

        redis_conn
            .expire(redis_key.clone(), self.ttl.num_seconds().try_into()?)
            .await?;

        let mut claims = Claims::new()?;
        claims.expiration(&dt.to_rfc3339())?;
        claims.add_additional("user_id", serde_json::json!(self.user_id))?;
        claims.add_additional("session_key", serde_json::json!(session_key))?;
        for (key, value) in self.claims.clone() {
            claims.add_additional(&key, value)?;
        }

        let sk = SymmetricKey::<V4>::from(self.secret_config.secret_key.as_bytes())?;
        Ok((
            local::encrypt(
                &sk,
                &claims,
                None,
                Some(self.secret_config.hmac_secret.as_bytes()),
            )?,
            self.ttl.num_seconds(),
        ))
    }
}

#[tracing::instrument(skip(redis))]
pub async fn verify_confirmation_token<'a>(
    token: String,
    redis: &'a RedisPool,
    is_password: bool,
) -> Result<ConfirmationToken> {
    let config = get_config()?;
    let mut redis_conn = redis.get().await?;
    let sk = SymmetricKey::<V4>::from(config.secret.secret_key.as_bytes())?;

    let validation_rules = ClaimsValidationRules::new();
    let untrusted_token = UntrustedToken::<Local, V4>::try_from(&token)?;
    let trusted_token = local::decrypt(
        &sk,
        &untrusted_token,
        &validation_rules,
        None,
        Some(config.secret.hmac_secret.as_bytes()),
    )?;
    let claims = trusted_token
        .payload_claims()
        .ok_or(eyre!("No claims found"))?;

    let uid = serde_json::to_value(
        claims
            .get_claim("user_id")
            .ok_or(eyre!("No claim with key 'user_id'"))?,
    )?;

    let uuid_string = serde_json::from_value::<String>(uid)?;
    let user_id = uuid::Uuid::parse_str(&uuid_string)?;

    let sss_key = serde_json::to_value(
        claims
            .get_claim("session_key")
            .ok_or(eyre!("No claim with key 'session_key'"))?,
    )?;
    let session_key = serde_json::from_value::<String>(sss_key)?;

    let redis_key = if is_password {
        format!(
            "{}_{}_{}",
            SESSION_KEY_PREFIX, PASSWORD_CHANGE_KEY_PREFIX, session_key
        )
    } else {
        format!("{}_{}", SESSION_KEY_PREFIX, session_key)
    };

    if redis_conn
        .get::<_, Option<String>>(redis_key.clone())
        .await?
        .is_none()
    {
        return Err(eyre!("Could not find redis key"));
    };

    redis_conn.del(redis_key.clone()).await?;
    Ok(ConfirmationToken {
        user_id,
        claims: claims.clone(),
    })
}
