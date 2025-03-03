mod structure;
mod jwt;
mod database_function;
mod work_with_wb_api;

use crate::jwt::{create_access_jwt};

#[macro_use] extern crate rocket;
use jwt::create_refresh_jwt;
use dotenvy::dotenv;
use rocket::{
    fairing::AdHoc, 
    Build, 
    Rocket, 
    State,
    http::Status,
    request::{FromRequest, Outcome, Request},
    serde::json::Json,
};
use work_with_wb_api::{get_root_from_url, get_top_10_ids_with_products, get_product_from_url};
pub struct Cors;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::Response;
use serde::{Serialize,Deserialize};
use chrono::{Utc, Duration};
use sha2::digest::consts::False;
use std::str::FromStr;
use sha2::{Sha512, Digest};
use tokio_postgres::NoTls;
use structure::send_structures::{ErrorMessage, RequestMessage, Token, Tokens, UsedWord};
use database_function::{connector::{User, UserSession}, create_full_weight_task};
use deadpool_postgres::{Pool, ManagerConfig, RecyclingMethod, Runtime};
use api::{
    parser_integration_service_client::ParserIntegrationServiceClient,
    ParserQueryRequest,
};
// use chrono::prelude::*;

mod api {
    tonic::include_proto!("api");
}


fn from_url_get_id(url: &str) -> Option<i64> {
    let id = url.split('/')
        .skip_while(|&s| s != "catalog")
        .nth(1)?;

    if id.chars().all(|c| c.is_ascii_digit()) {
        Some(match id.parse::<i64>() {
            Ok(i) => i,
            Err(_) => return None,
        })
    } else {
        None
    }
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
        let token_data = match jwt::validate_access_jwt(token) {
            Ok(a) => a,
            Err(e) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    format!("Invalid token: {}", e),
                ))
            }
        };
        Outcome::Success(AuthUser {
            user_id: token_data.id,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
struct Config {
    pg: deadpool_postgres::Config,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}

async fn init_db_pool(rocket: Rocket<Build>) -> Rocket<Build> {
    let figment = rocket.figment();
    let pool_size: u32 = figment
        .extract_inner("databases.postgres.pool_size")
        .unwrap_or(20);

    let cfg = Config::from_env().unwrap();
    let mgr = deadpool_postgres::Manager::new(cfg.pg.get_pg_config().expect("Not find env file"), NoTls);
    ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };

    let pool = Pool::builder(mgr)
        .max_size(pool_size as usize)
        .runtime(Runtime::Tokio1)
        .build()
        .unwrap();

    rocket.manage(pool)
}

fn hash_str(path: &str) -> Result<String, std::io::Error> {
    let bytes = path.as_bytes();
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                record.level(),
                chrono::Local::now().format("%H:%M:%S"),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("server.log")?)
        .apply()?;
    Ok(())
}

#[post("/authorization", data = "<data>")]
async fn authorization(pool: &State<Pool>, data: Json<crate::structure::receive_structures::Enter<'_>>) -> (Status, Json<Result<Tokens, ErrorMessage>>) {
    let access_life_time: chrono::TimeDelta = Duration::days(2);
    let refresh_life_time = Duration::weeks(2);

    let hash_id = match hash_str(&format!("{}{}", data.email, data.password)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Hash error: {}", e);
            return (
                Status::InternalServerError,
                Json(Err(ErrorMessage {
                    message: "hash error".to_string(),
                    details: e.to_string(),
                })),
            );
        }
    };

    let user = match User::find_by_id(pool, &hash_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                Status::NotFound,
                Json(Err(ErrorMessage {
                    message: "user not found".to_string(),
                    details: "doesn't exist".to_string(),
                })),
            );
        }
        Err(e) => {
            return (
                Status::InternalServerError,
                Json(Err(ErrorMessage {
                    message: "can't find".to_string(),
                    details: e.to_string(),
                })),
            );
        }
    };

    let access_token = match create_access_jwt(&user.id, access_life_time) {
        Ok(a_t) => Token {
            token: a_t,
            life_time: access_life_time,
        },
        Err(e) => {
            return (
                Status::InternalServerError,
                Json(Err(ErrorMessage {
                    message: "can't create access jwt".to_string(),
                    details: e.to_string(),
                })),
            );
        }
    };

    let refresh_token = match create_refresh_jwt(&user.id, data.browser, data.device, data.os, refresh_life_time) {
        Ok(r_t) => Token {
            token: r_t,
            life_time: refresh_life_time,
        },
        Err(e) => {
            return (
                Status::InternalServerError,
                Json(Err(ErrorMessage {
                    message: "can't create refresh jwt".to_string(),
                    details: e.to_string(),
                })),
            );
        }
    };

    match database_function::create_client_session(pool, &UserSession{ id_user: user.id, browser: data.browser.to_string(), device: data.device.to_string(), os: data.os.to_string()}).await{
        Ok(_) => (Status::Ok, Json(Ok(Tokens{ access_token, refresh_token}))),
        Err(e) => (
            Status::InternalServerError, 
            Json(Err(ErrorMessage {
                message: "can't create client session".to_string(),
                details: e.to_string(),
            }))
        ),
    }
}

