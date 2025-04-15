use chrono::Utc;
use uuid::Uuid;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub token: String,
    pub life_time: chrono::TimeDelta
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub access_token: Token,
    pub refresh_token: Token
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub message: String,
    pub details: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsedWord {
    pub id_word: i64,
    pub word: String,
    pub used: bool
}


#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestMessage {
    pub id_tasks: i64
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAnswer{
    pub id: i32, 
    pub name: String, 
    pub status: String,
    pub updated: chrono::DateTime<Utc>
}

#[derive(serde::Serialize)]
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


#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task{
    pub main: Product,
    pub products: Vec<Product>,
    pub used_words: Vec<String>,
    pub unused_words: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryElement{
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History{
    pub elements: Vec<HistoryElement>
}
