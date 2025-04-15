#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RefreshTokenPayload {
    id: String,
    browser: String,
    device: String,
    os: String,
    exp: usize,
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AccessTokenPayload {
    pub user_id: String,
    exp: usize,
}

#[derive(serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RefreshToken<'r> {
    id: &'r str,
    browser: &'r str,
    device: &'r str,
    os: &'r str,
    exp: usize,
}

#[derive(serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AccessToken<'r> {
    user_id: &'r str,
    exp: usize,
}

impl RefreshToken<'_> {
    pub fn new<'a>(
        id: &'a str,
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
    pub fn new<'a>(user_id: &'a str, exp: usize) -> AccessToken<'a> {
        AccessToken { user_id, exp }
    }
}