mod database_function;
mod jwt;
mod nats;
mod rabbit;
mod structure;
mod utils;

#[macro_use]
extern crate rocket;

use chrono::Duration;
use dotenvy::dotenv;

use crate::jwt::{
    create_access_jwt, create_refresh_jwt, validate_data_token_refresh, validate_refresh_jwt,
};
use api::parser_integration_service_client::ParserIntegrationServiceClient;
use database_function::{
    check_client_session_id, create_subscribe, create_task as create_task_db,
    delete_client_session, delete_task as delete_task_db, function_postgre::User, get_account_info,
    get_all_tasks, get_all_users, get_task_by_id, get_user_session_by_id, init_mongo_pools,
    init_postgre_pools, is_admin, set_admin, set_photo_analysis, set_review_analysis,
    set_text_analysis, sub_is_exist, update_check_session_time, update_task_name,
    update_user_session_id, MixMongoAndCustomError, MixPoolError, MixPostgresAndCustomError,
};
use futures::StreamExt;
use rocket::{config::SecretKey, http::CookieJar};
use rocket::{
    fairing::AdHoc,
    http::{Method, Status},
    request::{FromRequest, Outcome, Request},
    response::stream::TextStream,
    serde::json::Json,
    State,
};
use structure::receive_structures::{
    ChangeToAdminData, CreateSubscribe, CreateTask, DeleteTask, EditTask, EditTaskName, GetTask,
    InformationTask, RefreshTokenStructure, TaskMessage,
};
use structure::send_structures::{
    ErrorMessage, History, SendAccount, SendMessage, SendUser, Subscribtion, Task, TaskId, Token,
    Tokens,
};

use database_function::connection_mongo::Pool as MongoPool;
use deadpool_postgres::Pool as PostgresPool;
use log::error;
use nats::{get_messages_stream, init_connection_to_stream as init_nats_stream, NatsStream};
use rabbit::{
    init_rabbit_queues, send_task_to_photo_analysis_queue, send_task_to_reviews_analysis_queue,
    send_task_to_text_analysis_queue, RabbitChannel,
};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::net::{IpAddr, Ipv4Addr};
use std::str::{from_utf8, FromStr};
use utils::hash_str;
mod api {
    tonic::include_proto!("api");
}

const STREAM_NAME: &str = "ai_stream";

#[derive(Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub id: uuid::Uuid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = String;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = request.headers().get_one("Authorization");
        let token = match auth_header {
            Some(header) => header.strip_prefix("Bearer ").unwrap_or(header),
            None => return Outcome::Error((Status::Unauthorized, "Missing token".to_string())),
        };

        match jwt::validate_access_jwt(token) {
            Ok(token_data) => Outcome::Success(AuthUser {
                user_id: token_data.user_id,
                id: token_data.id,
            }),
            Err(e) => Outcome::Error((Status::Unauthorized, format!("Invalid token: {}", e))),
        }
    }
}

#[post("/authorization", data = "<data>")]
async fn authorization(
    pool: &State<PostgresPool>,
    data: Json<crate::structure::receive_structures::Enter>,
    cookies: &CookieJar<'_>,
) -> Result<(Status, Json<Tokens>), (Status, Json<ErrorMessage>)> {
    let access_life_time = Duration::minutes(30);
    let refresh_life_time = Duration::weeks(2);
    let hash_id = hash_str(&format!("{}{}", data.email, data.password)).map_err(|e| {
        error!("Hash error: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "hash error".to_string(),
            }),
        )
    })?;

    let user = User::find_by_id(pool, &hash_id)
        .await
        .map_err(|e| {
            error!("Failed to find user by ID: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't find".to_string(),
                }),
            )
        })?
        .ok_or((
            Status::NotFound,
            Json(ErrorMessage {
                message: "user not found".to_string(),
            }),
        ))?;

    let client_session_id = database_function::create_client_session(
        pool,
        &user.id,
        &data.browser,
        &data.os,
        &data.device,
    )
    .await
    .map_err(|e| {
        error!("Failed to create client session: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create client session".to_string(),
            }),
        )
    })?;

    let access_token = create_access_jwt(&client_session_id, &user.id, access_life_time)
        .map_err(|e| {
            error!("Failed to create access JWT: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create access jwt".to_string(),
                }),
            )
        })
        .map(|token| Token {
            token,
            life_time: access_life_time,
        })?;

    let refresh_token = create_refresh_jwt(
        &client_session_id,
        &data.browser,
        &data.device,
        &data.os,
        refresh_life_time,
    )
    .map_err(|e| {
        error!("Failed to create refresh JWT: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create refresh jwt".to_string(),
            }),
        )
    })
    .map(|token| Token {
        token,
        life_time: refresh_life_time,
    })?;
    cookies.add_private(("refresh_token", refresh_token.token));

    Ok((
        Status::Ok,
        Json(Tokens {
            access_token,
            refresh_token_life_time: refresh_life_time,
        }),
    ))
}

