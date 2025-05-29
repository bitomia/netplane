use axum::{
    extract::State, http::StatusCode, response::Json, routing::get, routing::post, serve::Serve, extract::Path,
    Router,
};
use log::info;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use uuid::Uuid;
use crate::common::calculate_network_address;

pub struct WebServer {}

#[derive(Clone)]
struct AppState {
    db: Arc<crate::db::Db>,
    server_url: String,
}

type ServerError = String;

#[derive(Deserialize)]
struct CreateClientRequest {
    sdn_client_ip: String,
    netmask: String,
}

#[derive(Deserialize)]
struct DeleteClientRequest {
    id: String,
}

type WebResult<T> = (StatusCode, Result<Json<T>, Json<ServerError>>);

macro_rules! web_ok {
    ($expression:expr) => { (StatusCode::OK, Ok(Json($expression))) };
}

macro_rules! web_err {
    ($status:expr, $value:expr) => { ($status, Err(Json($value))) };
    ($value:expr) => { (StatusCode::BAD_REQUEST, Err(Json($value))) };
}

impl WebServer {
    async fn get_clients(State(state): State<AppState>) -> WebResult<Vec<crate::db::Client>> {
        match state.db.get_all_clients().await {
            Ok(clients) => web_ok!(
                clients.iter().map(|c| crate::db::Client {
                    id: c.id.clone(),
                    auth_link_id: format!("http://{}/auth/{}", state.server_url, c.auth_link_id),
                    sdn_client_ip: c.sdn_client_ip.clone(),
                    network: c.network.clone(),
                    netmask: c.netmask.clone(),
                    used: c.used,
                }).collect::<Vec<crate::db::Client>>()
            ),
            Err(error) => web_err!(error.to_string())
        }
    }

    async fn create_client(
        State(state): State<AppState>,
        Json(payload): Json<CreateClientRequest>,
    ) -> WebResult<crate::db::Client> {
        let network_address = calculate_network_address(payload.sdn_client_ip.as_str(), payload.netmask.as_str());
        let network_address = match network_address {
            Ok(value) => value.to_string(),
            Err(err) => { return web_err!(format!("Invalid netmask or IP: {}", err)); } 
        };
        
        let id = Uuid::new_v4();
        match state.db.create_client(&id.to_string().as_str(), &payload.sdn_client_ip, &network_address.as_str(), &payload.netmask.as_str()).await {
            Ok(client) => web_ok!(client),
            Err(error) => web_err!(error.to_string())
        }
    }

    async fn delete_client(
        State(state): State<AppState>,
        Json(payload): Json<DeleteClientRequest>,
    ) -> WebResult<Vec<crate::db::Client>> {
        let delete_ret = state.db.delete_client(&payload.id).await;
        if let Err(error) = delete_ret {
            return web_err!(StatusCode::BAD_REQUEST, error.to_string());
        }

        match state.db.get_all_clients().await {
            Ok(clients) => web_ok!(clients),
            Err(error) => web_err!(error.to_string())
        }
    }

    async fn auth_client(
        State(state): State<AppState>,
        Path(auth_link_id): Path<String>,
        Json(payload): Json<crate::common::AuthClientRequest>
    ) -> (StatusCode, Result<String, Json<ServerError>>) {
        match state.db.auth_client(&auth_link_id, &payload.public_key).await {
            Ok(auth_key) => (StatusCode::OK, Ok(auth_key)),
            Err(error) =>  web_err!(error.to_string()),
        }
    }

    pub async fn new(addr: &str, db: Arc<crate::db::Db>) -> Serve<tokio::net::TcpListener, Router, Router> {
        info!("Starting web server {}", addr);
        let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| {
            info!("Couldn't find SERVER_URL env var. Using default value.");
            return "http://localhost:3000".to_string(); 
        });

        let state = AppState { db, server_url };
        let serve_dir = ServeDir::new("web/build/client");
        let app = Router::new()
            .route(
                "/api/clients",
                get(Self::get_clients)
                    .post(Self::create_client)
                    .delete(Self::delete_client),
            )
            .route("/auth/{auth_key}", post(Self::auth_client))
            .with_state(state)
            .fallback_service(serve_dir);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Cannot bind web server");
        axum::serve(listener, app)
    }
}
