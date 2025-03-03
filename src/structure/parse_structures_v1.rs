use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Price {
    basic: u32,
    product: u32,
    pub total: u32,
    logistics: u32,
    r#return: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Size {
    pub name: String,
    pub price: Price,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub root: u64,
    pub brand: String,
    #[serde(rename = "brandId")]
    pub brand_id: u32,
    pub name: String,
    pub rating: f32,
    #[serde(rename = "reviewRating")]
    pub review_rating: f32,
    pub sizes: Vec<Size>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub products: Vec<Product>,
    total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Root {
    pub data: Data,
}

