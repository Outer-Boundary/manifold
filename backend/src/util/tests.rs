use color_eyre::Result;
use sqlx::{mysql::MySqlPoolOptions, Executor, MySqlPool};

pub struct TestPool {
    pub pool: MySqlPool,
}

impl TestPool {
    pub async fn connect() -> Result<Self> {
        dotenvy::dotenv()?;

        let pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&std::env::var("DATABASE_URL")?)
            .await?;

        pool.execute("START TRANSACTION").await?;

        Ok(TestPool { pool })
    }

    pub fn get(&self) -> MySqlPool {
        self.pool.clone()
    }
}

impl Drop for TestPool {
    fn drop(&mut self) {
        let pool = self.get();
        actix_web::rt::spawn(async move {
            sqlx::query!("ROLLBACK").execute(&pool).await.ok();
        });
    }
}