#[post("/registration", data = "<data>")]
async fn registration(pool: &State<Pool>, data: Json<crate::structure::receive_structures::Registration<'_>>) -> (Status, Json<Result<(), ErrorMessage>>) {
    let hash_id = match hash_str(&format!("{}{}", data.email, data.password)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Hash error: {}", e);
            return (
                Status::InternalServerError,
                Json(Err(ErrorMessage {
                    message: "hash error".to_string(),
                    details: e.to_string(),
                })),
            );
        }
    };
    
    match User::find_by_email(pool, data.email).await {
        Ok(Some(_)) => (
            Status::Conflict, 
            Json(Err(ErrorMessage {
                message: "user with this email already exisist".to_string(),
                details: "please use function enter".to_string(),
            }))
        ),
        Ok(None) => {
            let user= User { id: hash_id, email: data.email.to_string(), name: data.name.to_string(), is_paid: false, is_admin: false };
            match User::create(&user, pool).await{
                Ok(_) => (
                    Status::Created, 
                    Json(Ok(()))
                ),
                Err(e) =>  (
                    Status::InternalServerError,
                    Json(Err(ErrorMessage {
                        message: "can`t create new user".to_string(),
                        details: e.to_string(),
                    })),
                ),
            }
        },
        Err(e) => (
            Status::InternalServerError,
            Json(Err(ErrorMessage {
                message: "database not available".to_string(),
                details: e.to_string(),
            })),
        ),
    }
}

#[post("/task", data = "<data>")]
async fn create_task(pool: &State<Pool>, data: Json<crate::structure::receive_structures::CreateTask<'_>>, user: AuthUser) -> Result<(Status, Json<RequestMessage>),(Status, Json<ErrorMessage>)> {
    let mut client = ParserIntegrationServiceClient::connect("http://localhost:50051").await.map_err(|e| (
        Status::InternalServerError,
        Json(ErrorMessage {
            message: "grpc client error".to_string(),
            details: e.to_string(),
        }),
    ))?;
    let id = match from_url_get_id(data.url) {
        Some(i) => i,
        None => return Err((
            Status::BadRequest,
            Json(ErrorMessage {
                message: "Url not work".to_string(),
                details: "".to_string(),
            }),
        )),
    }; 
    let request = tonic::Request::new(ParserQueryRequest { query_id: id });
    let response = client.get_parsed_content(request).await.map_err(|e| (
        Status::InternalServerError,
        Json(ErrorMessage {
            message: "grpc server return".to_string(),
            details: e.to_string(),
        }),
    ))?;

    let mut words:Vec<String> = response.into_inner().parsed_terms;
    let second_part = words.split_off(10);
    let first_part = words;
    let mut tasks = Vec::new();
    for word in second_part.clone(){
        tasks.push(tokio::spawn(async move {
            let result = get_root_from_url(&word).await;
            result
        }));
    }
    let results = futures::future::join_all(tasks).await;
    let mut roots= Vec::new();
    for r in results {
        let root = r.map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "futures error v1".to_string(),
                details: e.to_string(),
            }),
        ))?.map_err(|e| (
            Status::InternalServerError,
            Json(ErrorMessage {
                message: "serialize error".to_string(),
                details: e.to_string(),
            }),
        ))?;
        roots.push(root);
    }
    let processed = get_top_10_ids_with_products(&roots);
    let mut competitor_product= Vec::new();
    let mut ids_competisions = Vec::new();
    for pr in processed{
        competitor_product.push(pr.1);
        ids_competisions.push(pr.0 as i32);
    }
    let main_product = get_product_from_url(id as u32).await.map_err(|e| (
        Status::InternalServerError,
        Json(ErrorMessage {
            message: "futures error v2".to_string(),
            details: e.to_string(),
        }),
    ))?;
    let task  = create_full_weight_task(pool, second_part, first_part, competitor_product, main_product, user.user_id).await.map_err(|e| (
        Status::InternalServerError,
        Json(ErrorMessage {
            message: "futures error v3".to_string(),
            details: e.to_string(),
        }),
    ))?;
    let mut words = Vec::new();
    for u_w in task.used_words{
        words.push(UsedWord{ id_word: u_w.0 as i64, word: u_w.1, used: true });
    }
    for u_w in task.unused_words{
        words.push(UsedWord{ id_word: u_w.0 as i64, word: u_w.1, used: false });
    }
    return Ok((Status::Ok, 
    Json(RequestMessage{ id_tasks: task.task_id as i64, id_competision: ids_competisions, words: words } 
    )));
}

#[options("/<_..>")]
fn everything() -> Status {
    Status::Ok
}

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Cross-Origin-Resource-Sharing Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "http://localhost:5501"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, PATCH, PUT, DELETE, HEAD, OPTIONS, GET",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
#[launch]
fn rocket() -> _ {
    dotenv().ok();
    setup_logger().unwrap();
    rocket::build().attach(Cors).attach(AdHoc::on_ignite("Postgres", |rocket| async move {
        init_db_pool(rocket).await
    })).mount("/", routes![authorization, registration, everything])
    .mount("/create", routes![create_task])
}