#[post("/registration", data = "<data>")]
async fn registration(
    pool: &State<PostgresPool>,
    data: Json<crate::structure::receive_structures::Registration>,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    let hash_id = hash_str(&format!("{}{}", data.email, data.password)).map_err(|e| {
        error!("Hash error: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "hash error".to_string(),
            }),
        )
    })?;

    match User::find_by_email(pool, &data.email).await {
        Ok(true) => Err((
            Status::Conflict,
            Json(ErrorMessage {
                message: "user with this email already exisist".to_string(),
            }),
        )),
        Ok(false) => {
            let user = User {
                id: hash_id,
                email: data.email.to_string(),
                name: data.name.to_string(),
                is_admin: false,
            };
            User::create(&user, pool)
                .await
                .map(|_| Status::Created)
                .map_err(|e| {
                    error!("Failed to create new user: {}", e);
                    (
                        Status::InternalServerError,
                        Json(ErrorMessage {
                            message: "can't create new user".to_string(),
                        }),
                    )
                })
        }
        Err(e) => {
            error!("Database connection error: {}", e);
            Err((
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "database not available".to_string(),
                }),
            ))
        }
    }
}

#[post("/refresh", data = "<data>")]
async fn refresh(
    cookies: &CookieJar<'_>,
    pool: &State<PostgresPool>,
    data: Json<RefreshTokenStructure>,
) -> Result<(Status, Json<Tokens>), (Status, Json<ErrorMessage>)> {
    let access_life_time = Duration::minutes(30);
    let refresh_life_time = Duration::weeks(2);
    let option_refresh_token = cookies
        .get_private("refresh_token")
        .map(|crumb| crumb.value().to_string());
    let refresh_token = match option_refresh_token {
        Some(value) => value,
        None => "".to_string(),
    };
    let refresh_data_in_jwt = validate_refresh_jwt(&refresh_token).map_err(|e| {
        error!("Invalid refresh token, token timeout: {}", e);
        cookies.remove_private("refresh_token");
        (
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Token is time out.".to_string(),
            }),
        )
    })?;

    if !validate_data_token_refresh(&refresh_data_in_jwt, &data.browser, &data.os, &data.device) {
        cookies.remove_private("refresh_token");
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    let client_session_exsist = check_client_session_id(pool, &refresh_data_in_jwt.id)
        .await
        .map_err(|e| {
            error!("Database connection error: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't connect to database".to_string(),
                }),
            )
        })?;

    if !client_session_exsist {
        cookies.remove_private("refresh_token");
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    let id_user_session = update_user_session_id(pool, &refresh_data_in_jwt.id)
        .await
        .map_err(|e| {
            error!("Database connection error: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't connect to database".to_string(),
                }),
            )
        })?;

    let user_session = get_user_session_by_id(pool, &id_user_session)
        .await
        .map_err(|e| {
            error!("Database connection error: {:?}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't connect to database".to_string(),
                }),
            )
        })?;

    let access_token = create_access_jwt(&user_session.id, &user_session.user_id, access_life_time)
        .map_err(|e| {
            error!("Failed to create access JWT: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create access jwt".to_string(),
                }),
            )
        })
        .map(|token| Token {
            token,
            life_time: access_life_time,
        })?;
    let refresh_token = create_refresh_jwt(
        &user_session.id,
        &user_session.browser,
        &user_session.device,
        &user_session.os,
        refresh_life_time,
    )
    .map_err(|e| {
        error!("Failed to create refresh JWT: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create refresh jwt".to_string(),
            }),
        )
    })
    .map(|token| Token {
        token,
        life_time: refresh_life_time,
    })?;
    cookies.remove_private("refresh_token");
    cookies.add_private(("refresh_token", refresh_token.token));

    Ok((
        Status::Ok,
        Json(Tokens {
            access_token,
            refresh_token_life_time: refresh_life_time,
        }),
    ))
}

