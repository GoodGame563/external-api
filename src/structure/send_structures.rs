use chrono::{TimeDelta, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub token: String,
    pub life_time: TimeDelta,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub access_token: Token,
    pub refresh_token_life_time: TimeDelta,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsedWord {
    pub id_word: i64,
    pub word: String,
    pub used: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestMessage {
    pub id_tasks: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAnswer {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub updated: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: u64,
    pub root: u64,
    pub name: String,
    pub brand: String,
    pub price: f64,
    pub review_rating: f64,
    pub description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub main: Product,
    pub products: Vec<Product>,
    pub used_words: Vec<String>,
    pub unused_words: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryElement {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub elements: Vec<HistoryElement>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSession {
    pub id: Uuid,
    pub browser: String,
    pub last_activity: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAccount {
    pub name: String,
    pub email: String,
    pub sessions: Vec<SendSession>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskId {
    pub id: Uuid,
}

#[derive(Serialize)]
pub struct SendMessage {
    pub message: String,
    pub task_type: String,
}
