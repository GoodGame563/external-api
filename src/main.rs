mod structure;
mod jwt;
mod database_function;

// External crates
#[macro_use] 
extern crate rocket;

// Standard imports
use chrono::Duration;
use dotenvy::dotenv;
use sha2::{Sha512, Digest};

// Rocket imports
use rocket::{
    fairing::{AdHoc, Fairing, Info, Kind}, 
    State,
    http::{Status, Header, Method},
    request::{FromRequest, Outcome, Request},
    serde::json::Json,
    Response,
    response::stream::TextStream,
};

// Project imports
use api::parser_integration_service_client::ParserIntegrationServiceClient;
use crate::jwt::{create_access_jwt, create_refresh_jwt};
use structure::send_structures::{ErrorMessage, Token, Tokens, History, Task};
use structure::receive_structures::{CreateTask, EditTaskName, GetTask, EditTask};
use database_function::{
    function_postgre::{User, UserSession},
    init_postgre_pools,
    init_mongo_pools,
    MixPoolError,
    get_all_tasks,
    update_task_name,
    get_task_by_id,
};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use database_function::connection_mongo::Pool as MongoPool;
use deadpool_postgres::Pool as PostgresPool;

// Proto module
mod api {
    tonic::include_proto!("api");
}

#[derive(Debug)]
pub struct AuthUser {
    pub user_id: String,
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
            Ok(token_data) => Outcome::Success(AuthUser { user_id: token_data.user_id }),
            Err(e) => Outcome::Error((Status::Unauthorized, format!("Invalid token: {}", e))),
        }
    }
}

fn hash_str(path: &str) -> Result<String, std::io::Error> {
    let mut hasher = Sha512::new();
    hasher.update(path.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

#[post("/authorization", data = "<data>")]
async fn authorization(pool: &State<PostgresPool>, data: Json<crate::structure::receive_structures::Enter>) -> Result<(Status, Json<Tokens>), (Status, Json<ErrorMessage>)> {
    let access_life_time: chrono::TimeDelta = Duration::days(2);
    let refresh_life_time = Duration::weeks(2);

    let hash_id = hash_str(&format!("{}{}", data.email, data.password))
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "hash error".to_string(),
                details: e.to_string(),
            })
        ))?;

    let user = User::find_by_id(pool, &hash_id).await
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't find".to_string(),
                details: e.to_string(),
            })
        ))?
        .ok_or((
            Status::NotFound,
            Json(ErrorMessage {
                message: "user not found".to_string(),
                details: "doesn't exist".to_string(),
            })
        ))?;

    let access_token = create_access_jwt(&user.id, access_life_time)
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create access jwt".to_string(),
                details: e.to_string(),
            })
        ))
        .map(|token| Token {
            token,
            life_time: access_life_time,
        })?;

    let refresh_token = create_refresh_jwt(&user.id, &data.browser, &data.device, &data.os, refresh_life_time)
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create refresh jwt".to_string(),
                details: e.to_string(),
            })
        ))
        .map(|token| Token {
            token,
            life_time: refresh_life_time,
        })?;

    database_function::create_client_session(pool, &user.id, &data.browser, &data.device, &data.os).await
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create client session".to_string(),
                details: e.to_string(),
            })
        ))
        .and_then(|created| {
            Ok((Status::Ok, Json(Tokens { access_token, refresh_token })))
        })
}

#[post("/registration", data = "<data>")]
async fn registration(pool: &State<PostgresPool>, data: Json<crate::structure::receive_structures::Registration>) -> Result<Status, (Status, Json<ErrorMessage>)> {
    let hash_id = hash_str(&format!("{}{}", data.email, data.password))
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "hash error".to_string(),
                details: e.to_string(),
            })
        ))?;

    match User::find_by_email(pool, &data.email).await {
        Ok(true) => Err((
            Status::Conflict,
            Json(ErrorMessage {
                message: "user with this email already exisist".to_string(),
                details: "please use function enter".to_string(),
            })
        )),
        Ok(false) => {
            let user = User {
                id: hash_id,
                email: data.email.to_string(),
                name: data.name.to_string(),
                is_admin: false
            };
            User::create(&user, pool).await
                .map(|_| Status::Created)
                .map_err(|e| (
                    Status::InternalServerError,
                    Json(ErrorMessage {
                        message: "can't create new user".to_string(),
                        details: e.to_string(),
                    })
                ))
        },
        Err(e) => Err((
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "database not available".to_string(),
                details: e.to_string(),
            })
        )),
    }
}

#[post("/task", data = "<data>")]
async fn create_task(pool: &State<PostgresPool>, mongo_pool: &State<MongoPool>, data: Json<CreateTask>, user: AuthUser) -> Result<Status, (Status, Json<ErrorMessage>)> {
    database_function::create_task(
        pool, 
        mongo_pool, 
        &user.user_id, 
        &data.main.name, 
        &data.main, 
        &data.products, 
        data.used_words.iter().map(String::as_str).collect::<Vec<&str>>(), 
        data.unused_words.iter().map(String::as_str).collect::<Vec<&str>>()
    ).await
    .map(|_| Status::Created)
    .map_err(|e| match e {
        MixPoolError::Postgres(e) => (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create task side postgres".to_string(),
                details: e.to_string(),
            })
        ),
        MixPoolError::Mongo(e) => (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create task side mongo".to_string(),
                details: e.to_string(),
            })
        ),
        MixPoolError::Custom(e) => (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create task side idk is not exists".to_string(),
                details: e.to_string(),
            })
        ),
    })
}

