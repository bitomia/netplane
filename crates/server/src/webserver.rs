use axum::{Router, routing::get, routing::post, serve::Serve};
use log::info;
use std::path::Path;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::handlers::{
    AppState, auth_client, create_client, delete_client, get_clients, get_server_stats,
    get_user_data, login, logout,
};

pub struct WebServer {}

impl WebServer {
    pub async fn new(
        db: Arc<crate::db::Db>,
        server_stats: Arc<crate::server::ServerStats>,
    ) -> Serve<tokio::net::TcpListener, Router, Router> {
        let addr = std::env::var("WEBSERVER").unwrap_or("0.0.0.0:8000".to_string());
        info!("Starting web server {}", addr);
        let server_url = std::env::var("WEBSERVER_URL").unwrap_or_else(|_| {
            info!("Couldn't find WEBSERVER_URL env var. Using default value.");
            return "http://localhost:3000".to_string();
        });

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            info!("Couldn't find JWT_SECRET env var. Using default value.");
            return "your-secret-key".to_string();
        });
        let state = AppState {
            db,
            server_stats,
            server_url,
            jwt_secret,
        };
        let static_web_path = std::env::var("WEB_STATIC_PATH").expect("WEB_STATIC_PATH env var");
        let serve_dir = ServeDir::new(&static_web_path).fallback(ServeFile::new(
            Path::new(&static_web_path).join("index.html"),
        ));

        let cors = CorsLayer::new().allow_credentials(true);

        let app = Router::new()
            .route(
                "/api/clients",
                get(get_clients).post(create_client).delete(delete_client),
            )
            .route("/api/login", post(login))
            .route("/api/logout", get(logout))
            .route("/api/user", get(get_user_data))
            .route("/api/server", get(get_server_stats))
            .route("/auth/{auth_key}", post(auth_client))
            .layer(cors)
            .with_state(state)
            .fallback_service(serve_dir);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Cannot bind web server");
        axum::serve(listener, app)
    }
}
