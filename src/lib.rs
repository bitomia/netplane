pub mod client;
pub mod common;
pub mod db;
pub mod packet;
pub mod server;
pub mod tundev;
pub mod webserver;

unsafe fn c_str_to_string(c_str: *const libc::c_char) -> String {
    match std::ffi::CStr::from_ptr(c_str).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => String::default(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn start_client(
    tun_name: *const libc::c_char,
    netmask: *const libc::c_char,
    destination: *const libc::c_char,
    ip_addr: *const libc::c_char,
    server_addr: *const libc::c_char,
) {
    colog::init();
    let _ = client::run(
        c_str_to_string(tun_name),
        c_str_to_string(netmask),
        c_str_to_string(destination),
        c_str_to_string(ip_addr),
        c_str_to_string(server_addr),
    );
}
