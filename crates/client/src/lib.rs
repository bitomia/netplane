use log::{error, info};
use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;

use netplane_common::crypto::load_auth_key;
use netplane_common::transport::AnyTransport;

pub mod client;
mod fd;
mod http_client;
mod peer_session;
mod tray;
mod tundev;

use crate::fd::PlatformFd;
pub use netplane_common::crypto::try_generate_crypto_keys;

static GLOBAL_RT: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

static CANCEL_TOKEN: Lazy<Mutex<CancellationToken>> =
    Lazy::new(|| Mutex::new(CancellationToken::new()));

async fn do_handshake(
    authkey_path: *const c_char,
    transport: *mut std::ffi::c_void,
    result: *mut HandshakeResult,
) -> i32 {
    let authkey_path = unsafe {
        match CStr::from_ptr(authkey_path).to_str() {
            Ok(s) => s.to_string(),
            Err(err) => {
                error!("Error parsing auth_key: {:?}", err);
                return -2;
            }
        }
    };
    let auth_key = match load_auth_key(authkey_path) {
        Ok(auth_key) => auth_key,
        Err(err) => {
            error!("Error: {:?}", err);
            return -3;
        }
    };

    let transport_ref = unsafe { &mut *(transport as *mut AnyTransport) };

    let (start_params, _noise_session) =
        match client::handshake(auth_key, "connected".to_string(), transport_ref).await {
            Ok(params) => params,
            Err(err) => {
                error!("Handshake failed: {:?}", err);
                return -6;
            }
        };

    let netmask_cstr = match CString::new(start_params.netmask) {
        Ok(s) => s.into_raw(),
        Err(err) => {
            error!("Error converting netmask: {:?}", err);
            return -7;
        }
    };

    let destination_cstr = match CString::new(start_params.destination) {
        Ok(s) => s.into_raw(),
        Err(err) => {
            error!("Error converting destination: {:?}", err);
            unsafe {
                let _ = CString::from_raw(netmask_cstr);
            }
            return -8;
        }
    };

    let ip_addr_cstr = match CString::new(start_params.ip_addr) {
        Ok(s) => s.into_raw(),
        Err(err) => {
            error!("Error converting ip_addr: {:?}", err);
            unsafe {
                let _ = CString::from_raw(netmask_cstr);
                let _ = CString::from_raw(destination_cstr);
            }
            return -9;
        }
    };

    unsafe {
        (*result).netmask = netmask_cstr;
        (*result).destination = destination_cstr;
        (*result).ip_addr = ip_addr_cstr;
    }

    0
}

