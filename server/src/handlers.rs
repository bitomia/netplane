use axum::{
    extract::Path, extract::State, http::StatusCode, response::Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use bcrypt::verify;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<crate::db::Db>,
    pub server_url: String,
    pub jwt_secret: String,
}

type ServerError = String;

#[derive(Deserialize)]
pub struct CreateClientRequest {
    pub sdn_client_ip: String,
    pub netmask: String,
}

#[derive(Deserialize)]
pub struct DeleteClientRequest {
    pub id: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub email: String,
    pub exp: usize,
}

type WebResult<T> = (StatusCode, Result<Json<T>, Json<ServerError>>);

macro_rules! web_ok {
    ($expression:expr) => {
        (StatusCode::OK, Ok(Json($expression)))
    };
}

macro_rules! web_err {
    ($status:expr, $value:expr) => {
        ($status, Err(Json($value)))
    };
    ($value:expr) => {
        (StatusCode::BAD_REQUEST, Err(Json($value)))
    };
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> WebResult<LoginResponse> {
    match state.db.get_user_by_email(&payload.email).await {
        Ok(user) => {
            if verify(&payload.password, &user.password_hash).unwrap_or(false) {
                let claims = Claims {
                    email: payload.email.clone(),
                    exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
                };
                
                match encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(state.jwt_secret.as_ref()),
                ) {
                    Ok(token) => web_ok!(LoginResponse { token }),
                    Err(_) => web_err!(StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate token".to_string()),
                }
            } else {
                web_err!(StatusCode::UNAUTHORIZED, "Invalid credentials".to_string())
            }
        }
        Err(_) => web_err!(StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()),
    }
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;
    Ok(token_data.claims)
}

pub async fn get_clients(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> WebResult<Vec<crate::db::Client>> {
    if verify_jwt(auth.token(), &state.jwt_secret).is_err() {
        return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }
    match state.db.get_all_clients().await {
        Ok(clients) => web_ok!(
            clients
                .iter()
                .map(|c| crate::db::Client {
                    id: c.id.clone(),
                    auth_link_id: format!(
                        "http://{}/auth/{}",
                        state.server_url, c.auth_link_id
                    ),
                    sdn_client_ip: c.sdn_client_ip.clone(),
                    network: c.network.clone(),
                    netmask: c.netmask.clone(),
                    used: c.used,
                })
                .collect::<Vec<crate::db::Client>>()
        ),
        Err(error) => web_err!(error.to_string()),
    }
}

pub async fn create_client(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateClientRequest>,
) -> WebResult<crate::db::Client> {
    if verify_jwt(auth.token(), &state.jwt_secret).is_err() {
        return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }
    let network_address = common::calculate_network_address(
        payload.sdn_client_ip.as_str(),
        payload.netmask.as_str(),
    );
    let network_address = match network_address {
        Ok(value) => value.to_string(),
        Err(err) => {
            return web_err!(format!("Invalid netmask or IP: {}", err));
        }
    };

    let id = Uuid::new_v4();
    match state
        .db
        .create_client(
            &id.to_string().as_str(),
            &payload.sdn_client_ip,
            &network_address.as_str(),
            &payload.netmask.as_str(),
        )
        .await
    {
        Ok(client) => web_ok!(client),
        Err(error) => web_err!(error.to_string()),
    }
}

pub async fn delete_client(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<DeleteClientRequest>,
) -> WebResult<Vec<crate::db::Client>> {
    if verify_jwt(auth.token(), &state.jwt_secret).is_err() {
        return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }
    let delete_ret = state.db.delete_client(&payload.id).await;
    if let Err(error) = delete_ret {
        return web_err!(StatusCode::BAD_REQUEST, error.to_string());
    }

    match state.db.get_all_clients().await {
        Ok(clients) => web_ok!(clients),
        Err(error) => web_err!(error.to_string()),
    }
}

pub async fn auth_client(
    State(state): State<AppState>,
    Path(auth_link_id): Path<String>,
    Json(payload): Json<common::AuthClientRequest>,
) -> (StatusCode, Result<String, Json<ServerError>>) {
    match state
        .db
        .auth_client(&auth_link_id, &payload.public_key)
        .await
    {
        Ok(auth_key) => (StatusCode::OK, Ok(auth_key)),
        Err(error) => web_err!(error.to_string()),
    }
}