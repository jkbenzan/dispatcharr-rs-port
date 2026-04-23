use crate::{entities::user, AppState};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use crate::{AppState, entities::user};
use sea_orm::EntityTrait;

static JWT_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

fn jwt_secret() -> &'static [u8] {
    JWT_SECRET.get_or_init(|| {
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set")
            .into_bytes()
    })
}

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
            auth.split_whitespace().last().unwrap_or(auth)
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        // Decode the JWT
        let token_data = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret()),
            &Validation::default(),
        ) {
            Ok(d) => d,
            Err(e) => {
                println!("JWT Decode error: {:?}", e);
                return Err(StatusCode::UNAUTHORIZED);
            }
        };

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

    encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret()))
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    // Django PBKDF2 verification using djangohashers
    djangohashers::check_password(password, hash).unwrap_or(false)
}

pub fn hash_password(password: &str) -> String {
    djangohashers::make_password(password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn create_mock_user() -> user::Model {
        user::Model {
            id: 1,
            password: "hashed_password".to_string(),
            last_login: None,
            is_superuser: true,
            username: "testuser".to_string(),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            email: "test@example.com".to_string(),
            is_staff: true,
            is_active: true,
            date_joined: chrono::Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
            avatar_config: None,
            user_level: 1,
            custom_properties: None,
            api_key: None,
            stream_limit: 10,
        }
    }

    #[test]
    fn test_generate_jwt() {
        let user = create_mock_user();
        let token = generate_jwt(&user).expect("Should generate JWT");

        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        ).expect("Should decode JWT");

        assert_eq!(decoded.claims.user_id, 1);
        assert_eq!(decoded.claims.username, "testuser");
        assert_eq!(decoded.claims.is_superuser, true);
        assert!(decoded.claims.exp > chrono::Utc::now().timestamp() as usize);
    }
}
