use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub description: String, 
    pub brand_id: i32,
    pub price: f64,
    pub rating: f32,
    pub brand: String
}

#[derive(Debug, Deserialize)]
struct Response {
    data: Data,
}

#[derive(Debug, Deserialize)]
struct Data {
    products: Vec<ProductData>,
}

#[derive(Debug, Deserialize)]
struct ProductData {
    id: i32,
    name: String,
    brand: String,
    #[serde(rename = "brandId")]
    brand_id: i32,
    rating: f32,
    sizes: Vec<ProductSize>,
}

#[derive(Debug, Deserialize)]
struct ProductSize {
    #[serde(rename = "optionId")]
    option_id: i32,
    price: Option<ProductPrice>,
}

#[derive(Debug, Deserialize)]
struct ProductPrice {
    product: i32,
}

impl Product {
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let response: Response = serde_json::from_str(json_str)?;
        
        let product_data = &response.data.products[0];
        
        let price = product_data.sizes
            .iter()
            .find(|s| s.option_id == product_data.id)
            .and_then(|s| s.price.as_ref())
            .map_or(0.0, |p| p.product as f64 / 100.0);

        Ok(Product {
            id: product_data.id,
            name: product_data.name.clone(),
            description: String::new(), 
            brand_id: product_data.brand_id,
            price,
            rating: product_data.rating,
            brand: product_data.brand.clone()
        })
    }
}