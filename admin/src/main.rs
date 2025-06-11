use rusqlite::{Connection, Result};

struct User {
    id: i32,
    username: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} list", args[0]);
        return Ok(());
    }
    match args[1].as_str() {
        "list" => {
            list_users()?;
        },
        _ => {
            println!("Unknown command: {}", args[1]);
        }
    }
    Ok(())
}

fn list_users() -> Result<()> {
    let conn = Connection::open("users.db")?;
    let mut stmt = conn.prepare("SELECT id, username FROM users")?;
    let users_iter = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
        })
    })?;

    for user in users_iter {
        let user = user?;
        println!("Id: {}, Username: {}", user.id, user.username);
    }
    Ok(())
}
