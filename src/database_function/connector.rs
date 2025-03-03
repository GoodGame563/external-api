use chrono::{DateTime, Utc};
use deadpool_postgres::{Pool, PoolError};
use deadpool_postgres::tokio_postgres::Row;


#[derive(Debug)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub is_paid: bool,
    pub is_admin: bool,
}

impl User {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO users (id, email, is_paid, is_admin) VALUES ($1, $2, $3, $4)",
            &[&self.id, &self.email, &self.is_paid, &self.is_admin],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_id(pool: &Pool, id: &str) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM users WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: &str) -> Result<Option<Self>, PoolError>{
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, email, name, is_paid, is_admin FROM users WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    pub async fn find_by_email(pool: &Pool, email: &str) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, email, name, is_paid, is_admin FROM users WHERE email = $1",
            &[&email]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            name: row.get("name"),
            is_paid: row.get("is_paid"),
            is_admin: row.get("is_admin"),
        })
    }
}

#[derive(Debug)]
pub struct UserSession {
    pub id_user: String,
    pub browser: String,
    pub device: String,
    pub os: String,
}

impl UserSession {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO user_session (id_user, browser, device, os) VALUES ($1, $2, $3, $4)",
            &[&self.id_user, &self.browser, &self.device, &self.os],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_user_id(pool: &Pool, id_user: &str) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM user_session WHERE id_user = $1", &[&id_user]).await?;
        Ok(())
    }

    pub async fn find_by_user_id(pool: &Pool, id_user: &str) -> Result<Vec<Self>, PoolError> {
        let client = pool.get().await?;
        client.query(
            "SELECT id_user, browser, device, os FROM user_session WHERE id_user = $1",
            &[&id_user]
        ).await?.into_iter().map(Self::from_row).collect()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(UserSession {
            id_user: row.get("id_user"),
            browser: row.get("browser"),
            device: row.get("device"),
            os: row.get("os"),
        })
    }
}

#[derive(Debug)]
pub struct Brand {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

impl Brand {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO brands (id, name, description) VALUES ($1, $2, $3) on conflict DO NOTHING ",
            &[&self.id, &self.name, &self.description],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM brands WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, name, description, created_at FROM brands WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(Brand {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
        })
    }
}

#[derive(Debug)]
pub struct Marketplace {
    pub id: i32,
    pub name: String,
    pub base_url: String,
    pub created_at: DateTime<Utc>,
}

impl Marketplace {
    pub async fn create(&self, pool: &Pool) -> Result<(),PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO marketplaces (name, base_url) VALUES ($1, $2)",
            &[&self.name, &self.base_url],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(),PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM marketplaces WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>,PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, name, base_url, created_at FROM marketplaces WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self,PoolError> {
        Ok(Marketplace {
            id: row.get("id"),
            name: row.get("name"),
            base_url: row.get("base_url"),
            created_at: row.get("created_at"),
        })
    }
}
#[derive(Debug)]
pub struct Product {
    pub id: i32,
    pub marketplace_id: i32,
    pub name: String,
    pub description: String,
    pub brand_id: i32,
    pub price: f64,
    pub rating: f64,
    pub url: String,
    pub created_at:DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    pub async fn create(&self, pool: &Pool) -> Result<i32, PoolError> {
        let client = pool.get().await?;
        let row = client.query_one(
            "INSERT INTO products (marketplace_id, name, description, brand_id, price, rating, url) 
            VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
            &[
                &self.marketplace_id,
                &self.name,
                &self.description,
                &self.brand_id,
                &self.price,
                &self.rating,
                &self.url,
            ],
        ).await?;
        let id: i32 = row.get(0);
        Ok(id)
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM products WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, marketplace_id, sku, name, description, brand_id, price, rating, 
            url, status, created_at, updated_at FROM products WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(Product {
            id: row.get("id"),
            marketplace_id: row.get("marketplace_id"),
            name: row.get("name"),
            description: row.get("description"),
            brand_id: row.get("brand_id"),
            price: row.get("price"),
            rating: row.get("rating"),
            url: row.get("url"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[derive(Debug)]
pub struct PhotoAnalysis {
    pub id: i32,
    pub product_id: i32,
    pub status: String,
    pub error: String,
    pub message: String,
    pub analysis_date: DateTime<Utc>,
}

impl PhotoAnalysis {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO photo_analysis (product_id, status, error, message) 
            VALUES ($1, $2, $3, $4)",
            &[&self.product_id, &self.status, &self.error, &self.message],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM photo_analysis WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, product_id, status, error, message, analysis_date 
            FROM photo_analysis WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(PhotoAnalysis {
            id: row.get("id"),
            product_id: row.get("product_id"),
            status: row.get("status"),
            error: row.get("error"),
            message: row.get("message"),
            analysis_date: row.get("analysis_date"),
        })
    }
}

#[derive(Debug)]
pub struct ReviewsAnalysis {
    pub id: i32,
    pub product_id: i32,
    pub status: String,
    pub error: String,
    pub message: String,
    pub analysis_date: DateTime<Utc>,
}

impl ReviewsAnalysis {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO reviews_analysis (product_id, status, error, message) 
            VALUES ($1, $2, $3, $4)",
            &[&self.product_id, &self.status, &self.error, &self.message],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM reviews_analysis WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, product_id, status, error, message, analysis_date 
            FROM reviews_analysis WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(ReviewsAnalysis {
            id: row.get("id"),
            product_id: row.get("product_id"),
            status: row.get("status"),
            error: row.get("error"),
            message: row.get("message"),
            analysis_date: row.get("analysis_date"),
        })
    }
}

#[derive(Debug)]
pub struct SeoAnalysis {
    pub id: i32,
    pub product_id: i32,
    pub status: String,
    pub error: String,
    pub message: String,
    pub analysis_date: DateTime<Utc>,
}

impl SeoAnalysis {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO seo_analysis (product_id, status, error, message) 
            VALUES ($1, $2, $3, $4)",
            &[&self.product_id, &self.status, &self.error, &self.message],
        ).await?;
        Ok(())
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM seo_analysis WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, product_id, status, error, message, analysis_date 
            FROM seo_analysis WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(SeoAnalysis {
            id: row.get("id"),
            product_id: row.get("product_id"),
            status: row.get("status"),
            error: row.get("error"),
            message: row.get("message"),
            analysis_date: row.get("analysis_date"),
        })
    }
}

#[derive(Debug)]
pub struct Task {
    pub id: i32,
    pub user_id: Option<String>,
    pub product_id: Option<i32>,
    pub photo_analysis_id: Option<i32>,
    pub reviews_analysis_id: Option<i32>,
    pub seo_analysis_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl Task {
    pub async fn create(&self, pool: &Pool) -> Result<i32, PoolError> {
        let client = pool.get().await?;
        let row = client.query_one(
            "INSERT INTO tasks (user_id, product_id, photo_analysis_id, reviews_analysis_id, seo_analysis_id, created_at) 
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id",
            &[
                &self.user_id,
                &self.product_id,
                &self.photo_analysis_id,
                &self.reviews_analysis_id,
                &self.seo_analysis_id,
                &self.created_at
            ],
        ).await?;
    
        let id: i32 = row.get(0);
        Ok(id)
    }

    pub async fn delete_by_id(pool: &Pool, id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute("DELETE FROM tasks WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: i32) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, user_id, product_id, photo_analysis_id, reviews_analysis_id, seo_analysis_id, created_at 
            FROM tasks WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(Task {
            id: row.get("id"),
            user_id: row.get("user_id"),
            product_id: row.get("product_id"),
            photo_analysis_id: row.get("photo_analysis_id"),
            reviews_analysis_id: row.get("reviews_analysis_id"),
            seo_analysis_id: row.get("seo_analysis_id"),
            created_at: row.get("created_at"),
        })
    }
}

#[derive(Debug)]
pub struct ProductCompetitor {
    pub product_id: i32,
    pub competitor_id: i32,
    pub created_at: DateTime<Utc>,
}

impl ProductCompetitor {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO product_competitors (product_id, competitor_id) VALUES ($1, $2)",
            &[&self.product_id, &self.competitor_id],
        ).await?;
        Ok(())
    }

