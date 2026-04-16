use axum::Json;
use serde_json::{json, Value};

// --------------------------------------------------------
// DATA SHAPES FOR THE FRONTEND
// --------------------------------------------------------

/// 1. FLAT ARRAY: Solves the `TypeError: .reduce is not a function`
pub async fn get_flat_array() -> Json<Value> {
    Json(json!([]))
}

/// 2. DRF OBJECT: Solves the `TypeError: Cannot read properties of undefined (reading 'filter')`
pub async fn get_paginated_object() -> Json<Value> {
    Json(json!({
        "count": 0,
        "next": null,
        "previous": null,
        "results": []
    }))
}

// --------------------------------------------------------
// CORE SETTINGS & USER STATE
// Solves the `TypeError: Cannot read properties of undefined (reading 'length')`
// --------------------------------------------------------

pub async fn get_core_settings() -> Json<Value> {
    Json(json!({
        "app_name": "Dispatcharr",
        "proxy_enabled": true,
        "registration_enabled": false,
        "backend_url": "",
        "version": "0.22.1",
        "maintenance_mode": false,
        // Empty arrays prevent the UI from crashing when mapping/measuring length
        "channel_profiles": [],
        "stream_profiles": [],
        "user_agents": [],
        "notification_types": [],
        "providers": [] 
    }))
}

use crate::{AppState, auth::{CurrentUser, generate_jwt, verify_password}};
use crate::entities::{user, channel, m3u_account, epg_source};
use axum::{extract::State, Json, http::StatusCode};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use std::sync::Arc;

pub async fn get_current_user(current_user: CurrentUser) -> Json<Value> {
    let u = current_user.0;
    Json(json!({
        "id": u.id,
        "username": u.username,
        "email": u.email,
        "is_superuser": u.is_superuser,
        "is_active": u.is_active,
        "is_staff": u.is_staff,
        "permissions": ["*"], 
        "profile": u.custom_properties.unwrap_or(json!({
            "theme": "dark",
            "language": "en",
            "navigation_order": [], 
            "hidden_nav_items": []
        }))
    }))
}

pub async fn get_core_version() -> Json<Value> {
    Json(json!({ "version": "0.22.1", "name": "Dispatcharr" }))
}

pub async fn check_superuser(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    // Check if any superuser exists
    let has_superuser = user::Entity::find()
        .filter(user::Column::IsSuperuser.eq(true))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    Json(json!({ "initialized": has_superuser }))
}

#[derive(serde::Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<Value>, StatusCode> {
    let user = user::Entity::find()
        .filter(user::Column::Username.eq(&payload.username))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !verify_password(&user.password, &payload.password) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if !user.is_active {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = generate_jwt(&user).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ 
        "access": token, 
        "refresh": token, // We use the same for now
        "user": { "id": user.id, "username": user.username, "is_superuser": user.is_superuser, "is_staff": user.is_staff }
    })))
}

pub async fn refresh_token() -> Json<Value> {
    // Simple stub: require re-login for real refresh logic later
    Json(json!({}))
}

pub async fn logout() -> Json<Value> {
    Json(json!({"success": true}))
}

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}

// --------------------------------------------------------
// STRICT ENDPOINT MAPPINGS
// --------------------------------------------------------

// THESE REQUIRE DRF PAGINATED OBJECTS {"count": 0, "results": []}
pub async fn get_channels(State(state): State<Arc<AppState>>) -> Json<Value> {
    let channels = match channel::Entity::find().all(&state.db).await {
        Ok(c) => c,
        Err(_) => vec![],
    };

    Json(json!({
        "count": channels.len(),
        "next": null,
        "previous": null,
        "results": channels
    }))
}

pub async fn get_notifications() -> Json<Value> { get_paginated_object().await }
pub async fn get_useragents() -> Json<Value> { get_paginated_object().await }
pub async fn get_streamprofiles() -> Json<Value> { get_paginated_object().await }

// THESE REQUIRE FLAT ARRAYS []
pub async fn get_channel_groups() -> Json<Value> { get_flat_array().await }
pub async fn get_profiles() -> Json<Value> { get_flat_array().await }
pub async fn get_ids_stub() -> Json<Value> { get_flat_array().await }

pub async fn get_m3u_accounts(State(state): State<Arc<AppState>>) -> Json<Value> {
    let accounts = match m3u_account::Entity::find().all(&state.db).await {
        Ok(a) => a,
        Err(_) => vec![],
    };
    Json(json!(accounts))
}

pub async fn get_epg_sources(State(state): State<Arc<AppState>>) -> Json<Value> {
    let sources = match epg_source::Entity::find().all(&state.db).await {
        Ok(s) => s,
        Err(_) => vec![],
    };
    Json(json!(sources))
}

pub async fn get_epgdata() -> Json<Value> { get_flat_array().await }

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_core_version() {
        let response = get_core_version().await;
        let body = response.0;

        assert_eq!(body["version"], "0.22.1");
        assert_eq!(body["name"], "Dispatcharr");
    }
}