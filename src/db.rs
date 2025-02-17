use log::{debug, error, info};
use sqlx::{migrate, sqlite::SqlitePoolOptions, Pool, Sqlite};
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

    pub async fn check_client(self: &Self, client_ip: &String, sdn_client_ip: &String) -> bool {
        let query_ret: (i64,) =
            sqlx::query_as("select count(*) from clients where client_ip=? and sdn_client_ip=?")
                .bind(&client_ip)
                .bind(&sdn_client_ip)
                .fetch_one(&self.pool)
                .await
                .expect("");
        return query_ret.0 > 0;
    }
}
