#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Enter {
    pub email: String,
    pub password: String, 
    pub browser: String,
    pub device: String,
    pub os: String,
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Registration {
    pub email: String,
    pub password: String, 
    pub name: String
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Product {
    pub brand: String,
    pub description: String,
    pub id: u64,
    pub name: String,
    pub price: u64,
    #[serde(rename = "reviewRating")]
    pub review_rating: f32,
    pub root: u64,
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MainProduct {
    pub description: String,
    pub id: u64,
    pub name: String,
    pub root: u64,
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateTask {
    pub products: Vec<Product>,
    pub main: MainProduct,
    pub used_words: Vec<String>, 
    pub unused_words: Vec<String>
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct EditTask {
    pub id: uuid::Uuid,
    pub products: Vec<Product>,
    pub main: MainProduct,
    pub used_words: Vec<String>, 
    pub unused_words: Vec<String>
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct EditTaskName {
    pub id: uuid::Uuid,
    #[serde(rename = "newName")]
    pub new_name: String 
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GetTask {
    pub id: uuid::Uuid,
}