use uuid::Uuid;

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RefreshTokenPayload {
    pub id: Uuid,
    pub browser: String,
    pub device: String,
    pub os: String,
    #[serde(rename = "exp")]
    _exp: usize,
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AccessTokenPayload {
    pub user_id: String,
    pub id: Uuid,
    #[serde(rename = "exp")]
    _exp: usize,
}

#[derive(serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RefreshToken<'r> {
    id: &'r Uuid,
    browser: &'r str,
    device: &'r str,
    os: &'r str,
    exp: usize,
}

#[derive(serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AccessToken<'r> {
    id: &'r Uuid,
    user_id: &'r str,
    exp: usize,
}

impl RefreshToken<'_> {
    pub fn new<'a>(
        id: &'a Uuid,
        browser: &'a str,
        device: &'a str,
        os: &'a str,
        exp: usize,
    ) -> RefreshToken<'a> {
        RefreshToken {
            id,
            browser,
            device,
            os,
            exp,
        }
    }
}

impl AccessToken<'_> {
    pub fn new<'a>(id: &'a Uuid, user_id: &'a str, exp: usize) -> AccessToken<'a> {
        AccessToken { id, user_id, exp }
    }
}
