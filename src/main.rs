use std::env;

pub mod client;
pub mod common;
pub mod server;
pub mod tundev;

fn echo_syntax(args: &Vec<String>) {
    println!(
        "Use {} [server|client] [tun_name] [ip] [server_ip]",
        args[0]
    );
}

fn main() -> std::io::Result<()> {
    colog::init();

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "server" {
        return server::run();
    } else if args.len() == 5 && args[1] == "client" {
        return client::run(args[2].clone(), args[3].clone(), args[4].clone());
    } else {
        echo_syntax(&args);
        std::process::exit(1);
    }
}
