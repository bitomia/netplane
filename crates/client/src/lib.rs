mod client;

use anyhow::Result;
pub use netplane_common::crypto::try_generate_crypto_keys;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_run(
    tun_dev: *const c_char,
    host: *const c_char,
    port: u16,
    transport_type: *const c_char,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    runtime.block_on(async {
        let tun_dev = unsafe {
            match CStr::from_ptr(tun_dev).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    println!("Error: {:?}", err);
                    return -1;
                }
            }
        };
        let host = unsafe {
            match CStr::from_ptr(host).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    println!("Error: {:?}", err);
                    return -1;
                }
            }
        };
        let transport_type = if transport_type.is_null() {
            None
        } else {
            unsafe {
                match CStr::from_ptr(transport_type).to_str() {
                    Ok(s) => Some(s.to_string()),
                    Err(err) => {
                        println!("Error: {:?}", err);
                        return -1;
                    }
                }
            }
        };
        let port_opt = if port == 0 { None } else { Some(port) };

        match client::run(tun_dev, host, port_opt, transport_type).await {
            Ok(_) => 0,
            Err(err) => {
                println!("Error: {:?}", err);
                -1
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_init_logger() {
    client::init_logger();
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_auth(
    host: *const c_char,
    link_code: *const c_char,
    auth_port: u16,
    auth_key_out: *mut *mut c_char,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            println!("Error: {:?}", err);
            return -1;
        }
    };

    runtime.block_on(async {
        let host = unsafe {
            match CStr::from_ptr(host).to_str() {
                Ok(s) => s,
                Err(err) => {
                    println!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let link_code = unsafe {
            match CStr::from_ptr(link_code).to_str() {
                Ok(s) => s,
                Err(err) => {
                    println!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let auth_port_opt = if auth_port == 0 {
            None
        } else {
            Some(auth_port)
        };

        match client::auth_client(host, link_code, auth_port_opt).await {
            Ok(_) => 0,
            Err(err) => {
                println!("Error: {:?}", err);
                -1
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_free_auth(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_try_generate_crypto_keys(
    public_filepath: *const c_char,
    private_filepath: *const c_char,
) -> i32 {
    let public_filepath = unsafe {
        match CStr::from_ptr(public_filepath).to_str() {
            Ok(s) => s,
            Err(err) => {
                println!("Error: {:?}", err);
                return -1;
            }
        }
    };

    let private_filepath = unsafe {
        match CStr::from_ptr(private_filepath).to_str() {
            Ok(s) => s,
            Err(err) => {
                println!("Error: {:?}", err);
                return -1;
            }
        }
    };

    match try_generate_crypto_keys(public_filepath, private_filepath) {
        Ok(_) => 0,
        Err(err) => {
            println!("Error: {:?}", err);
            -1
        }
    }
}
