use crate::database_function::function_mongo::check_and_create_db;
use deadpool::managed;
use mongodb::{bson::doc, error::Error, options::ClientOptions, Client};
use rocket::{Build, Rocket};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct Config {
    connection_string: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        let connection_string = env::var("CONNECTION_STRING_MONGO")?;
        Ok(Self { connection_string })
    }
}
pub struct MongoManager {
    connection_string: String,
}

impl MongoManager {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

impl managed::Manager for MongoManager {
    type Type = Client;
    type Error = Error;

    async fn create(&self) -> Result<Client, Error> {
        let options = ClientOptions::parse(&self.connection_string).await?;
        Client::with_options(options)
    }

    async fn recycle(&self, _: &mut Client, _: &managed::Metrics) -> managed::RecycleResult<Error> {
        Ok(())
    }
}

pub type Pool = managed::Pool<MongoManager>;
pub type PoolError = managed::PoolError<Error>;

pub async fn init_db_pool(rocket: Rocket<Build>) -> Rocket<Build> {
    let figment = rocket.figment();
    let pool_size: usize = figment
        .extract_inner("default.databases.mongo_db.pool_size")
        .unwrap_or(20);
    let config = Config::from_env().expect("Failed to load MongoDB config");
    let manager = MongoManager::new(config.connection_string);
    let pool = Pool::builder(manager).max_size(pool_size).build().unwrap();
    let client = pool.get().await.unwrap();
    check_and_create_db(&client).await.unwrap();
    rocket.manage(pool)
}
