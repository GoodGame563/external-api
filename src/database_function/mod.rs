pub mod connector;
use deadpool_postgres::{Pool, PoolError};
use connector::{Brand, Keyword, Product, Task, TaskKeyword, UserSession, ProductCompetitor};
use crate::structure::parse_structures_v1::Product as ProductParseV1;
use crate::structure::parse_structures_v2::Product as ProductParseV2;
use chrono::{DateTime, Utc};


pub async fn check_client_session(pool: &Pool, id_user: &str, browser: &str, device:&str, os:&str) -> Result<Option<UserSession>, PoolError>{
    let users =  UserSession::find_by_user_id(pool, id_user).await?;
    for user in users{
        if user.browser == browser && user.device == device && user.os == os{
            return Ok(Some(user))
        }
    }
    Ok(None)
}


pub async fn create_client_session(pool: &Pool, user_session: &UserSession) -> Result<(), PoolError>{
    match check_client_session(pool, &user_session.id_user, &user_session.browser, &user_session.device, &user_session.os).await?{
        Some(_) => Ok(()),
        None => {
            UserSession::create(&user_session, pool).await?;
            Ok(())
        },
    }
}

pub struct WeightTaskAnswer{
    pub task_id: i32,
    pub used_words: Vec<(i32, String)>,
    pub unused_words: Vec<(i32, String)>
}

pub async fn create_full_weight_task(pool: &Pool, used_words: Vec<String>, unused_words: Vec<String>, roots: Vec<&ProductParseV1>, main_product: ProductParseV2, user_id: String) -> Result<WeightTaskAnswer, PoolError>{
    Brand{ 
        id: main_product.brand_id, 
        name: main_product.brand, 
        description: "rofl".to_string(), 
        created_at: chrono::offset::Utc::now() }.create(pool).await?;
    
    let main = Product{
        id: main_product.id,
        marketplace_id: 1,
        name: main_product.name,
        description: main_product.description,
        brand_id: main_product.brand_id,
        price: main_product.price,
        rating: main_product.rating as f64,
        url: format!("https://www.wildberries.ru/catalog/{}/detail.aspx?targetUrl=EX", main_product.id),
        created_at: chrono::offset::Utc::now() ,
        updated_at: chrono::offset::Utc::now(),
    };

    let main_id = main.create(pool).await?;
    let task_time = chrono::offset::Utc::now();
    let task_id = Task{
        id: 0,
        user_id: Some(user_id),
        product_id: Some(main_id),
        photo_analysis_id: None,
        reviews_analysis_id: None,
        seo_analysis_id: None,
        created_at: task_time,
    }.create(pool).await?;
    let mut used_words_in_table = Vec::new(); 

    for u_w in used_words{
        let keyword = Keyword::create(pool, &u_w).await?;
        used_words_in_table.push((keyword.id, keyword.keyword));
        TaskKeyword{ task_id, task_created_at: task_time, keyword_id: keyword.id, used_in_analysis: true }.create(pool).await?;
    }

    let mut unused_words_in_table = Vec::new(); 

    for u_w in unused_words{
        let keyword = Keyword::create(pool, &u_w).await?;
        unused_words_in_table.push((keyword.id, keyword.keyword));
        TaskKeyword{ task_id, task_created_at: task_time, keyword_id: keyword.id, used_in_analysis: false }.create(pool).await?;
    }

    for prod in roots{
        Brand{ 
            id: prod.brand_id as i32, 
            name: prod.brand.clone(), 
            description: "rofl".to_string(), 
            created_at: chrono::offset::Utc::now() }.create(pool).await?;

        let competitor_id = Product{
            id: prod.id as i32,
            marketplace_id: 1,
            name: prod.name.clone(),
            description: "rofl".to_string(),
            brand_id: prod.brand_id as i32,
            price: prod.sizes[0].price.total as f64 / 100.0,
            rating: prod.rating as f64,
            url:  format!("https://www.wildberries.ru/catalog/{}/detail.aspx?targetUrl=EX", prod.id),
            created_at: chrono::offset::Utc::now() ,
            updated_at: chrono::offset::Utc::now(),
        }.create(pool).await?;
        ProductCompetitor{
            product_id: main_id,
            competitor_id,
            created_at: chrono::offset::Utc::now(),
        }.create(pool).await?;
    }

    Ok(WeightTaskAnswer{task_id, used_words: used_words_in_table, unused_words: unused_words_in_table})

}