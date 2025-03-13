use crate::db::Db;
use axum::{extract::State, http::StatusCode, response::Json, routing::get, serve::Serve, Router};
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use tower_http::services::ServeDir;
use uuid::Uuid;

pub struct WebServer {}

#[derive(Clone)]
struct AppState {
    db: Arc<Db>,
}

type ServerError = String;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Client {
    id: String,
    client_ip: String,
    sdn_client_ip: String,
}

#[derive(Deserialize)]
struct CreateClientRequest {
    client_ip: String,
    sdn_client_ip: String,
}

#[derive(Deserialize)]
struct DeleteClientRequest {
    id: String,
}

impl WebServer {
    async fn get_clients(State(state): State<AppState>) -> Json<Vec<Client>> {
        let clients = sqlx::query_as!(Client, "select * from clients",)
            .fetch_all(&state.db.pool)
            .await
            .expect("Cannot fetch clients");

        Json(clients)
    }

    async fn create_client(
        State(state): State<AppState>,
        Json(payload): Json<CreateClientRequest>,
    ) -> (StatusCode, Json<Result<Client, ServerError>>) {
        let id = Uuid::new_v4();
        let client = Client {
            id: id.to_string(),
            client_ip: payload.client_ip,
            sdn_client_ip: payload.sdn_client_ip,
        };

        let insert_ret =
            sqlx::query("INSERT INTO clients (id, client_ip, sdn_client_ip) VALUES (?, ?, ?)")
                .bind(&client.id)
                .bind(&client.client_ip)
                .bind(&client.sdn_client_ip)
                .execute(&state.db.pool)
                .await;
        if let Err(error) = insert_ret {
            return (StatusCode::BAD_REQUEST, Json(Err(error.to_string())));
        }
        (StatusCode::CREATED, Json(Ok(client)))
    }

    async fn delete_client(
        State(state): State<AppState>,
        Json(payload): Json<DeleteClientRequest>,
    ) -> (StatusCode, Json<Result<Vec<Client>, ServerError>>) {
        let delete_ret = sqlx::query("DELETE FROM clients WHERE id=? RETURNING *")
            .bind(&payload.id)
            .execute(&state.db.pool)
            .await;
        if let Err(error) = delete_ret {
            return (StatusCode::BAD_REQUEST, Json(Err(error.to_string())));
        }

        let clients = sqlx::query_as!(Client, "select * from clients",)
            .fetch_all(&state.db.pool)
            .await
            .expect("Cannot fetch clients");

        (StatusCode::OK, Json(Ok(clients)))
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
            .with_state(state)
            .fallback_service(serve_dir);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Cannot bind web server");
        axum::serve(listener, app)
    }
}