#[post("/task", data = "<data>")]
async fn delete_task(
    pool: &State<PostgresPool>,
    mongo_pool: &State<MongoPool>,
    data: Json<DeleteTask>,
    user: AuthUser,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    delete_task_db(pool, mongo_pool, &user.user_id, &data.id)
        .await
        .map_err(|e| {
            error!("Failed to delete task: {:?}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't delete task".to_string(),
                }),
            )
        })?;
    Ok(Status::Ok)
}

#[post("/task", data = "<data>")]
async fn create_task(
    pool: &State<PostgresPool>,
    mongo_pool: &State<MongoPool>,
    data: Json<CreateTask>,
    user: AuthUser,
    channel: &State<RabbitChannel>,
) -> Result<(Status, Json<TaskId>), (Status, Json<ErrorMessage>)> {
    if !update_check_session_time(pool, &user.id)
        .await
        .map_err(|e| {
            error!("Failed to update session time: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't update session time".to_string(),
                }),
            )
        })?
    {
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    if !sub_is_exist(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check subscription existence: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create new user".to_string(),
            }),
        )
    })? {
        return Err((
            Status::PaymentRequired,
            Json(ErrorMessage {
                message: "user is not paid".to_string(),
            }),
        ));
    }
    let task_id = create_task_db(
        pool,
        mongo_pool,
        &user.user_id,
        &data.main.name,
        &data.main,
        &data.products,
        data.used_words
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
        data.unused_words
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
    )
    .await
    .map_err(|e| match e {
        MixPoolError::Postgres(e) => {
            error!("Failed to create task side postgres: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create task side postgres".to_string(),
                }),
            )
        }
        MixPoolError::Mongo(e) => {
            error!("Failed to create task side mongo: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create task side mongo".to_string(),
                }),
            )
        }
    })?;
    let mut text_vec = vec![];
    let mut photo_vec = vec![];
    let mut reviews_vec = Vec::new();

    let mut main_reviews = Vec::new();
    for review in &data.main.reviews {
        let review_str = format!(
            "текст:{};понравилось:{};не понравилось:{};",
            review.text, review.pros, review.cons
        );
        main_reviews.push(review_str);
    }
    reviews_vec.push(main_reviews);

    text_vec.push(data.main.description.as_str());
    photo_vec.push(data.main.image_url.as_str());

    for product in &data.products {
        text_vec.push(product.description.as_str());
        photo_vec.push(product.image_url.as_str());

        let mut product_reviews = Vec::new();
        for review in &product.reviews {
            let review_str = format!(
                "текст:{};понравилось:{};не понравилось:{};",
                review.text, review.pros, review.cons
            );
            product_reviews.push(review_str);
        }
        reviews_vec.push(product_reviews);
    }

    let text_future = send_task_to_text_analysis_queue(channel, &task_id, text_vec);
    let photo_future = send_task_to_photo_analysis_queue(channel, &task_id, photo_vec);
    let reviews_future = send_task_to_reviews_analysis_queue(channel, &task_id, reviews_vec);

    let (text_result, photo_result, reviews_result) =
        futures::join!(text_future, photo_future, reviews_future);

    text_result.map_err(|e| {
        error!("Failed to send task to text analysis queue: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't send task to text analysis queue".to_string(),
            }),
        )
    })?;

    photo_result.map_err(|e| {
        error!("Failed to send task to photo analysis queue: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't send task to photo analysis queue".to_string(),
            }),
        )
    })?;

    reviews_result.map_err(|e| {
        error!("Failed to send task to reviews analysis queue: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't send task to reviews analysis queue".to_string(),
            }),
        )
    })?;
    Ok((Status::Created, Json(TaskId { id: task_id })))
}

