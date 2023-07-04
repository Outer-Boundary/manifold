use crate::{
    routes::{health_check::health_check_route, users::users_scope},
    util::configuration::{Configuration, DatabaseConfiguration},
};
use actix_web::{
    dev::Server,
    web::{scope, Data},
    App, HttpServer,
};
use color_eyre::{eyre::eyre, Result};
use sqlx::MySqlPool;
use std::net::TcpListener;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(config: Configuration, test_pool: Option<MySqlPool>) -> Result<Self> {
        let connection_pool = if let Some(pool) = test_pool {
            pool
        } else {
            get_connection_pool(&config.database).await
        };

        sqlx::migrate!()
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database");

        let address = format!("{}:{}", config.server.host, config.server.port);

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, config).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<()> {
        self.server.await.map_err(|err| eyre!(err))
    }
}

pub async fn get_connection_pool(settings: &DatabaseConfiguration) -> MySqlPool {
    MySqlPool::connect_lazy_with(settings.connect_to_db())
}

async fn run(listener: TcpListener, db_pool: MySqlPool, config: Configuration) -> Result<Server> {
    let cfg = deadpool_redis::Config::from_url(config.redis.url);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Cannot create deadpool redis");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db_pool.clone()))
            .app_data(Data::new(redis_pool.clone()))
            .service(
                scope("/api")
                    .service(health_check_route)
                    .service(scope("/users").configure(users_scope)),
            )
    })
    .listen(listener)?
    .run();

    Ok(server)
}
