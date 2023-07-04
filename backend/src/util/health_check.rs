use crate::types::redis::RedisPool;
use color_eyre::{eyre::eyre, Result};
use deadpool_redis::redis::{cmd, Value};
use sqlx::MySqlPool;

pub async fn database_connection_check(pool: &MySqlPool) -> Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|err| eyre!(err))
}

pub async fn redis_connection_check(pool: &RedisPool) -> Result<()> {
    let mut conn = pool.get().await?;

    cmd("SET")
        .arg(&["connection_check", "success"])
        .query_async(&mut conn)
        .await?;

    let get_value: Value = cmd("GET")
        .arg(&["connection_check"])
        .query_async(&mut conn)
        .await?;

    cmd("DEL")
        .arg(&["connection_check"])
        .query_async(&mut conn)
        .await?;

    if get_value != Value::Data("success".into()) {
        return Err(eyre!("Unexpected redis value returned"));
    }

    Ok(())
}
