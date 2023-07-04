use deadpool_redis::Pool;

pub type RedisPool = Pool;
pub type RedisConnection = deadpool_redis::redis::aio::Connection;