#[post("/task", data = "<data>")]
async fn edit_task(pool: &State<PostgresPool>, mongo_pool: &State<MongoPool>, data: Json<EditTask>, user: AuthUser) -> Result<Status, (Status, Json<ErrorMessage>)> {
    database_function::regenerate_task(
        pool, 
        mongo_pool, 
        &data.id,
        &user.user_id, 
        &data.main, 
        &data.products, 
        data.used_words.iter().map(String::as_str).collect::<Vec<&str>>(), 
        data.unused_words.iter().map(String::as_str).collect::<Vec<&str>>()
    ).await
    .map(|_| Status::Created)
    .map_err(|e| match e {
        MixPoolError::Postgres(e) => (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create task side postgres".to_string(),
                details: e.to_string(),
            })
        ),
        MixPoolError::Mongo(e) => (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create task side mongo".to_string(),
                details: e.to_string(),
            })
        ),
        MixPoolError::Custom(e) => (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't create task side idk is not exists".to_string(),
                details: e.to_string(),
            })
        ),
    })
}

#[get("/words/<product_id>")]
async fn get_words_from_url(product_id: i32) -> Result<(Status, Json<Vec<String>>), (Status, Json<ErrorMessage>)> {
    let mut client = ParserIntegrationServiceClient::connect("http://localhost:50051")
        .await
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "Failed to create parser client".to_string(),
                details: e.to_string(),
            })
        ))?;
    let request = tonic::Request::new(api::ParserQueryRequest { query_id: product_id });
    
    let response = client
        .get_parsed_content(request)
        .await
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "Failed to parse content".to_string(),
                details: e.to_string(),
            })
        ))?;

    let parsed_terms = response.into_inner().parsed_terms;
    Ok((Status::Ok, Json(parsed_terms)))
}

#[post("/task", data = "<task_id>")]
async fn get_task(task_id: Json<GetTask>, user: AuthUser, pool: &State<MongoPool>) -> Result<(Status, Json<Task>), (Status, Json<ErrorMessage>)> {
    get_task_by_id(&pool, &user.user_id, task_id.id).await
        .map(|task| {
            (Status::Ok, Json(task))
        })
        .map_err(|e| match e {
            MixPoolError::Postgres(e) => (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't get task side postgres is not work".to_string(),
                    details: e.to_string(),
                })
            ),
            MixPoolError::Mongo(e) => (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "can't get task side mongo".to_string(),
                    details: e.to_string(),
                })
            ),
            MixPoolError::Custom(e) => (
                Status::InternalServerError,
                Json(ErrorMessage {
                    message: "task is not exist now".to_string(),
                    details: e.to_string(),
                })
            ),
        }
    )
}
#[put("/task", data = "<data>")]
async fn edit_task_name(pool: &State<PostgresPool>, data:Json<EditTaskName>) -> Result<Status, (Status, Json<ErrorMessage>)> {
    update_task_name(pool, data.id, &data.new_name).await.map_err(|e| (
        Status::InternalServerError,
        Json(ErrorMessage {
            message: "can't update task name".to_string(),
            details: e.to_string(),
        })
    ))?;
    Ok(Status::Accepted)
}

#[get("/history")]
async fn get_history(user: AuthUser, pool: &State<PostgresPool>) -> Result<(Status, Json<History>), (Status, Json<ErrorMessage>)> {
    get_all_tasks(pool, &user.user_id).await
        .map(|history| (Status::Ok, Json(history)))
        .map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "can't get history".to_string(),
                details: e.to_string(),
            })
        ))
}

// #[get("/infinite-hellos")]
// async fn hello() -> TextStream![&'static str] {
//     TextStream! {
//         let mut interval = interval(tokio_duration::from_secs(1));
//         loop {
//             yield "hello";
//             interval.tick().await;
//         }
//     }
// }

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    let allowed_origins = AllowedOrigins::some_exact(&["http://localhost:3000"]);

    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post, Method::Put].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors().unwrap();
    
    rocket::build()
        .attach(AdHoc::on_ignite("Postgres", |rocket| async move {
            init_postgre_pools(rocket).await
        }))
        .attach(AdHoc::on_ignite("MongoDB", |rocket| async move {
            init_mongo_pools(rocket).await
        }))
        .mount("/", routes![authorization, registration])
        .mount("/get", routes![get_words_from_url, get_history, get_task])
        .mount("/create", routes![create_task])
        .mount("/edit", routes![edit_task_name])
        .mount("/regenerate", routes![edit_task])
        .attach(cors)
}