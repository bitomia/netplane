const COMMANDS: &[&str] = &[
    "ping",
    "request_vpn_permission",
    "start_vpn",
    "stop_vpn",
    "get_tunnel_fd",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