#[post("/task", data = "<data>")]
async fn edit_task(
    pool: &State<PostgresPool>,
    mongo_pool: &State<MongoPool>,
    data: Json<EditTask>,
    user: AuthUser,
    channel: &State<RabbitChannel>,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    if !update_check_session_time(pool, &user.id)
        .await
        .map_err(|e| {
            error!("Failed to update session time: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't update session time".to_string(),
                }),
            )
        })?
    {
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    if !sub_is_exist(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check subscription existence: {}", e);
        (
            Status::PaymentRequired,
            Json(ErrorMessage {
                message: "buy subscribtion".to_string(),
            }),
        )
    })? {
        return Ok(Status::PaymentRequired);
    }
    database_function::regenerate_task(
        pool,
        mongo_pool,
        &data.id,
        &user.user_id,
        &data.main,
        &data.products,
        data.used_words
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
        data.unused_words
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>(),
    )
    .await
    .map_err(|e| match e {
        MixPoolError::Postgres(e) => {
            error!("Failed to create task side postgres: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create task side postgres".to_string(),
                }),
            )
        }
        MixPoolError::Mongo(e) => {
            error!("Failed to create task side mongo: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create task side mongo".to_string(),
                }),
            )
        }
    })?;
    let mut text_vec = vec![];
    let mut photo_vec = vec![];
    let mut reviews_vec = Vec::new();

    let mut main_reviews = Vec::new();
    for review in &data.main.reviews {
        let review_str = format!(
            "текст:{};понравилось:{};не понравилось:{};",
            review.text, review.pros, review.cons
        );
        main_reviews.push(review_str);
    }
    reviews_vec.push(main_reviews);

    text_vec.push(data.main.description.as_str());
    photo_vec.push(data.main.image_url.as_str());

    for product in &data.products {
        text_vec.push(product.description.as_str());
        photo_vec.push(product.image_url.as_str());

        let mut product_reviews = Vec::new();
        for review in &product.reviews {
            let review_str = format!(
                "текст:{};понравилось:{};не понравилось:{};",
                review.text, review.pros, review.cons
            );
            product_reviews.push(review_str);
        }
        reviews_vec.push(product_reviews);
    }

    let text_future = send_task_to_text_analysis_queue(channel, &data.id, text_vec);
    let photo_future = send_task_to_photo_analysis_queue(channel, &data.id, photo_vec);
    let reviews_future = send_task_to_reviews_analysis_queue(channel, &data.id, reviews_vec);

    let (text_result, photo_result, reviews_result) =
        futures::join!(text_future, photo_future, reviews_future);

    text_result.map_err(|e| {
        error!("Failed to send task to text analysis queue: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't send task to text analysis queue".to_string(),
            }),
        )
    })?;

    photo_result.map_err(|e| {
        error!("Failed to send task to photo analysis queue: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't send task to photo analysis queue".to_string(),
            }),
        )
    })?;

    reviews_result.map_err(|e| {
        error!("Failed to send task to reviews analysis queue: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't send task to reviews analysis queue".to_string(),
            }),
        )
    })?;
    Ok(Status::Created)
}

#[get("/words/<product_id>")]
async fn get_words_from_url(
    product_id: i32,
    user: AuthUser,
    pool: &State<PostgresPool>,
) -> Result<(Status, Json<Vec<String>>), (Status, Json<ErrorMessage>)> {
    if !sub_is_exist(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check subscription existence: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create new user".to_string(),
            }),
        )
    })? {
        return Ok((Status::PaymentRequired, Json(vec![])));
    }
    let mut client =
        ParserIntegrationServiceClient::connect(std::env::var("URL_INTEGRATION_SERVICE")
            .unwrap_or("internal_api:50051".to_string()))
            .await
            .map_err(|e| {
                error!("Failed to create parser client: {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "Failed to create parser client".to_string(),
                    }),
                )
            })?;
    let request = tonic::Request::new(api::ParserQueryRequest {
        query_id: product_id,
    });

    let response = client.get_parsed_content(request).await.map_err(|e| {
        error!("Failed to parse content: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "Failed to parse content".to_string(),
            }),
        )
    })?;

    let parsed_terms = response.into_inner().parsed_terms;
    Ok((Status::Ok, Json(parsed_terms)))
}

