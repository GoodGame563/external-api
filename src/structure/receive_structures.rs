use serde::Deserialize;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Enter {
    pub email: String,
    pub password: String,
    pub browser: String,
    pub device: String,
    pub os: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Registration {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RefreshTokenStructure {
    pub browser: String,
    pub device: String,
    pub os: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Review {
    pub text: String,
    pub pros: String,
    pub cons: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Product {
    pub description: String,
    pub id: u64,
    pub name: String,
    pub price: u64,
    #[serde(rename = "reviewRating")]
    pub review_rating: f32,
    pub image_url: String,
    pub root: u64,
    pub reviews: Vec<Review>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MainProduct {
    pub description: String,
    pub id: u64,
    pub name: String,
    pub image_url: String,
    pub root: u64,
    pub reviews: Vec<Review>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateTask {
    pub products: Vec<Product>,
    pub main: MainProduct,
    pub used_words: Vec<String>,
    pub unused_words: Vec<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct DeleteTask {
    pub id: uuid::Uuid,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct EditTask {
    pub id: uuid::Uuid,
    pub products: Vec<Product>,
    pub main: MainProduct,
    pub used_words: Vec<String>,
    pub unused_words: Vec<String>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct EditTaskName {
    pub id: uuid::Uuid,
    #[serde(rename = "newName")]
    pub new_name: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct GetTask {
    pub id: uuid::Uuid,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TaskMessage {
    pub message: String,
    pub task_type: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct InformationTask {
    pub id: uuid::Uuid,
    #[serde(rename = "taskType")]
    pub task_type: String,
    pub message: String,
}
