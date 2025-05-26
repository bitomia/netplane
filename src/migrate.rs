use dotenv::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use std::{env, path::Path as FilePath};
use log::info;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let db_file_path = env::var("DATABASE_URL").unwrap();
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
