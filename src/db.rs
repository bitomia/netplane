use log::info;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path as FilePath;

pub enum ErrorCode {
    CannotConnect,
}

pub struct Db {
    pub pool: Pool<Sqlite>,
}

impl Db {
    pub async fn new() -> Db {
        let db_path = "reticula.db";

        if !FilePath::new(&db_path).exists() {
            info!("Database file not found, creating...");
            std::fs::File::create(db_path).expect("Failed to create SQLite file");
        }
        let pool = SqlitePoolOptions::new()
            .connect(db_path)
            .await
            .expect("Failed to connect to database");
        sqlx::migrate!("./src/migrations")
            .run(&pool)
            .await
            .expect("Migrations failed");

        Db { pool }
    }

    pub async fn check_client(self: &Self, base64_identity: &String, sdn_client_ip: &String) -> bool {
        println!("{} {}", base64_identity, sdn_client_ip);
        let query_ret: (i64,) =
            sqlx::query_as("select count(*) from clients where client_ip=? and sdn_client_ip=?")
                .bind(base64_identity)
                .bind(sdn_client_ip)
                .fetch_one(&self.pool)
                .await
                .expect("");
        query_ret.0 > 0
    }
}
