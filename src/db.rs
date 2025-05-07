use log::info;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::{env, path::Path as FilePath};

pub enum ErrorCode {
    CannotConnect,
}

pub struct Db {
    pub pool: Pool<Sqlite>,
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
        sqlx::query_scalar("SELECT sdn_client_ip FROM clients WHERE client_key = $1")
            .bind(client_key)
            .fetch_optional(&self.pool)
            .await.ok()?
    }
}
