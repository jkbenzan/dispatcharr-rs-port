use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{AppState, entities::user};
use sea_orm::EntityTrait;

const JWT_SECRET: &[u8] = b"dispatcharr_super_secret_temporary_key"; // In prod, load from env
const JWT_EXPIRATION_SECS: usize = 3600 * 24; // 1 day

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub is_superuser: bool,
    pub exp: usize,
}

pub struct CurrentUser(pub user::Model);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for CurrentUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let token = if let Some(auth) = auth_header {
            if auth.starts_with("Bearer ") {
                auth.trim_start_matches("Bearer ")
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        // Decode the JWT
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        ).map_err(|_| StatusCode::UNAUTHORIZED)?;

        // Fetch user from DB to verify they still exist and are active
        let user = user::Entity::find_by_id(token_data.claims.user_id)
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !user.is_active {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(CurrentUser(user))
    }
}

pub fn generate_jwt(user: &user::Model) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        user_id: user.id,
        username: user.username.clone(),
        is_superuser: user.is_superuser,
        exp: now + JWT_EXPIRATION_SECS,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    // Django PBKDF2 verification using djangohashers
    djangohashers::check_password(password, hash).unwrap_or(false)
}
