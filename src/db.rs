use anyhow::anyhow;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, FromRow};
use std::{env, path::Path as FilePath};
use serde::{Deserialize, Serialize};
use log::info;

pub enum ErrorCode {
    CannotConnect,
}

pub struct Db {
    pub pool: Pool<Sqlite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Client {
    pub id: String,
    pub sdn_client_ip: String,
    pub network: String,
    pub netmask: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthClient {
    pub client_id: Option<String>,
    pub used: Option<bool>,
}

impl Db {
    pub async fn new() -> Db {
        let db_file_path = env::var("DATABASE_URL").unwrap();
        let db_path = db_file_path.replace("sqlite://", "");

        if !FilePath::new(&db_path).exists() {
            info!("Database file not found, creating...");
            std::fs::File::create(&db_path).expect("Failed to create SQLite file");
        }
        let pool = SqlitePoolOptions::new()
            .connect(&db_file_path)
            .await
            .expect("Failed to connect to database");
        sqlx::migrate!("./src/migrations")
            .run(&pool)
            .await
            .expect("Migrations failed");

        Db { pool }
    }

    pub async fn get_client_sdn_ip(self: &Self, client_key: &str) -> Option<String> {
        sqlx::query_scalar("SELECT sdn_client_ip FROM clients WHERE public_key = $1")
            .bind(client_key)
            .fetch_optional(&self.pool)
            .await.ok()?
    }

    pub async fn create_client(self: &Self, client: &Client) -> Result<(), anyhow::Error> {
        let mut tx = self.pool.begin().await?;
        
        sqlx::query("INSERT INTO clients (id, sdn_client_ip, network, netmask) VALUES (?, ?, ?, ?)")
            .bind(&client.id)
            .bind(&client.sdn_client_ip)
            .bind(&client.network)
            .bind(&client.netmask)
            .execute(&mut *tx)
            .await?;

        let auth_link_id = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO auth_links (id, client_id) VALUES (?, ?)")
            .bind(&auth_link_id)
            .bind(&client.id)
            .execute(&mut *tx)
            .await?;
        
        tx.commit().await?;
        Ok(())
    }

    pub async fn is_auth(self: &Self, auth_id: &String) -> Result<AuthClient, anyhow::Error> {
        let auth_entry = sqlx::query_as!(AuthClient, "SELECT client_id, used FROM auth_links WHERE id=? LIMIT 1", auth_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(auth_entry)
    }
    
    pub async fn auth_client(self: &Self, auth_id: &String, pub_key: &String) -> Result<(), anyhow::Error> {
        let is_authed = self.is_auth(&auth_id).await?;
        if is_authed.client_id.is_none() { return Err(anyhow!("No user")); }
        if is_authed.used.is_none() { return Err(anyhow!("Unexpected error on auth")); }
        if is_authed.used.unwrap() == true { return Err(anyhow!("Auth already used")); }

        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE auth_links SET used=true WHERE id=?").bind(&auth_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE clients SET pub_key=? WHERE id=?").bind(&pub_key).bind(&is_authed.client_id).execute(&mut *tx).await?;
        
        tx.commit().await?;
        Ok(())
    }
    
    pub async fn get_all_clients(self: &Self) -> Result<Vec<Client>, anyhow::Error> {
        let clients = sqlx::query_as!(Client, "SELECT id, sdn_client_ip, network, netmask FROM clients",)
            .fetch_all(&self.pool)
            .await?;
        Ok(clients)
    }
    
    pub async fn delete_client(self: &Self, client_id: &String) -> Result<(), anyhow::Error> {
        sqlx::query("DELETE FROM clients WHERE id=? RETURNING *")
            .bind(&client_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