#[post("/task", data = "<task_id>")]
async fn get_task(
    task_id: Json<GetTask>,
    user: AuthUser,
    pool: &State<MongoPool>,
) -> Result<(Status, Json<Task>), (Status, Json<ErrorMessage>)> {
    get_task_by_id(&pool, &user.user_id, task_id.id)
        .await
        .map(|task| (Status::Ok, Json(task)))
        .map_err(|e| match e {
            MixMongoAndCustomError::Mongo(e) => {
                error!("Failed to get task side mongo: {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "can't get task side mongo".to_string(),
                    }),
                )
            }
            MixMongoAndCustomError::Custom(e) => {
                error!("Task does not exist: {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "task is not exist now".to_string(),
                    }),
                )
            }
        })
}

#[put("/task", data = "<data>")]
async fn edit_task_name(
    pool: &State<PostgresPool>,
    data: Json<EditTaskName>,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    update_task_name(pool, data.id, &data.new_name)
        .await
        .map_err(|e| {
            error!("Failed to update task name: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't update task name".to_string(),
                }),
            )
        })?;
    Ok(Status::Accepted)
}

#[get("/history")]
async fn get_history(
    user: AuthUser,
    pool: &State<PostgresPool>,
) -> Result<(Status, Json<History>), (Status, Json<ErrorMessage>)> {
    get_all_tasks(pool, &user.user_id)
        .await
        .map(|history| (Status::Ok, Json(history)))
        .map_err(|e| {
            error!("Failed to get history: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't get history".to_string(),
                }),
            )
        })
}

#[get("/account")]
async fn get_account(
    user: AuthUser,
    pool: &State<PostgresPool>,
) -> Result<Json<SendAccount>, (Status, Json<ErrorMessage>)> {
    if !update_check_session_time(pool, &user.id)
        .await
        .map_err(|e| {
            error!("Failed to update session time: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't update session time".to_string(),
                }),
            )
        })?
    {
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    Ok(Json(get_account_info(pool, &user.user_id).await.map_err(
        |e| match e {
            MixPostgresAndCustomError::Postgres(e) => {
                error!("Failed to get account side postgres: {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "can't get account side postgres".to_string(),
                    }),
                )
            }
            MixPostgresAndCustomError::Custom(e) => {
                error!("Failed to get account side mongo: {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "can't get account side mongo".to_string(),
                    }),
                )
            }
        },
    )?))
}

#[delete("/session", data = "<session_id>")]
async fn delete_session(
    session_id: Json<GetTask>,
    user: AuthUser,
    pool: &State<PostgresPool>,
) -> Result<(), (Status, Json<ErrorMessage>)> {
    if !update_check_session_time(pool, &user.id)
        .await
        .map_err(|e| {
            error!("Failed to update session time: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't update session time".to_string(),
                }),
            )
        })?
    {
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    if user.id == session_id.id {
        return Err((
            Status::BadRequest,
            Json(ErrorMessage {
                message: "you can't delete your own session".to_string(),
            }),
        ));
    }
    delete_client_session(pool, session_id.id)
        .await
        .map_err(|e| {
            error!("Failed to delete client session: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't get sessions".to_string(),
                }),
            )
        })?;
    Ok(())
}

#[post("/exit")]
async fn exit(
    user: AuthUser,
    pool: &State<PostgresPool>,
    cookies: &CookieJar<'_>,
) -> Result<(), (Status, Json<ErrorMessage>)> {
    if !check_client_session_id(pool, &user.id).await.map_err(|e| {
        error!("Failed to check client session ID: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't update session time".to_string(),
            }),
        )
    })? {
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    delete_client_session(pool, user.id).await.map_err(|e| {
        error!("Failed to delete client session: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't get sessions".to_string(),
            }),
        )
    })?;
    cookies.remove_private("refresh_token");
    Ok(())
}

