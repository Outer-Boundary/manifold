use crate::{
    routes::{auth::auth_scope, health_check::health_check_route, users::users_scope},
    util::configuration::{Configuration, DatabaseConfiguration, Environment},
};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie,
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
        let db_pool = if let Some(db_pool) = test_pool {
            db_pool
        } else {
            get_connection_pool(&config.database).await
        };

        sqlx::migrate!().run(&db_pool).await?;

        let address = format!("{}:{}", config.server.host, config.server.port);

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr()?.port();
        let server = run(listener, db_pool, config).await?;

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
    let redis_config = deadpool_redis::Config::from_url(config.redis.url);
    let redis_pool = redis_config.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;

    let secret_key = cookie::Key::from(config.secret.hmac_secret.as_bytes());

    let server = HttpServer::new(move || {
        App::new()
            .wrap(if let Environment::Development = config.environment {
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_http_only(true)
                    .cookie_same_site(cookie::SameSite::None)
                    .cookie_secure(true)
                    .build()
            } else {
                SessionMiddleware::new(CookieSessionStore::default(), secret_key.clone())
            })
            .app_data(Data::new(db_pool.clone()))
            .app_data(Data::new(redis_pool.clone()))
            .service(
                scope("/api")
                    .service(health_check_route)
                    .service(scope("/users").configure(users_scope))
                    .service(scope("/auth").configure(auth_scope)),
            )
    })
    .listen(listener)?
    .run();

    Ok(server)
}
