use std::env;

pub mod client;
pub mod common;
pub mod packet;
pub mod server;
pub mod tundev;

fn echo_syntax(args: &Vec<String>) {
    println!(
        "Use {} [server|client] [tun_name] [destination] [netmask] [ip] [server_ip]",
        args[0]
    );
}

#[tokio::main]
async fn main() {
    colog::init();

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "server" {
        if server::run().is_err() {
            std::process::exit(1);
        }
    } else if args.len() == 7 && args[1] == "client" {
        let _ = client::run(
            args[2].clone(),
            args[3].clone(),
            args[4].clone(),
            args[5].clone(),
            args[6].clone(),
        )
        .await;
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
}
