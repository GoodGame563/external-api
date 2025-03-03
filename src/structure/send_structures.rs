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
    pub id_tasks: i64,
    pub id_competision: Vec<i32>, 
    pub words: Vec<UsedWord>
}

