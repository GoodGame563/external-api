pub mod function_postgre;
pub mod function_mongo;
mod connection_postgresql;
pub mod connection_mongo;

use deadpool_postgres::{Pool as PostgresPool, PoolError as PostgresPoolError};
use connection_mongo::{Pool as MongoPool, PoolError as MongoPoolError};
use rocket::{Build, Rocket};
use function_postgre::UserSession;
use tokio::task;
use uuid::Uuid;
use crate::structure::{receive_structures::{MainProduct, Product}, send_structures::{History, HistoryElement, Task, Product as SendProduct}};

pub enum MixPoolError {
    Postgres(PostgresPoolError),
    Mongo(MongoPoolError),
    Custom(String),
}

pub async fn check_client_session_user_id(
    pool: &PostgresPool,
    id_user: &str,
) -> Result<bool, PostgresPoolError> {
    let users = UserSession::find_by_user_id(pool, id_user).await?;
    if users.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub async fn check_client_session_id(
    pool: &PostgresPool,
    id: Uuid,
) -> Result<bool, PostgresPoolError> {
    UserSession::find_by_id(pool, &id).await.map(|us| {
        if us.is_some() {
            return Ok(true);
        }
        Ok(false)
    })?
}

pub async fn create_client_session(
    pool: &PostgresPool,
    id_user: &str,
    browser: &str,
    os: &str,
    device: &str,
) -> Result<bool, PostgresPoolError> {
    for cs in UserSession::find_by_user_id(pool, id_user).await? {
        if browser == cs.browser && os == cs.os && device == cs.device {
            return Ok(false);
        }
    }
    UserSession::create(id_user, browser, device, os, pool).await?;
    Ok(true)
}

pub async fn delete_client_session(
    pool: &PostgresPool,
    id: Uuid,
) -> Result<(), PostgresPoolError> {
    UserSession::delete_by_id(pool, id).await?;
    Ok(())
}

pub async fn create_task(
    post_pool: &PostgresPool,
    mongo_pool: &MongoPool,
    user_id: &str,
    name: &str,
    main_product: &MainProduct,
    competitors: &Vec<Product>,
    used_words: Vec<&str>,
    unused_words: Vec<&str>,
) -> Result<(), MixPoolError> {
    let competitors = competitors
        .iter()
        .map(|p| function_mongo::Product {
            description: p.description.clone(),
            id: p.id,
            name: p.name.clone(),
            root: p.root,
            price: p.price,
            review: p.review_rating,
        })
        .collect::<Vec<_>>();

    let uuid = function_postgre::Task::create(name, user_id, post_pool)
        .await
        .map_err(MixPoolError::Postgres)?;

    function_mongo::create_task(
        mongo_pool,
        user_id,
        uuid,
        function_mongo::Product {
            description: main_product.description.clone(),
            id: main_product.id,
            name: main_product.name.clone(),
            root: main_product.root,
            price: 0,
            review: 0.0,
        },
        competitors,
        used_words.into_iter().map(|s| s.to_string()).collect(),
        unused_words.into_iter().map(|s| s.to_string()).collect(),
    )
    .await
    .map_err(MixPoolError::Mongo)?;

    Ok(())
}

pub async fn regenerate_task(
    post_pool: &PostgresPool,
    mongo_pool: &MongoPool,
    id: &Uuid,
    user_id: &str,
    main_product: &MainProduct,
    competitors: &Vec<Product>,
    used_words: Vec<&str>,
    unused_words: Vec<&str>,
) -> Result<(), MixPoolError> {
    let competitors = competitors
        .iter()
        .map(|p| function_mongo::Product {
            description: p.description.clone(),
            id: p.id,
            name: p.name.clone(),
            root: p.root,
            price: p.price,
            review: p.review_rating,
        })
        .collect::<Vec<_>>();

    function_postgre::Task::update_time(post_pool, id)
        .await
        .map_err(MixPoolError::Postgres)?;

    function_mongo::update_task(
        mongo_pool,
        user_id,
        id.clone(),
        function_mongo::Product {
            description: main_product.description.clone(),
            id: main_product.id,
            name: main_product.name.clone(),
            root: main_product.root,
            price: 0,
            review: 0.0,
        },
        competitors,
        used_words.into_iter().map(|s| s.to_string()).collect(),
        unused_words.into_iter().map(|s| s.to_string()).collect(),
    )
    .await
    .map_err(MixPoolError::Mongo)?;

    Ok(())
}

pub async fn init_postgre_pools(rocket: Rocket<Build>) -> Rocket<Build> {
    connection_postgresql::init_db_pool(rocket).await
}

pub async fn init_mongo_pools(rocket: Rocket<Build>) -> Rocket<Build> {
    connection_mongo::init_db_pool(rocket).await
}

pub async fn get_all_tasks(
    postgre_pool: &PostgresPool,
    user_id: &str,
) -> Result<History, PostgresPoolError> {
    let tasks = function_postgre::Task::find_by_user_id(postgre_pool, user_id).await?.into_iter().map(|task| {HistoryElement{ id: task.id, name: task.name, created_at:task.created_at}}).collect();
    Ok(History{
        elements: tasks
    })
}

pub async fn update_task_name(
    postgre_pool: &PostgresPool,
    id: Uuid,
    name: &str,
) -> Result<(), PostgresPoolError> {
    function_postgre::Task::update_name(postgre_pool, id, name).await?;
    Ok(())
}

pub async fn get_task_by_id(
    mongo_pool: &MongoPool,
    user_id: &str,
    id: Uuid,
) -> Result<Task, MixPoolError> {
    let option_task = function_mongo::get_task(mongo_pool, user_id, id)
    .await
    .map_err(MixPoolError::Mongo)?; 
    let real_task = option_task.ok_or(MixPoolError::Custom("Task not found".to_string()))?;
    let task = Task {
        main: SendProduct {
            id: real_task.main_product.id,
            root: real_task.main_product.root,
            name: real_task.main_product.name,
            brand: "".to_string(),
            price: 0.0,
            review_rating: 0.0,
            description: real_task.main_product.description,
        },
        products: real_task.competitors.into_iter().map(|p| SendProduct {
            id: p.id,
            root: p.root,
            name: p.name,
            brand: "".to_string(),
            price: p.price as f64,
            review_rating: p.review as f64,
            description: p.description,
        }).collect(),
        used_words: real_task.words_analysis.used_words,
        unused_words: real_task.words_analysis.unused_words,
    };
    Ok(task)
    
}