#[get("/information?<id>")]
pub async fn information<'a>(
    id: String,
    stream: &'a State<NatsStream>,
) -> Result<TextStream![String], (Status, Json<ErrorMessage>)> {
    let norm_id = match uuid::Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to parse UUID: {}", e);
            return Err((
                Status::BadRequest,
                Json(ErrorMessage {
                    message: "invalid id".to_string(),
                }),
            ));
        }
    };
    let mut messages = get_messages_stream(stream, norm_id).await.map_err(|e| {
        error!("Failed to create consumer: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can`t create consumer".to_string(),
            }),
        )
    })?;

    Ok(TextStream! {
        yield serde_json::to_string(&SendMessage{ message: "start".to_string(), task_type: "system".to_string() }).unwrap()+"\n\n";
        let mut text_is_end = false;
        let mut reviews_is_end = false;
        let mut photo_is_end = false;
        while let Some(message) = messages.next().await{
            let message = match message{
                Ok(message) => message,
                Err(e) => {
                    log::error!("Failed to receive message: {}", e);
                    continue;
                }
            };
            message.ack().await.unwrap();

            let text = match from_utf8(&message.payload){
                Ok(s) => s,
                Err(_) => todo!(),
            };
            let nats_message: TaskMessage = match serde_json::from_str(text) {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!("Ошибка парсинга JSON: {}", e);
                    yield format!("Ошибка: невалидный JSON в сообщении");
                    continue;
                }
            };
            if nats_message.message == "__end__" {
                if nats_message.task_type == "text"{
                    text_is_end = true;
                }else if nats_message.task_type == "reviews"{
                    reviews_is_end = true;
                }else if nats_message.task_type == "photo"{
                    photo_is_end = true;
                }
            }
            yield serde_json::to_string(&SendMessage{ message: nats_message.message, task_type: nats_message.task_type }).unwrap()+"\n\n";
            if text_is_end && reviews_is_end && photo_is_end {
                yield serde_json::to_string(&SendMessage{ message: "done".to_string(), task_type: "system".to_string() }).unwrap()+"\n\n";
                break;
            }

        }
    })
}

#[post("/task", data = "<data>")]
pub async fn add_information_by_task<'a>(
    pool: &'a State<MongoPool>,
    user: AuthUser,
    data: Json<InformationTask>,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    if !(data.task_type == "photo" || data.task_type == "reviews" || data.task_type == "text") {
        return Err((
            Status::BadRequest,
            Json(ErrorMessage {
                message: "your type is not exist".to_string(),
            }),
        ));
    }
    if data.task_type == "photo" {
        set_photo_analysis(pool, &user.user_id, data.id, &data.message)
            .await
            .map_err(|e| {
                error!("Mongo is not connected. Error {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "internal service is not online please wait".to_string(),
                    }),
                )
            })?;
        Ok(Status::Ok)
    } else if data.task_type == "reviews" {
        set_review_analysis(pool, &user.user_id, data.id, &data.message)
            .await
            .map_err(|e| {
                error!("Mongo is not connected. Error {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "internal service is not online please wait".to_string(),
                    }),
                )
            })?;
        Ok(Status::Ok)
    } else if data.task_type == "text" {
        set_text_analysis(pool, &user.user_id, data.id, &data.message)
            .await
            .map_err(|e| {
                error!("Mongo is not connected. Error {}", e);
                (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "internal service is not online please wait".to_string(),
                    }),
                )
            })?;
        Ok(Status::Ok)
    } else {
        Err((
            Status::BadRequest,
            Json(ErrorMessage {
                message: "your type is not exist".to_string(),
            }),
        ))
    }
}

#[get("/users")]
pub async fn all_users(
    pool: &State<PostgresPool>,
    user: AuthUser,
) -> Result<Json<Vec<SendUser>>, (Status, Json<ErrorMessage>)> {
    let is_admin_result = is_admin(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check admin status: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't check admin status".to_string(),
            }),
        )
    })?;
    if !is_admin_result {
        return Err((
            Status::Forbidden,
            Json(ErrorMessage {
                message: "you are not admin".to_string(),
            }),
        ));
    }
    let all_user = get_all_users(pool).await.map_err(|e| {
        error!("Failed to get all users: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't get all users".to_string(),
            }),
        )
    })?;
    let all_user_to_json = all_user
        .into_iter()
        .map(|user| SendUser {
            name: user.name,
            email: user.email,
            subscription: Subscribtion {
                created_at: user.created_at,
                expires_at: user.valid_to,
            },
            id: user.id,
            is_admin: user.is_admin,
        })
        .collect::<Vec<SendUser>>();
    Ok(Json(all_user_to_json))
}

