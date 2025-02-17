use dotenv::dotenv;

pub mod db;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let _ = db::Db::new().await;
}