    pub async fn delete(pool: &Pool, product_id: i32, competitor_id: i32) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "DELETE FROM product_competitors WHERE product_id = $1 AND competitor_id = $2",
            &[&product_id, &competitor_id],
        ).await?;
        Ok(())
    }

    pub async fn find_by_product(pool: &Pool, product_id: i32) -> Result<Vec<Self>, PoolError> {
        let client = pool.get().await?;
        let rows = client.query(
            "SELECT product_id, competitor_id, created_at FROM product_competitors WHERE product_id = $1",
            &[&product_id],
        ).await?;
        
        rows.into_iter().map(Self::from_row).collect()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(ProductCompetitor {
            product_id: row.get("product_id"),
            competitor_id: row.get("competitor_id"),
            created_at: row.get("created_at"),
        })
    }
}

#[derive(Debug)]
pub struct Keyword {
    pub id: i32,
    pub keyword: String,
    pub created_at: DateTime<Utc>,
}

impl Keyword {
    pub async fn create(pool: &Pool, keyword: &str) -> Result<Self, PoolError> {
        let client = pool.get().await?;
        let row = client.query_one(
            "INSERT INTO keywords (keyword) VALUES ($1) ON CONFLICT (keyword) DO UPDATE SET keyword = EXCLUDED.keyword RETURNING id, keyword, created_at",
            &[&keyword],
        ).await?;
        Ok(Self::from_row(row)?)
    }

    pub async fn find_by_keyword(pool: &Pool, keyword: &str) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, keyword, created_at FROM keywords WHERE keyword ILIKE $1",
            &[&format!("%{}%", keyword)],
        ).await?.map(Self::from_row).transpose()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(Keyword {
            id: row.get("id"),
            keyword: row.get("keyword"),
            created_at: row.get("created_at"),
        })
    }
}

#[derive(Debug)]
pub struct TaskKeyword {
    pub task_id: i32,
    pub task_created_at: DateTime<Utc>,
    pub keyword_id: i32,
    pub used_in_analysis: bool,
}

impl TaskKeyword {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client.execute(
            "INSERT INTO task_keywords (task_id, task_created_at, keyword_id, used_in_analysis) 
            VALUES ($1, $2, $3, $4)",
            &[&self.task_id, &self.task_created_at, &self.keyword_id, &self.used_in_analysis],
        ).await?;
        Ok(())
    }

    pub async fn find_by_task(pool: &Pool, task_id: i32, task_created_at: DateTime<Utc>) -> Result<Vec<Self>, PoolError> {
        let client = pool.get().await?;
        let rows = client.query(
            "SELECT task_id, task_created_at, keyword_id, used_in_analysis 
            FROM task_keywords 
            WHERE task_id = $1 AND task_created_at = $2",
            &[&task_id, &task_created_at],
        ).await?;
        
        rows.into_iter().map(Self::from_row).collect()
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(TaskKeyword {
            task_id: row.get("task_id"),
            task_created_at: row.get("task_created_at"),
            keyword_id: row.get("keyword_id"),
            used_in_analysis: row.get("used_in_analysis"),
        })
    }
}