use color_eyre::Result;
use sqlx::{mysql::MySqlPoolOptions, Executor, MySqlPool};

pub struct TestPool {
    pub db_pool: MySqlPool,
}

impl TestPool {
    pub async fn connect() -> Result<Self> {
        dotenvy::dotenv()?;

        let db_pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&std::env::var("DATABASE_URL")?)
            .await?;

        db_pool.execute("START TRANSACTION").await?;

        Ok(TestPool { db_pool })
    }

    pub fn get(&self) -> MySqlPool {
        self.db_pool.clone()
    }
}

impl Drop for TestPool {
    fn drop(&mut self) {
        let db_pool = self.get();
        actix_web::rt::spawn(async move {
            sqlx::query!("ROLLBACK").execute(&db_pool).await.ok();
        });
    }
}
