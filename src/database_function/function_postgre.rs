use chrono::{DateTime, Utc};
use deadpool_postgres::tokio_postgres::Row;
use deadpool_postgres::{Pool, PoolError};
use uuid::Uuid;

#[derive(Debug)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub is_admin: bool,
}

impl User {
    pub async fn create(&self, pool: &Pool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute(
                "SELECT create_user($1, $2, $3)",
                &[&self.id, &self.email, &self.name],
            )
            .await?;
        Ok(())
    }

    pub async fn set_to_admin(pool: &Pool, id: &str, is_admin: &bool) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute(
                "UPDATE users SET is_admin = $1 WHERE id = $2",
                &[is_admin, &id],
            )
            .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &Pool, id: &str) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client
            .query_opt(
                "SELECT id, email, name, is_admin FROM users WHERE id = $1",
                &[&id],
            )
            .await?
            .map(Self::from_row)
            .transpose()
    }

    pub async fn find_by_email(pool: &Pool, email: &str) -> Result<bool, PoolError> {
        let client = pool.get().await?;
        Ok(client
            .query_one("SELECT user_exists_email($1)", &[&email])
            .await?
            .get("user_exists_email"))
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            name: row.get("name"),
            is_admin: row.get("is_admin"),
        })
    }
}

#[derive(Debug)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: String,
    pub browser: String,
    pub device: String,
    pub os: String,
    pub _last_activity: DateTime<Utc>,
}

impl UserSession {
    pub async fn create(
        id_user: &str,
        browser: &str,
        device: &str,
        os: &str,
        pool: &Pool,
    ) -> Result<Uuid, PoolError> {
        let client = pool.get().await?;
        Ok(client
            .query_one(
                "SELECT create_user_session($1, $2, $3, $4)",
                &[&id_user, &browser, &device, &os],
            )
            .await?
            .get(0))
    }

    pub async fn delete_by_id(pool: &Pool, id: Uuid) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute("DELETE FROM user_session WHERE id = $1", &[&id])
            .await?;
        Ok(())
    }

    pub async fn find_by_user_id(pool: &Pool, id_user: &str) -> Result<Vec<Self>, PoolError> {
        let client = pool.get().await?;
        client.query(
            "SELECT id, id_user, browser, device, os, last_activity FROM user_session WHERE id_user = $1",
            &[&id_user]
        ).await?.into_iter().map(Self::from_row).collect()
    }

    pub async fn find_by_id(pool: &Pool, id: &Uuid) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client.query_opt(
            "SELECT id, id_user, browser, device, os, last_activity FROM user_session WHERE id = $1",
            &[&id]
        ).await?.map(Self::from_row).transpose()
    }
    pub async fn check_by_id(pool: &Pool, id: &Uuid) -> Result<bool, PoolError> {
        let client = pool.get().await?;
        Ok(client
            .query_one("SELECT get_user_session($1)", &[&id])
            .await?
            .get("get_user_session"))
    }

    pub async fn update_time(pool: &Pool, id: &Uuid) -> Result<bool, PoolError> {
        let client = pool.get().await?;
        Ok(client
            .query_one("SELECT update_session_activity($1)", &[&id])
            .await?
            .get("update_session_activity"))
    }
    pub async fn update_user_session_id(pool: &Pool, id: &Uuid) -> Result<Uuid, PoolError> {
        let client = pool.get().await?;
        Ok(client
            .query_one("SELECT update_user_session_id($1)", &[&id])
            .await?
            .get("update_user_session_id"))
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(UserSession {
            id: row.get("id"),
            user_id: row.get("id_user"),
            browser: row.get("browser"),
            device: row.get("device"),
            os: row.get("os"),
            _last_activity: row.get("last_activity"),
        })
    }
}

#[derive(Debug)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub _user_id: String,
    pub created_at: DateTime<Utc>,
}

impl Task {
    pub async fn create(name: &str, user_id: &str, pool: &Pool) -> Result<Uuid, PoolError> {
        let client = pool.get().await?;
        let row = client
            .query_one(
                "INSERT INTO tasks (id, name, user_id, created_at) 
             VALUES (uuid_generate_v7(), $1, $2, $3)
             RETURNING id",
                &[&name, &user_id, &Utc::now()],
            )
            .await?;

        let id: uuid::Uuid = row.get(0);
        Ok(id)
    }

    pub async fn delete_by_id(pool: &Pool, id: &Uuid) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute(
                "DELETE FROM tasks 
             WHERE id = $1",
                &[id],
            )
            .await?;
        Ok(())
    }

    pub async fn find_by_user_id(pool: &Pool, id: &str) -> Result<Vec<Self>, PoolError> {
        let client = pool.get().await?;
        let rows = client
            .query(
                "SELECT id, name, user_id, created_at 
             FROM tasks 
             WHERE user_id = $1 
             ORDER BY created_at DESC",
                &[&id],
            )
            .await?;

        let results: Vec<Self> = rows
            .into_iter()
            .map(|row| Self {
                id: row.get("id"),
                name: row.get("name"),
                _user_id: row.get("user_id"),
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(results)
    }
    pub async fn update_time(pool: &Pool, id: &Uuid) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute(
                "UPDATE tasks 
             SET created_at = $2 
             WHERE id = $1",
                &[&id, &Utc::now()],
            )
            .await?;
        Ok(())
    }

    pub async fn update_name(pool: &Pool, id: Uuid, new_name: &str) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute(
                "UPDATE tasks 
             SET name = $2 
             WHERE id = $1",
                &[&id, &new_name],
            )
            .await?;
        Ok(())
    }
}

pub struct SubscribeUser {
    pub _created_at: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
}

impl SubscribeUser {
    pub async fn get_by_user_id(pool: &Pool, user_id: &str) -> Result<Option<Self>, PoolError> {
        let client = pool.get().await?;
        client
            .query_opt(
                "SELECT created_at, valid_to
	        FROM public.subscribe_users WHERE user_id = $1;",
                &[&user_id],
            )
            .await?
            .map(Self::from_row)
            .transpose()
    }

    pub async fn create(
        user_id: &str,
        created_at: DateTime<Utc>,
        valid_to: DateTime<Utc>,
        pool: &Pool,
    ) -> Result<(), PoolError> {
        let client = pool.get().await?;
        client
            .execute(
                "INSERT INTO public.subscribe_users (user_id, created_at, valid_to) 
             VALUES ($1, $2, $3)",
                &[&user_id, &created_at, &valid_to],
            )
            .await?;
        Ok(())
    }
    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(SubscribeUser {
            _created_at: row.get("created_at"),
            valid_to: row.get("valid_to"),
        })
    }
}

pub struct FullUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub is_admin: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
}

impl FullUser {
    pub async fn find_all(pool: &Pool) -> Result<Vec<Self>, PoolError> {
        let client = pool.get().await?;
        let rows = client
            .query("SELECT u.id, u.email, u.name, u.is_admin, s.created_at, s.valid_to FROM public.users u LEFT JOIN public.subscribe_users s ON u.id = s.user_id;", &[])
            .await?;
        let results: Vec<Self> = rows
            .into_iter()
            .map(Self::from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    fn from_row(row: Row) -> Result<Self, PoolError> {
        Ok(FullUser {
            id: row.get("id"),
            email: row.get("email"),
            name: row.get("name"),
            is_admin: row.get("is_admin"),
            created_at: row.get("created_at"),
            valid_to: row.get("valid_to"),
        })
    }
}