#[get("/admin")]
pub async fn check_is_admin(
    pool: &State<PostgresPool>,
    user: AuthUser,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    let is_admin = is_admin(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check admin status: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't check admin status".to_string(),
            }),
        )
    })?;
    if !is_admin {
        return Ok(Status::Forbidden);
    }
    Ok(Status::Ok)
}

#[post("/admin", data = "<data>")]
pub async fn change_admin(
    pool: &State<PostgresPool>,
    user: AuthUser,
    data: Json<ChangeToAdminData>,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    let is_admin = is_admin(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check admin status: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't check admin status".to_string(),
            }),
        )
    })?;
    if !is_admin {
        return Ok(Status::Forbidden);
    }
    if user.user_id == data.user_id {
        return Err((
            Status::BadRequest,
            Json(ErrorMessage {
                message: "you can't change your own admin status".to_string(),
            }),
        ));
    }
    set_admin(pool, &data.user_id, data.is_admin)
        .await
        .map_err(|e| {
            error!("Failed to set admin status: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't set admin status".to_string(),
                }),
            )
        })?;
    Ok(Status::Ok)
}

#[post("/subscribe", data = "<data>")]
pub async fn add_subscribe(
    pool: &State<PostgresPool>,
    user: AuthUser,
    data: Json<CreateSubscribe>,
) -> Result<Status, (Status, Json<ErrorMessage>)> {
    if !update_check_session_time(pool, &user.id)
        .await
        .map_err(|e| {
            error!("Failed to update session time: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't update session time".to_string(),
                }),
            )
        })?
    {
        return Err((
            Status::Unauthorized,
            Json(ErrorMessage {
                message: "invalid refresh token. Your session is not avalable.".to_string(),
            }),
        ));
    }
    let is_admin = is_admin(pool, &user.user_id).await.map_err(|e| {
        error!("Failed to check admin status: {}", e);
        (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't check admin status".to_string(),
            }),
        )
    })?;
    if !is_admin {
        return Ok(Status::Forbidden);
    }
    create_subscribe(pool, &data.user_id, data.created_at, data.valid_to)
        .await
        .map_err(|e| {
            error!("Failed to create subscription: {}", e);
            (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't create new user".to_string(),
                }),
            )
        })?;
    Ok(Status::Created)
}

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    let log_level = log::LevelFilter::from_str(
        std::env::var("RUST_LOG_LEVEL")
            .unwrap_or("info".to_string())
            .as_str(),
    )
    .unwrap_or(log::LevelFilter::Info);
    femme::with_level(log_level);
    let secret_key = SecretKey::from(
        &dotenvy::var("SECRET_KEY_COOKIE")
            .expect("Failed to load secret key from environment variable")
            .as_bytes(),
    );
    let host = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let config = rocket::Config {
        address: host,
        secret_key,
        ..rocket::Config::debug_default()
    };
    let allowed_origins = AllowedOrigins::some_exact(&[
        &std::env::var("URL_CORS").unwrap_or("http://localhost:3000".to_string())
    ]);
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post, Method::Put, Method::Delete]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .unwrap();
    rocket::build()
        .configure(config)
        .attach(AdHoc::on_ignite("Postgres", |rocket| async move {
            init_postgre_pools(rocket).await
        }))
        .attach(AdHoc::on_ignite("MongoDB", |rocket| async move {
            init_mongo_pools(rocket).await
        }))
        .attach(AdHoc::on_ignite("RabbitMQ", |rocket| async move {
            init_rabbit_queues(rocket).await
        }))
        .attach(AdHoc::on_ignite("Nats", |rocket| async move {
            init_nats_stream(rocket).await
        }))
        .mount("/api/v1", routes![information,])
        .mount(
            "/api/v1/auth",
            routes![authorization, registration, exit, refresh],
        )
        .mount("/api/v1/check", routes![check_is_admin])
        .mount(
            "/api/v1/get",
            routes![
                get_words_from_url,
                get_history,
                get_task,
                get_account,
                all_users
            ],
        )
        .mount("/api/v1/create", routes![create_task])
        .mount("/api/v1/edit", routes![edit_task_name, change_admin])
        .mount("/api/v1/regenerate", routes![edit_task])
        .mount("/api/v1/delete", routes![delete_session, delete_task])
        .mount(
            "/api/v1/add",
            routes![add_information_by_task, add_subscribe],
        )
        .attach(cors)
}
