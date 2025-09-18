use axum::{
    extract::Path,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
};
use axum_extra::{TypedHeader, headers::Cookie};
use bcrypt::verify;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<crate::db::Db>,
    pub server_stats: Arc<crate::server::ServerStats>,
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
    pub success: bool,
}

#[derive(Serialize)]
pub struct UserDataResponse {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub email: String,
    pub exp: usize,
}

#[derive(Serialize)]
pub struct ServerStatsResponse {
    pub transport_mode: String,
    pub in_bytes: usize,
    pub out_bytes: usize,
}

type WebResult<T> = (StatusCode, Result<Json<T>, Json<ServerError>>);
type WebResultWithHeaders<T> = (StatusCode, HeaderMap, Result<Json<T>, Json<ServerError>>);

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
) -> WebResultWithHeaders<LoginResponse> {
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
                    Ok(token) => {
                        let mut headers = HeaderMap::new();
                        let cookie_value = format!(
                            "auth_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400",
                            token
                        );
                        headers.insert("Set-Cookie", cookie_value.parse().unwrap());
                        (
                            StatusCode::OK,
                            headers,
                            Ok(Json(LoginResponse { success: true })),
                        )
                    }
                    Err(_) => {
                        let headers = HeaderMap::new();
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            headers,
                            Err(Json("Failed to generate token".to_string())),
                        )
                    }
                }
            } else {
                let headers = HeaderMap::new();
                (
                    StatusCode::UNAUTHORIZED,
                    headers,
                    Err(Json("Invalid credentials".to_string())),
                )
            }
        }
        Err(_) => {
            let headers = HeaderMap::new();
            (
                StatusCode::UNAUTHORIZED,
                headers,
                Err(Json("Invalid credentials".to_string())),
            )
        }
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
    TypedHeader(cookie): TypedHeader<Cookie>,
) -> WebResult<Vec<crate::db::Client>> {
    let token = match cookie.get("auth_token") {
        Some(token) => token,
        None => return web_err!(StatusCode::UNAUTHORIZED, "No auth token".to_string()),
    };

    if verify_jwt(token, &state.jwt_secret).is_err() {
        return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }
    match state.db.get_all_clients().await {
        Ok(clients) => web_ok!(
            clients
                .iter()
                .map(|c| crate::db::Client {
                    id: c.id.clone(),
                    auth_link_id: format!("{}/auth/{}", state.server_url, c.auth_link_id),
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
    TypedHeader(cookie): TypedHeader<Cookie>,
    Json(payload): Json<CreateClientRequest>,
) -> WebResult<crate::db::Client> {
    let token = match cookie.get("auth_token") {
        Some(token) => token,
        None => return web_err!(StatusCode::UNAUTHORIZED, "No auth token".to_string()),
    };

    if verify_jwt(token, &state.jwt_secret).is_err() {
        return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string());
    }
    let network_address = netplane_common::calculate_network_address(
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
    TypedHeader(cookie): TypedHeader<Cookie>,
    Json(payload): Json<DeleteClientRequest>,
) -> WebResult<Vec<crate::db::Client>> {
    let token = match cookie.get("auth_token") {
        Some(token) => token,
        None => return web_err!(StatusCode::UNAUTHORIZED, "No auth token".to_string()),
    };

    if verify_jwt(token, &state.jwt_secret).is_err() {
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
    Json(payload): Json<netplane_common::AuthClientRequest>,
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

pub async fn get_user_data(
    State(state): State<AppState>,
    TypedHeader(cookie): TypedHeader<Cookie>,
) -> WebResult<UserDataResponse> {
    let token = match cookie.get("auth_token") {
        Some(token) => token,
        None => return web_err!(StatusCode::UNAUTHORIZED, "No auth token".to_string()),
    };

    let claims = match verify_jwt(token, &state.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
    };

    match state.db.get_user_by_email(&claims.email).await {
        Ok(user) => web_ok!(UserDataResponse {
            email: user.email,
            role: user.role,
        }),
        Err(_) => web_err!(StatusCode::UNAUTHORIZED, "User not found".to_string()),
    }
}

pub async fn logout() -> WebResultWithHeaders<LoginResponse> {
    let mut headers = HeaderMap::new();
    let cookie_value = "auth_token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    headers.insert("Set-Cookie", cookie_value.parse().unwrap());
    (
        StatusCode::OK,
        headers,
        Ok(Json(LoginResponse { success: true })),
    )
}

pub async fn get_server_stats(
    State(state): State<AppState>,
    TypedHeader(cookie): TypedHeader<Cookie>,
) -> WebResult<ServerStatsResponse> {
    let token = match cookie.get("auth_token") {
        Some(token) => token,
        None => return web_err!(StatusCode::UNAUTHORIZED, "No auth token".to_string()),
    };
    let _ = match verify_jwt(token, &state.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => return web_err!(StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
    };

    web_ok!(ServerStatsResponse {
        transport_mode: state.server_stats.transport_mode.as_string(),
        in_bytes: state
            .server_stats
            .in_bytes
            .load(std::sync::atomic::Ordering::Relaxed),
        out_bytes: state
            .server_stats
            .out_bytes
            .load(std::sync::atomic::Ordering::Relaxed),
    })
}
