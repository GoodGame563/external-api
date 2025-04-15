use deadpool_postgres::{Pool, Runtime};
use rocket::{Build, Rocket};
use tokio_postgres::NoTls;

#[derive(Debug, serde::Deserialize)]
struct Config {
    pg: deadpool_postgres::Config,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
            )
            .build()?
            .try_deserialize()
    }
}

pub async fn init_db_pool(rocket: Rocket<Build>) -> Rocket<Build> {
    let figment = rocket.figment();
    let pool_size: u32 = figment
        .extract_inner("databases.postgres.pool_size")
        .unwrap_or(20);

    let cfg = Config::from_env().unwrap();
    let mgr = deadpool_postgres::Manager::new(
         cfg.pg.get_pg_config().expect("Not find env file"),
        NoTls,
    );


    let pool = Pool::builder(mgr)
        .max_size(pool_size as usize)
        .runtime(Runtime::Tokio1)
        .build()
        .unwrap();

    rocket.manage(pool)
}