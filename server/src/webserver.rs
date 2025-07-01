use axum::{
    Router, routing::get, routing::post, serve::Serve,
};
use log::info;
use std::sync::Arc;
use tower_http::services::ServeDir;

use crate::handlers::{AppState, get_clients, create_client, delete_client, auth_client};

pub struct WebServer {}

impl WebServer {

    pub async fn new(db: Arc<crate::db::Db>) -> Serve<tokio::net::TcpListener, Router, Router> {
        let addr = std::env::var("WEBSERVER").unwrap_or("0.0.0.0:8000".to_string());
        info!("Starting web server {}", addr);
        let server_url = std::env::var("WEBSERVER_URL").unwrap_or_else(|_| {
            info!("Couldn't find WEBSERVER_URL env var. Using default value.");
            return "http://localhost:3000".to_string();
        });

        let state = AppState { db, server_url };
        let static_web_path = std::env::var("WEB_STATIC_PATH").expect("WEB_STATIC_PATH env var");
        let serve_dir = ServeDir::new(static_web_path);
        let app = Router::new()
            .route(
                "/api/clients",
                get(get_clients)
                    .post(create_client)
                    .delete(delete_client),
            )
            .route("/auth/{auth_key}", post(auth_client))
            .with_state(state)
            .fallback_service(serve_dir);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Cannot bind web server");
        axum::serve(listener, app)
    }
}