#[repr(C)]
pub struct HandshakeResult {
    pub netmask: *mut c_char,
    pub destination: *mut c_char,
    pub ip_addr: *mut c_char,
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_init_logger() {
    client::init_logger();
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_auth(
    authkey_path: *const c_char,
    publickey_path: *const c_char,
    privatekey_path: *const c_char,
    host: *const c_char,
    link_code: *const c_char,
    auth_port: u16,
) -> i32 {
    GLOBAL_RT.block_on(async {
        let host = unsafe {
            match CStr::from_ptr(host).to_str() {
                Ok(s) => s,
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let link_code = unsafe {
            match CStr::from_ptr(link_code).to_str() {
                Ok(s) => s,
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let authkey_path = unsafe {
            match CStr::from_ptr(authkey_path).to_str() {
                Ok(s) => s,
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let publickey_path = unsafe {
            match CStr::from_ptr(publickey_path).to_str() {
                Ok(s) => s,
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let privatekey_path = unsafe {
            match CStr::from_ptr(privatekey_path).to_str() {
                Ok(s) => s,
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -1;
                }
            }
        };

        let auth_port_opt = if auth_port == 0 {
            None
        } else {
            Some(auth_port)
        };

        match client::auth_client(
            authkey_path,
            publickey_path,
            privatekey_path,
            host,
            link_code,
            auth_port_opt,
        )
        .await
        {
            Ok(_) => 0,
            Err(err) => {
                error!("Error: {:?}", err);
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
                error!("Error: {:?}", err);
                return -1;
            }
        }
    };

    let private_filepath = unsafe {
        match CStr::from_ptr(private_filepath).to_str() {
            Ok(s) => s,
            Err(err) => {
                error!("Error: {:?}", err);
                return -1;
            }
        }
    };

    match try_generate_crypto_keys(public_filepath, private_filepath) {
        Ok(_) => 0,
        Err(err) => {
            error!("Error: {:?}", err);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_handshake(
    authkey_path: *const c_char,
    transport: *mut std::ffi::c_void,
    result: *mut HandshakeResult,
) -> i32 {
    GLOBAL_RT.block_on(async {
        tokio::select! {
            ret = do_handshake(authkey_path, transport, result) => ret,
            _ = sleep(Duration::from_secs(10)) => {
                error!("Handshake timeout");
                -100
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_free_handshake(result: *mut HandshakeResult) {
    if result.is_null() {
        return;
    }

    unsafe {
        let result = &mut *result;

        if !result.ip_addr.is_null() {
            let _ = CString::from_raw(result.ip_addr);
            result.ip_addr = std::ptr::null_mut();
        }
        if !result.destination.is_null() {
            let _ = CString::from_raw(result.destination);
            result.destination = std::ptr::null_mut();
        }
        if !result.netmask.is_null() {
            let _ = CString::from_raw(result.netmask);
            result.netmask = std::ptr::null_mut();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_create_transport(
    server_addr: *const c_char,
    server_port: u16,
    transport_type: *const c_char,
) -> *mut std::ffi::c_void {
    GLOBAL_RT.block_on(async {
        let control_addr = unsafe {
            match CStr::from_ptr(server_addr).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Error parsing control_addr: {:?}", err);
                    return std::ptr::null_mut();
                }
            }
        };
        let control_addr = format!("{}:{}", control_addr, server_port);

        let transport_type_opt = if transport_type.is_null() {
            None
        } else {
            unsafe {
                match CStr::from_ptr(transport_type).to_str() {
                    Ok(s) => Some(s.to_string()),
                    Err(err) => {
                        error!("Error parsing transport_type: {:?}", err);
                        return std::ptr::null_mut();
                    }
                }
            }
        };

        match client::create_transport(&control_addr, transport_type_opt).await {
            Ok(transport) => Box::into_raw(Box::new(transport)) as *mut std::ffi::c_void,
            Err(err) => {
                error!("Error creating transport: {:?}", err);
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_free_transport(transport: *mut std::ffi::c_void) {
    if !transport.is_null() {
        unsafe {
            let _ = Box::from_raw(transport as *mut AnyTransport);
        }
    }
}

fn reset_cancel_token() {
    let mut token = CANCEL_TOKEN.lock().unwrap();
    *token = CancellationToken::new();
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_run(
    tun_fd: c_int,
    transport: *mut std::ffi::c_void,
    handshake: *mut HandshakeResult,
    loopback_relay: bool,
    no_encryption: bool,
) -> i32 {
    let cancel_token = {
        let token = CANCEL_TOKEN.lock().unwrap();
        token.child_token()
    };

    let handshake_result = {
        let netmask = unsafe {
            match CStr::from_ptr((*handshake).netmask).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -3;
                }
            }
        };
        let destination = unsafe {
            match CStr::from_ptr((*handshake).destination).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -3;
                }
            }
        };
        let ip_addr = unsafe {
            match CStr::from_ptr((*handshake).ip_addr).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Error: {:?}", err);
                    return -3;
                }
            }
        };
        client::StartParams {
            netmask,
            destination,
            ip_addr,
        }
    };
    if transport.is_null() {
        error!("Transport pointer is null");
        return -4;
    }
    let transport_ref = unsafe { &mut *(transport as *mut AnyTransport) };

    // Convert c_int to PlatformFd
    #[cfg(unix)]
    let platform_fd = PlatformFd::from_raw_fd(tun_fd);
    #[cfg(windows)]
    let platform_fd = PlatformFd::from_raw_handle(tun_fd as _);

    GLOBAL_RT.spawn(async move {
        tokio::select! {
            retval = client::run_from_fd(platform_fd, &handshake_result, transport_ref, loopback_relay, no_encryption) => {
                    reset_cancel_token();
                match retval {
                    Ok(_) => 0,
                    Err(err) => {
                        error!("Error running client: {}", err);
                        return -12;
                    }
                }
            },
            _ = cancel_token.cancelled() => {
                info!("Client stopped");
                reset_cancel_token();
                1
            },
        }
    });

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_client_stop() {
    info!("Stopping netplane");

    let token = CANCEL_TOKEN.lock().unwrap();
    token.cancel();
}

#[unsafe(no_mangle)]
pub extern "C" fn netplane_run(
    tun_dev: *const c_char,
    host: *const c_char,
    port: u16,
    transport_type: *const c_char,
    loopback_relay: bool,
    no_encryption: bool,
) -> i32 {
    GLOBAL_RT.block_on(async {
        let tun_dev = unsafe {
            match CStr::from_ptr(tun_dev).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Error parsing tun_dev: {:?}", err);
                    return -1;
                }
            }
        };

        let host = unsafe {
            match CStr::from_ptr(host).to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Error parsing host: {:?}", err);
                    return -1;
                }
            }
        };

        let port_opt = if port == 0 { None } else { Some(port) };

        let transport_type_opt = if transport_type.is_null() {
            None
        } else {
            unsafe {
                match CStr::from_ptr(transport_type).to_str() {
                    Ok(s) => Some(s.to_string()),
                    Err(err) => {
                        error!("Error parsing transport_type: {:?}", err);
                        return -1;
                    }
                }
            }
        };

        match client::run(
            tun_dev,
            host,
            port_opt,
            transport_type_opt,
            loopback_relay,
            no_encryption,
        )
        .await
        {
            Ok(_) => 0,
            Err(err) => {
                error!("Error running client: {:?}", err);
                -1
            }
        }
    })
}
