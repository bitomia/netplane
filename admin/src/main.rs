use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use rpassword::read_password;

struct User {
    id: i32,
    username: String,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} list", args[0]);
        return Ok(());
    }
    let pool = SqlitePool::connect("sqlite://users.db").await?;
    match args[1].as_str() {
        "list" => {
            list_users(&pool).await?;
        },
        "create" => {
            if args.len() < 3 {
                println!("Usage: {} create <username>", args[0]);
            } else {
                create_user(&pool, &args[2]).await?;
            }
        },
        _ => {
            println!("Unknown command: {}", args[1]);
        }
    }
    Ok(())
}

async fn create_user(pool: &SqlitePool, username: &str) -> Result<(), sqlx::Error> {
    println!("Enter password:");
    let password = read_password().unwrap();
    println!("Confirm password:");
    let password_confirmation = read_password().unwrap();
    if password != password_confirmation {
        println!("Passwords do not match");
        return Ok(());
    }
    sqlx::query("INSERT INTO users (username, password) VALUES (?1, ?2)")
        .bind(username)
        .bind(&password)
        .execute(pool)
        .await?;
    println!("User '{}' created", username);
    Ok(())
}

async fn list_users(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("SELECT id, username FROM users")
        .fetch_all(pool)
        .await?;
    for row in rows {
        let id: i32 = row.get("id");
        let username: String = row.get("username");
        println!("Id: {}, Username: {}", id, username);
    }
    Ok(())
}
