use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, FromRow, Pool, Sqlite};
use std::{env, path::Path as FilePath};
use tracing::info;

const DEFAULT_DATABASE_URL: &str = "sqlite://netplane.db";

pub async fn do_migrate() {
    let db_file_path = std::env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());

    let db_path = db_file_path.replace("sqlite://", "");

    if !FilePath::new(&db_path).exists() {
        info!("Database file not found, creating...");
        std::fs::File::create(&db_path).expect("Failed to create SQLite file");
    }
    let pool = SqlitePoolOptions::new()
        .connect(&db_file_path)
        .await
        .expect("Cannot connect to database");
    sqlx::migrate!("./src/migrations")
        .run(&pool)
        .await
        .expect("Migration failed");
    println!("Migration successfully finished");
}

pub struct Db {
    pub pool: Pool<Sqlite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Client {
    pub id: String,
    pub auth_link_id: String,
    pub sdn_client_ip: String,
    pub network: String,
    pub netmask: String,
    pub used: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthClient {
    pub client_id: Option<String>,
    pub used: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

impl Db {
    pub async fn new() -> Db {
        let db_file_path = env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
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

    pub async fn create_client(
        self: &Self,
        client_id: &str,
        sdn_client_ip: &str,
        network: &str,
        netmask: &str,
    ) -> Result<Client, anyhow::Error> {
        let auth_link_id = uuid::Uuid::new_v4();

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO clients (id, sdn_client_ip, network, netmask) VALUES (?, ?, ?, ?)",
        )
        .bind(&client_id)
        .bind(&sdn_client_ip)
        .bind(&network)
        .bind(&netmask)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO auth_links (id, client_id) VALUES (?, ?)")
            .bind(&auth_link_id.to_string())
            .bind(&client_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        self.get_client(&client_id).await
    }

    pub async fn is_auth(self: &Self, auth_id: &String) -> Result<AuthClient, anyhow::Error> {
        let auth_entry = sqlx::query_as!(
            AuthClient,
            "SELECT client_id, used FROM auth_links WHERE id=? LIMIT 1",
            auth_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(auth_entry)
    }

    pub async fn auth_client(
        self: &Self,
        auth_id: &String,
        pub_key: &String,
    ) -> Result<String, anyhow::Error> {
        match self.is_auth(&auth_id).await {
            Ok(is_authed) => {
                let has_used = match is_authed.used {
                    Some(value) => value,
                    _ => {
                        return Err(anyhow!("Unexpected error on auth"));
                    }
                };
                if has_used == true {
                    return Err(anyhow!("Auth link already used"));
                }

                let client_id = match is_authed.client_id {
                    Some(value) => value,
                    _ => {
                        return Err(anyhow!("No user"));
                    }
                };

                let mut tx = self.pool.begin().await?;
                sqlx::query("UPDATE auth_links SET used=true WHERE id=?")
                    .bind(&auth_id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE clients SET pub_key=? WHERE id=?")
                    .bind(&pub_key)
                    .bind(&client_id)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;

                let auth_data = netplane_common::crypto::AuthData { client_id };
                let auth_data = serde_json::json!(auth_data).to_string();

                Ok(netplane_common::crypto::sign_key(auth_data.as_bytes()))
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub async fn get_all_clients(self: &Self) -> Result<Vec<Client>, anyhow::Error> {
        let clients = sqlx::query_as!(
            Client,
            r#"
SELECT clients.id, auth_links.id as auth_link_id, sdn_client_ip, network, netmask, used FROM clients
INNER JOIN auth_links ON clients.id=auth_links.client_id
"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(clients)
    }

    pub async fn get_client(self: &Self, client_id: &str) -> Result<Client, anyhow::Error> {
        let client = sqlx::query_as!(
            Client,
            r#"
SELECT clients.id, auth_links.id as auth_link_id, sdn_client_ip, network, netmask, used FROM clients
INNER JOIN auth_links ON clients.id=auth_links.client_id WHERE clients.id=?
"#,
            client_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(client)
    }

    pub async fn delete_client(self: &Self, client_id: &String) -> Result<(), anyhow::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM auth_links WHERE client_id=?")
            .bind(&client_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM clients WHERE id=?")
            .bind(&client_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn create_or_update_user(
        self: &Self,
        email: &str,
        password_hash: &str,
    ) -> Result<bool, anyhow::Error> {
        match self.get_user_by_email(email).await {
            Ok(_) => {
                sqlx::query("UPDATE users SET password_hash = ? WHERE email = ?")
                    .bind(password_hash)
                    .bind(email)
                    .execute(&self.pool)
                    .await?;
                Ok(false)
            }
            Err(_) => {
                sqlx::query("INSERT INTO users (email, password_hash, role) VALUES (?, ?, ?)")
                    .bind(email)
                    .bind(password_hash)
                    .bind("admin")
                    .execute(&self.pool)
                    .await?;
                Ok(true)
            }
        }
    }

    pub async fn get_user_by_email(self: &Self, email: &str) -> Result<User, anyhow::Error> {
        let user = sqlx::query_as!(
            User,
            "SELECT email, password_hash, role FROM users WHERE email = ?",
            email
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn get_hostname(
        self: &Self,
        hostname: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let resolved_sdn_ip = sqlx::query_scalar!(
            r#"SELECT sdn_client_ip FROM clients WHERE hostname=?"#,
            hostname
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(resolved_sdn_ip)
    }
}
