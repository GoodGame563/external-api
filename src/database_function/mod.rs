pub mod connection_mongo;
mod connection_postgresql;
pub mod function_mongo;
pub mod function_postgre;
use crate::database_function::connection_mongo::PoolError;
use crate::structure::{
    receive_structures::{MainProduct, Product},
    send_structures::{
        History, HistoryElement, Product as SendProduct, SendAccount, SendSession, Task,
    },
};
use connection_mongo::{Pool as MongoPool, PoolError as MongoPoolError};
use deadpool_postgres::{Pool as PostgresPool, PoolError as PostgresPoolError};
use function_mongo::{update_photo_analysis, update_review_analysis, update_text_analysis};
use function_postgre::{SubscribeUser, User, UserSession};
use rocket::{Build, Rocket};
use uuid::Uuid;
// Error type definitions
pub enum MixPoolError {
    Postgres(PostgresPoolError),
    Mongo(MongoPoolError),
}
pub enum MixMongoAndCustomError {
    Mongo(MongoPoolError),
    Custom(String),
}
#[derive(Debug)]
pub enum MixPostgresAndCustomError {
    Postgres(PostgresPoolError),
    Custom(String),
}

// Database initialization
pub async fn init_postgre_pools(rocket: Rocket<Build>) -> Rocket<Build> {
    connection_postgresql::init_db_pool(rocket).await
}

pub async fn init_mongo_pools(rocket: Rocket<Build>) -> Rocket<Build> {
    connection_mongo::init_db_pool(rocket).await
}

// Session management
pub async fn check_client_session_id(
    pool: &PostgresPool,
    id: &Uuid,
) -> Result<bool, PostgresPoolError> {
    UserSession::check_by_id(pool, &id).await
}

pub async fn create_client_session(
    pool: &PostgresPool,
    id_user: &str,
    browser: &str,
    os: &str,
    device: &str,
) -> Result<Uuid, PostgresPoolError> {
    Ok(UserSession::create(id_user, browser, device, os, pool).await?)
}

pub async fn delete_client_session(pool: &PostgresPool, id: Uuid) -> Result<(), PostgresPoolError> {
    UserSession::delete_by_id(pool, id).await?;
    Ok(())
}

pub async fn update_check_session_time(
    pool: &PostgresPool,
    id: &Uuid,
) -> Result<bool, PostgresPoolError> {
    UserSession::update_time(pool, &id).await
}
pub async fn update_user_session_id(
    pool: &PostgresPool,
    id: &Uuid,
) -> Result<Uuid, PostgresPoolError> {
    UserSession::update_user_session_id(pool, &id).await
}
pub async fn get_user_session_by_id(
    pool: &PostgresPool,
    id: &Uuid,
) -> Result<UserSession, MixPostgresAndCustomError> {
    let option = UserSession::find_by_id(pool, &id)
        .await
        .map_err(MixPostgresAndCustomError::Postgres)?;
    let session = option.ok_or(MixPostgresAndCustomError::Custom(
        "Session not found".to_string(),
    ))?;
    Ok(session)
}

/// Task management
pub async fn create_task(
    post_pool: &PostgresPool,
    mongo_pool: &MongoPool,
    user_id: &str,
    name: &str,
    main_product: &MainProduct,
    competitors: &Vec<Product>,
    used_words: Vec<&str>,
    unused_words: Vec<&str>,
) -> Result<Uuid, MixPoolError> {
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

    Ok(uuid)
}

pub async fn set_text_analysis(
    mongo_pool: &MongoPool,
    user_id: &str,
    id: Uuid,
    data: &str,
) -> Result<(), PoolError> {
    update_text_analysis(mongo_pool, id, &user_id, data).await
}

pub async fn set_photo_analysis(
    mongo_pool: &MongoPool,
    user_id: &str,
    id: Uuid,
    data: &str,
) -> Result<(), PoolError> {
    update_photo_analysis(mongo_pool, id, user_id, data).await
}

pub async fn set_review_analysis(
    mongo_pool: &MongoPool,
    user_id: &str,
    id: Uuid,
    data: &str,
) -> Result<(), PoolError> {
    update_review_analysis(mongo_pool, id, user_id, data).await
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

pub async fn get_all_tasks(
    postgre_pool: &PostgresPool,
    user_id: &str,
) -> Result<History, PostgresPoolError> {
    let tasks = function_postgre::Task::find_by_user_id(postgre_pool, user_id)
        .await?
        .into_iter()
        .map(|task| HistoryElement {
            id: task.id,
            name: task.name,
            created_at: task.created_at,
        })
        .collect();
    Ok(History { elements: tasks })
}

pub async fn get_task_by_id(
    mongo_pool: &MongoPool,
    user_id: &str,
    id: Uuid,
) -> Result<Task, MixMongoAndCustomError> {
    let option_task = function_mongo::get_task(mongo_pool, user_id, id)
        .await
        .map_err(MixMongoAndCustomError::Mongo)?;
    let real_task =
        option_task.ok_or(MixMongoAndCustomError::Custom("Task not found".to_string()))?;
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
        products: real_task
            .competitors
            .into_iter()
            .map(|p| SendProduct {
                id: p.id,
                root: p.root,
                name: p.name,
                brand: "".to_string(),
                price: p.price as f64,
                review_rating: p.review as f64,
                description: p.description,
            })
            .collect(),
        used_words: real_task.words_analysis.used_words,
        unused_words: real_task.words_analysis.unused_words,
    };
    Ok(task)
}

pub async fn update_task_name(
    postgre_pool: &PostgresPool,
    id: Uuid,
    name: &str,
) -> Result<(), PostgresPoolError> {
    function_postgre::Task::update_name(postgre_pool, id, name).await?;
    Ok(())
}

// User/Account management
pub async fn sub_is_exist(pool: &PostgresPool, user_id: &str) -> Result<bool, PostgresPoolError> {
    match User::find_by_id(pool, user_id).await? {
        Some(user) => {
            if user.is_admin {
                return Ok(true);
            }
        }
        None => (),
    }
    Ok(SubscribeUser::get_by_user_id(pool, user_id)
        .await?
        .is_some())
}

pub async fn get_account_info(
    pool: &PostgresPool,
    user_id: &str,
) -> Result<SendAccount, MixPostgresAndCustomError> {
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(MixPostgresAndCustomError::Postgres)?;
    let sessions = UserSession::find_by_user_id(pool, user_id)
        .await
        .map_err(MixPostgresAndCustomError::Postgres)?;
    if let Some(user) = user {
        return Ok(SendAccount {
            name: user.name,
            email: user.email,
            sessions: sessions
                .into_iter()
                .map(|s| SendSession {
                    id: s.id,
                    browser: s.browser,
                    last_activity: s._last_activity,
                })
                .collect(),
        });
    } else {
        return Err(MixPostgresAndCustomError::Custom(
            "User not found".to_string(),
        ));
    }
}
