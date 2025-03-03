#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Enter<'r> {
    pub email: &'r str,
    pub password: &'r str, 
    pub browser: &'r str,
    pub device: &'r str,
    pub os: &'r str,
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Registration<'r> {
    pub email: &'r str,
    pub password: &'r str, 
    pub name: &'r str
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateTask<'r> {
    pub url: &'r str
}



