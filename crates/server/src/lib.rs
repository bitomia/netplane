use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::runtime::Runtime;

use netplane_common::transport::TransportMode;

mod db;
mod peers;
mod server;
mod source;
mod trafficlog;
mod udpserver;
mod wsserver;

static GLOBAL_RT: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

#[unsafe(no_mangle)]
pub extern "C" fn netplane_server_run(transport_mode: TransportMode) {
    GLOBAL_RT.block_on(async {
        let db = Arc::new(db::Db::new().await);
        let server_stats = Arc::new(server::ServerStats::new(transport_mode.clone()));

        match server::run(db, server_stats, transport_mode, None, None, None, None).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => tracing::error!("Server exited with error: {:?}", err),
            Err(err) => tracing::error!("Server task panicked: {:?}", err),
        }
    });
}
