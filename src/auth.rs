use crate::{entities::user, AppState};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
            auth.split_whitespace().last().unwrap_or(auth)
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        // Decode the JWT
        let token_data = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET),
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

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
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
    use crate::entities::user;
    use chrono::{FixedOffset, Utc};
    use jsonwebtoken::{decode, DecodingKey, Validation};

    #[test]
    fn test_generate_jwt() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

        let mock_user = user::Model {
            id: 42,
            password: "hashed_password".to_string(),
            last_login: Some(now.clone()),
            is_superuser: true,
            username: "testuser".to_string(),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            email: "test@example.com".to_string(),
            is_staff: true,
            is_active: true,
            date_joined: now,
            avatar_config: None,
            user_level: 1,
            custom_properties: None,
            api_key: None,
            stream_limit: 10,
        };

        let token_result = generate_jwt(&mock_user);
        assert!(token_result.is_ok(), "JWT generation should succeed");
        let token = token_result.unwrap();

        // Verify the token can be decoded correctly
        let mut validation = Validation::default();
        validation.validate_exp = false; // We can check exp manually

        let token_data =
            decode::<Claims>(&token, &DecodingKey::from_secret(JWT_SECRET), &validation)
                .expect("Failed to decode the generated JWT");

        assert_eq!(token_data.claims.user_id, 42);
        assert_eq!(token_data.claims.username, "testuser");
        assert_eq!(token_data.claims.is_superuser, true);

        let current_time = chrono::Utc::now().timestamp() as usize;
        assert!(
            token_data.claims.exp > current_time,
            "Expiration time should be in the future"
        );
        assert!(
            token_data.claims.exp <= current_time + JWT_EXPIRATION_SECS + 5,
            "Expiration time should be roughly now + JWT_EXPIRATION_SECS"
        );
    }

    #[test]
    fn test_generate_jwt_non_superuser() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());

        let mock_user = user::Model {
            id: 84,
            password: "hashed_password".to_string(),
            last_login: Some(now.clone()),
            is_superuser: false,
            username: "regularuser".to_string(),
            first_name: "Regular".to_string(),
            last_name: "User".to_string(),
            email: "regular@example.com".to_string(),
            is_staff: false,
            is_active: true,
            date_joined: now,
            avatar_config: None,
            user_level: 1,
            custom_properties: None,
            api_key: None,
            stream_limit: 5,
        };

        let token_result = generate_jwt(&mock_user);
        assert!(token_result.is_ok(), "JWT generation should succeed");
        let token = token_result.unwrap();

        let mut validation = Validation::default();
        validation.validate_exp = false;

        let token_data =
            decode::<Claims>(&token, &DecodingKey::from_secret(JWT_SECRET), &validation)
                .expect("Failed to decode the generated JWT");

        assert_eq!(token_data.claims.user_id, 84);
        assert_eq!(token_data.claims.username, "regularuser");
        assert_eq!(token_data.claims.is_superuser, false);
    }
}
