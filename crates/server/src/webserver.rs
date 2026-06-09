use axum::{Router, routing::get, routing::post, serve::Serve};
use axum_embed::ServeEmbed;
use rust_embed::RustEmbed;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(RustEmbed, Clone)]
#[folder = "../../web/dist/"]
struct Assets;

use crate::handlers::{
    AppState, auth_client, create_client, delete_client, get_clients, get_server_stats,
    get_user_data, login, logout, verify_client,
};

pub struct WebServer {}

impl WebServer {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        db: Arc<crate::db::Db>,
        server_stats: Arc<crate::server::ServerStats>,
        dynamic_clients_key: Option<String>,
    ) -> Serve<tokio::net::TcpListener, Router, Router> {
        let addr = std::env::var("WEBSERVER").unwrap_or("0.0.0.0:8000".to_string());
        info!("Starting web server {}", addr);

        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET env var not found");
        let state = AppState {
            db,
            server_stats,
            jwt_secret,
            dynamic_clients_key,
        };

        let cors = CorsLayer::new().allow_credentials(true);

        let app = Router::new()
            // Endpoints for the administation web panel
            .route(
                "/api/clients",
                get(get_clients).post(create_client).delete(delete_client),
            )
            .route("/api/login", post(login))
            .route("/api/logout", get(logout))
            .route("/api/user", get(get_user_data))
            .route("/api/server", get(get_server_stats))
            // Endpoints for netplane clients
            .route("/auth/{link_key}", post(auth_client))
            .route("/auth", get(verify_client))
            .layer(cors)
            .with_state(state)
            .fallback_service(ServeEmbed::<Assets>::new());

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Cannot bind web server");
        axum::serve(listener, app)
    }
}
