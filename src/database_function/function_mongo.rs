use crate::database_function::connection_mongo::Pool;
use crate::database_function::connection_mongo::PoolError;
use mongodb::{
    bson::{doc, Binary, DateTime},
    Client, Collection,
};
use uuid::Uuid;

use serde::{Deserialize, Serialize};

const DB_NAME: &str = "ai_tasks";

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub root: u64,
    pub price: u64,
    pub review: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WordsAnalysis {
    pub used_words: Vec<String>,
    pub unused_words: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductAnalysis {
    #[serde(rename = "_id")]
    pub id: Uuid,
    pub created_at: DateTime,
    pub main_product: Product,
    pub competitors: Vec<Product>,
    pub words_analysis: WordsAnalysis,
    pub text_analyses: Option<String>,
    pub photo_analysis: Option<String>,
    pub review_analysis: Option<String>,
}

pub async fn check_and_create_db(client: &Client) -> Result<(), mongodb::error::Error> {
    let db_names = client.list_database_names().await?;
    if !db_names.contains(&DB_NAME.to_string()) {
        println!("Database 'ai_tasks' not found, creating...");
        client.database(DB_NAME).create_collection("tasks").await?;
    }
    Ok(())
}

pub async fn create_task<'a>(
    mongo_pool: &Pool,
    user_id: &str,
    id: Uuid,
    main_product: Product,
    competitors: Vec<Product>,
    used_words: Vec<String>,
    unused_words: Vec<String>,
) -> Result<(), PoolError> {
    let client = mongo_pool.get().await?;
    let collection: Collection<ProductAnalysis> = client.database(DB_NAME).collection(user_id);
    let task = ProductAnalysis {
        id,
        created_at: DateTime::now(),
        main_product,
        competitors,
        words_analysis: WordsAnalysis {
            used_words,
            unused_words,
        },
        text_analyses: None,
        photo_analysis: None,
        review_analysis: None,
    };
    collection
        .insert_one(task)
        .await
        .map_err(|e| PoolError::from(e))?;
    Ok(())
}

pub async fn get_task<'a>(
    mongo_pool: &'a Pool,
    user_id: &'a str,
    id: Uuid,
) -> Result<Option<ProductAnalysis>, PoolError> {
    let client = mongo_pool.get().await?;
    let collection: Collection<ProductAnalysis> = client.database(DB_NAME).collection(user_id);
    let uuid_bytes = id.as_bytes();
    let filter = doc! { "_id": Binary { subtype: mongodb::bson::spec::BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };

    let not_task = collection
        .find_one(filter)
        .await
        .map_err(|e| PoolError::from(e))?;
    let task = not_task.map(|doc| ProductAnalysis {
        id: doc.id,
        created_at: doc.created_at,
        main_product: doc.main_product,
        competitors: doc.competitors,
        words_analysis: doc.words_analysis,
        text_analyses: doc.text_analyses,
        photo_analysis: doc.photo_analysis,
        review_analysis: doc.review_analysis,
    });
    Ok(task)
}

pub async fn update_task<'a>(
    mongo_pool: &Pool,
    user_id: &str,
    id: Uuid,
    main_product: Product,
    competitors: Vec<Product>,
    used_words: Vec<String>,
    unused_words: Vec<String>,
) -> Result<(), PoolError> {
    let client = mongo_pool.get().await?;
    let collection: Collection<ProductAnalysis> = client.database(DB_NAME).collection(user_id);

    let uuid_bytes = id.as_bytes();
    let filter = doc! { "_id": Binary { subtype: mongodb::bson::spec::BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };
    let update = doc! {
        "$set": {
            "created_at": DateTime::now(),
            "main_product": bson::to_bson(&main_product).unwrap(),
            "competitors": bson::to_bson(&competitors).unwrap(),
            "words_analysis": {
                "used_words": used_words,
                "unused_words": unused_words
            }
        }
    };

    collection
        .update_one(filter, update)
        .await
        .map_err(|e| PoolError::from(e))?;
    Ok(())
}

pub async fn update_text_analysis<'a>(
    mongo_pool: &Pool,
    id: Uuid,
    user_id: &str,
    data: &str,
) -> Result<(), PoolError> {
    let client = mongo_pool.get().await?;
    let collection: Collection<ProductAnalysis> = client.database(DB_NAME).collection(user_id);

    let uuid_bytes = id.as_bytes();
    let filter = doc! { "_id": Binary { subtype: mongodb::bson::spec::BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };
    let update = doc! {
        "$set": {
            "text_analyses": data
        }
    };

    collection
        .update_one(filter, update)
        .await
        .map_err(|e| PoolError::from(e))?;
    Ok(())
}

pub async fn update_photo_analysis<'a>(
    mongo_pool: &Pool,
    id: Uuid,
    user_id: &str,
    data: &str,
) -> Result<(), PoolError> {
    let client = mongo_pool.get().await?;
    let collection: Collection<ProductAnalysis> = client.database(DB_NAME).collection(user_id);

    let uuid_bytes = id.as_bytes();
    let filter = doc! { "_id": Binary { subtype: mongodb::bson::spec::BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };
    let update = doc! {
        "$set": {
            "photo_analysis": data
        }
    };

    collection
        .update_one(filter, update)
        .await
        .map_err(|e| PoolError::from(e))?;
    Ok(())
}

pub async fn update_review_analysis<'a>(
    mongo_pool: &Pool,
    id: Uuid,
    user_id: &str,
    data: &str,
) -> Result<(), PoolError> {
    let client = mongo_pool.get().await?;
    let collection: Collection<ProductAnalysis> = client.database(DB_NAME).collection(user_id);

    let uuid_bytes = id.as_bytes();
    let filter = doc! { "_id": Binary { subtype: mongodb::bson::spec::BinarySubtype::Generic, bytes: uuid_bytes.to_vec() } };
    let update = doc! {
        "$set": {
            "review_analysis": data
        }
    };

    collection
        .update_one(filter, update)
        .await
        .map_err(|e| PoolError::from(e))?;
    Ok(())
}
