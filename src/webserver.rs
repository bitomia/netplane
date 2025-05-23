use axum::{
    extract::State, http::StatusCode, response::Json, routing::get, routing::post, serve::Serve,
    Router,
};
use log::info;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use uuid::Uuid;

pub struct WebServer {}

#[derive(Clone)]
struct AppState {
    db: Arc<Db>,
}

type ServerError = String;

#[derive(Deserialize)]
struct CreateClientRequest {
    client_key: String,
    sdn_client_ip: String,
    network: String,
    netmask: String,
}

#[derive(Deserialize)]
struct DeleteClientRequest {
    id: String,
}

#[derive(Deserialize)]
struct AuthClientRequest {
    public_key: String,
}

impl WebServer {
    async fn get_clients(State(state): State<AppState>) -> (StatusCode, Json<Result<Vec<crate::db::Client>, ServerError>>) {
        match state.db.get_all_clients().await {
            Ok(clients) => (StatusCode::OK, Json(Ok(clients))),
            Err(error) => (StatusCode::BAD_REQUEST, Json(Err(error.to_string())))
        }
    }

    async fn create_client(
        State(state): State<AppState>,
        Json(payload): Json<CreateClientRequest>,
    ) -> (StatusCode, Json<Result<crate::db::Client, ServerError>>) {
        let id = Uuid::new_v4();

        let client = crate::db::Client {
            id: id.to_string(),
            sdn_client_ip: payload.sdn_client_ip,
            network: payload.network,
            netmask: payload.netmask,
        };
        match state.db.create_client(&client).await {
            Ok(_) => (StatusCode::CREATED, Json(Ok(client))),
            Err(error) => (StatusCode::BAD_REQUEST, Json(Err(error.to_string())))
        }
    }

    async fn delete_client(
        State(state): State<AppState>,
        Json(payload): Json<DeleteClientRequest>,
    ) -> (StatusCode, Json<Result<Vec<crate::db::Client>, ServerError>>) {
        let delete_ret = state.db.delete_client(payload.id).await;
        if let Err(error) = delete_ret {
            return (StatusCode::BAD_REQUEST, Json(Err(error.to_string())));
        }
z
        match state.db.get_all_clients().await {
            Ok(clients) => (StatusCode::OK, Json(Ok(clients))),
            Err(error) => (StatusCode::BAD_REQUEST, Json(Err(error.to_string())))
        }
    }

    async fn auth_client(
        State(AppState): State<AppState>,
        Json(payload): Json<AuthClientRequest>
    ) -> (StatusCode, Json<Result<(), ServerError>>) {
    }

    pub async fn new(addr: &str, db: Arc<Db>) -> Serve<tokio::net::TcpListener, Router, Router> {
        info!("Starting web server {}", addr);
        let state = AppState { db };
        let serve_dir = ServeDir::new("web/build/client");
        let app = Router::new()
            .route(
                "/api/clients",
                get(Self::get_clients)
                    .post(Self::create_client)
                    .delete(Self::delete_client),
            )
            .route("/auth/:auth_key", post(Self::auth_client))
            .with_state(state)
            .fallback_service(serve_dir);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Cannot bind web server");
        axum::serve(listener, app)
    }
}
