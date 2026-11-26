use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use serde_json::json;

const JWT_SECRET: &[u8] = b"jebat-jwt-secret-change-in-production";
const TOKEN_EXPIRY_HOURS: i64 = 24;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/verify", post(verify))
        .with_state(state)
}

async fn login(
    State(_state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    if req.username == "admin" && req.password == "admin" {
        let expiry = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(TOKEN_EXPIRY_HOURS))
            .unwrap()
            .timestamp() as usize;

        let claims = Claims {
            sub: req.username,
            role: "admin".into(),
            exp: expiry,
        };

        match encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET),
        ) {
            Ok(token) => (
                StatusCode::OK,
                Json(json!({ "token": token, "role": claims.role })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("token error: {}", e) })),
            ),
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid credentials" })),
        )
    }
}

async fn verify(Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    let token = body
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match validate_token(token) {
        Ok(claims) => (
            StatusCode::OK,
            Json(json!({ "valid": true, "username": claims.sub, "role": claims.role })),
        ),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "valid": false, "error": e })),
        ),
    }
}

pub fn validate_token(token: &str) -> Result<Claims, String> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("invalid token: {}", e))
}
