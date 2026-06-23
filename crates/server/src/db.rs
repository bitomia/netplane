// Database access layer: some query helpers and row structs are part of the
// API surface but not yet wired into every code path.
#![allow(dead_code)]

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::{collections::HashSet, env, net::Ipv4Addr, path::Path as FilePath};
use tracing::{error, info};

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
    pub auth_link_id: Option<String>,
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
        &self,
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
        .bind(client_id)
        .bind(sdn_client_ip)
        .bind(network)
        .bind(netmask)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO auth_links (id, client_id) VALUES (?, ?)")
            .bind(auth_link_id.to_string())
            .bind(client_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        self.get_client(client_id).await
    }

    pub async fn create_dynamic_client(
        &self,
        pub_key: &str,
        max_attempts: usize,
    ) -> Result<Client, anyhow::Error> {
        // TODO: handle the empty-table case (no existing client to derive the network from)
        let row = sqlx::query!("SELECT network, netmask FROM clients LIMIT 1")
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("No existing client to derive the network from"))?;
        let network = row.network;
        let netmask = row.netmask;

        info!(
            "Creating client for dynamic link (network={} netmask={})",
            network, netmask
        );
        for _ in 0..max_attempts {
            let client_id = uuid::Uuid::new_v4().to_string();
            let free_ip = self.find_free_sdn_client_ip(&network, &netmask).await?;

            match sqlx::query(
                "INSERT INTO clients (id, sdn_client_ip, network, netmask, pub_key) VALUES (?, ?, ?, ?, ?)",
            )
                .bind(&client_id)
                .bind(&free_ip)
                .bind(&network)
                .bind(&netmask)
                .bind(pub_key)
                .execute(&self.pool)
                .await
            {
                Ok(_) => {
                    info!("Client with dynamic link created (network={} netmask={} client_id={})", network, netmask, client_id);
                    return self.get_client(&client_id).await;
                }
                Err(err) => {
                    let lost_race = err.as_database_error()
                        .map(|e| e.is_unique_violation())
                        .unwrap_or(false);
                    if lost_race {
                        continue;
                    }
                    error!("Creating client for dynamic link failed: {}", err.to_string());
                    return Err(anyhow!("Database error on create_dynamic_client: {}", err));
                }
            }
        }

        Err(anyhow!(
            "Could not assign a free sdn_client_ip in {}/{} after {} attempts",
            network,
            netmask,
            max_attempts,
        ))
    }

    async fn find_free_sdn_client_ip(
        &self,
        network: &str,
        netmask: &str,
    ) -> Result<String, anyhow::Error> {
        let network_addr: Ipv4Addr = network
            .parse()
            .map_err(|_| anyhow!("Invalid network address: {}", network))?;
        let netmask_addr: Ipv4Addr = netmask
            .parse()
            .map_err(|_| anyhow!("Invalid netmask: {}", netmask))?;

        let network_bits = u32::from(network_addr);
        let netmask_bits = u32::from(netmask_addr);
        let broadcast_bits = network_bits | !netmask_bits;

        // First and last usable host addresses (exclude network & broadcast).
        let first_host = network_bits.checked_add(1).unwrap_or(network_bits);
        let last_host = broadcast_bits.checked_sub(1).unwrap_or(broadcast_bits);
        if first_host > last_host {
            return Err(anyhow!(
                "No usable host addresses in {}/{}",
                network,
                netmask
            ));
        }

        let used: HashSet<u32> = sqlx::query_scalar!("SELECT sdn_client_ip FROM clients")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .filter_map(|ip| ip.parse::<Ipv4Addr>().ok())
            .map(u32::from)
            .collect();

        for candidate in first_host..=last_host {
            if !used.contains(&candidate) {
                return Ok(Ipv4Addr::from(candidate).to_string());
            }
        }

        Err(anyhow!(
            "No free sdn_client_ip available in {}/{}",
            network,
            netmask
        ))
    }

    pub async fn check_link_key(&self, auth_id: &String) -> Result<AuthClient, anyhow::Error> {
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
        &self,
        link_key: &String,
        pub_key: &String,
    ) -> Result<String, anyhow::Error> {
        match self.check_link_key(link_key).await {
            Ok(is_linked) => {
                let has_used = match is_linked.used {
                    Some(value) => value,
                    _ => {
                        return Err(anyhow!("Unexpected error on auth"));
                    }
                };
                if has_used {
                    return Err(anyhow!("Link key already used"));
                }

                let client_id = match is_linked.client_id {
                    Some(value) => value,
                    _ => {
                        return Err(anyhow!("No user"));
                    }
                };

                let mut tx = self.pool.begin().await?;
                sqlx::query("UPDATE auth_links SET used=true WHERE id=?")
                    .bind(link_key)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE clients SET pub_key=? WHERE id=?")
                    .bind(pub_key)
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

    pub async fn get_all_clients(&self) -> Result<Vec<Client>, anyhow::Error> {
        let clients = sqlx::query_as!(
            Client,
            r#"
SELECT clients.id, auth_links.id as auth_link_id, sdn_client_ip, network, netmask, used FROM clients
LEFT JOIN auth_links ON clients.id=auth_links.client_id
"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(clients)
    }

    pub async fn get_client(&self, client_id: &str) -> Result<Client, anyhow::Error> {
        let client = sqlx::query_as!(
            Client,
            r#"
SELECT clients.id, auth_links.id as auth_link_id, sdn_client_ip, network, netmask, used FROM clients
LEFT JOIN auth_links ON clients.id=auth_links.client_id WHERE clients.id=?
"#,
            client_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(client)
    }

    pub async fn delete_client(&self, client_id: &String) -> Result<(), anyhow::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM auth_links WHERE client_id=?")
            .bind(client_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM clients WHERE id=?")
            .bind(client_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn create_or_update_user(
        &self,
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

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, anyhow::Error> {
        let user = sqlx::query_as!(
            User,
            "SELECT email, password_hash, role FROM users WHERE email = ?",
            email
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn get_hostname(&self, hostname: &str) -> Result<Option<String>, anyhow::Error> {
        let resolved_sdn_ip = sqlx::query_scalar!(
            r#"SELECT sdn_client_ip FROM clients WHERE hostname=?"#,
            hostname
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(resolved_sdn_ip)
    }
}
