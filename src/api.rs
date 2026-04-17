use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use sea_orm::Set;
use chrono::Utc;

// --------------------------------------------------------
// DATA SHAPES FOR THE FRONTEND
// --------------------------------------------------------

/// 1. FLAT ARRAY: Solves the `TypeError: .reduce is not a function`
pub async fn get_flat_array() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_timezones() -> Json<Value> {
    Json(json!([
        {"value": "UTC", "label": "UTC/GMT"},
        {"value": "America/New_York", "label": "America/New_York"},
        {"value": "America/Chicago", "label": "America/Chicago"},
        {"value": "America/Denver", "label": "America/Denver"},
        {"value": "America/Los_Angeles", "label": "America/Los_Angeles"},
        {"value": "Europe/London", "label": "Europe/London"},
        {"value": "Europe/Berlin", "label": "Europe/Berlin"}
    ]))
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
use crate::entities::{user, channel, m3u_account, epg_source, channel_group, channel_profile, stream};
use crate::{m3u, epg};
use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::IntoResponse,
};
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter,
    ConnectionTrait, Statement, PaginatorTrait, QuerySelect, QueryOrder
};
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
        "user_level": u.user_level,
        "stream_limit": u.stream_limit,
        "channel_profiles": [],
        "hide_adult_content": false,
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

    Ok(Json(json!({ "initialized": has_superuser })))
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
    println!("🔐 Login attempt for user: '{}'", payload.username);

    let user = match user::Entity::find()
        .filter(user::Column::Username.eq(&payload.username))
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            println!("❌ User '{}' not found in database", payload.username);
            return Err(StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            println!("❌ DB Error during login: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    println!("✅ User found: {}", user.username);
    
    let is_valid = verify_password(&user.password, &payload.password);
    println!("🔑 Password Verify Result: {}", is_valid);

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if !user.is_active {
        println!("❌ User is completely inactive!");
        return Err(StatusCode::UNAUTHORIZED);
    }

    println!("🎉 Login Success for {}", user.username);

    let token = generate_jwt(&user).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ 
        "access": token, 
        "refresh": token, // We use the same for now
        "user": { "id": user.id, "username": user.username, "is_superuser": user.is_superuser, "is_staff": user.is_staff }
    })))
}

#[derive(serde::Deserialize)]
pub struct RefreshRequest {
    pub refresh: String,
}

pub async fn refresh_token(Json(payload): Json<RefreshRequest>) -> Json<Value> {
    Json(json!({ "access": payload.refresh }))
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
pub async fn get_channels(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let page: u64 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u64 = 50; 
    let offset = (page.saturating_sub(1)) * page_size;

    let count = channel::Entity::find().count(&state.db).await.unwrap_or(0);

    let channels = match channel::Entity::find()
        .order_by_asc(channel::Column::Id)
        .limit(page_size)
        .offset(offset)
        .all(&state.db).await {
            Ok(c) => c,
            Err(_) => vec![],
        };

    let mut results = vec![];
    for ch in channels {
        let mut ch_json = serde_json::to_value(&ch).unwrap();
        
        let groups = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT channelgroup_id FROM dispatcharr_channels_channel_groups WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        let group_ids: Vec<i64> = groups.into_iter().filter_map(|gr| gr.try_get("", "channelgroup_id").ok()).collect();
        ch_json["groups"] = json!(group_ids);

        let profiles = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT channelprofile_id FROM dispatcharr_channels_channel_channel_profiles WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        let profile_ids: Vec<i64> = profiles.into_iter().filter_map(|pr| pr.try_get("", "channelprofile_id").ok()).collect();
        ch_json["channel_profiles"] = json!(profile_ids);

        let epg = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT epgsource_id FROM dispatcharr_channels_channel_epg_sources WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        let epg_ids: Vec<i64> = epg.into_iter().filter_map(|e| e.try_get("", "epgsource_id").ok()).collect();
        ch_json["epg_sources"] = json!(epg_ids);

        let stream_links = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT stream_id FROM dispatcharr_channels_channelstream WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        
        let mut streams_arr = vec![];
        for link in stream_links {
            if let Ok(stream_id) = link.try_get::<i64>("", "stream_id") {
                if let Ok(Some(stream_obj)) = stream::Entity::find_by_id(stream_id).one(&state.db).await {
                    streams_arr.push(stream_obj);
                }
            }
        }
        ch_json["streams"] = json!(streams_arr);

        results.push(ch_json);
    }

    let has_next = (offset + page_size) < count;
    let next_page = if has_next { Some(format!("/api/channels/channels/?page={}", page + 1)) } else { None };
    let prev_page = if page > 1 { Some(format!("/api/channels/channels/?page={}", page - 1)) } else { None };

    // Important: React heavily relies on exact counts to build datagrid pagination.
    Json(json!({
        "count": count,
        "next": next_page,
        "previous": prev_page,
        "results": results
    }))
}

pub async fn get_notifications() -> Json<Value> { 
    Json(json!({ "notifications": [] }))
}

pub async fn post_stub() -> Json<Value> {
    Json(json!({ "id": 9999, "success": true, "message": "created mock" }))
}

pub async fn get_useragents() -> Json<Value> { get_paginated_object().await }
pub async fn get_streamprofiles() -> Json<Value> { get_paginated_object().await }
pub async fn get_dashboard_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let channels_count = channel::Entity::find().count(&state.db).await.unwrap_or(0);
    let streams_count = stream::Entity::find().count(&state.db).await.unwrap_or(0);
    let accounts_count = m3u_account::Entity::find().count(&state.db).await.unwrap_or(0);
    let sources_count = epg_source::Entity::find().count(&state.db).await.unwrap_or(0);
    
    Json(json!({
        "channels": channels_count,
        "streams": streams_count,
        "m3u_accounts": accounts_count,
        "epg_sources": sources_count,
        "active_users": 1,
        "system_health": "Healthy",
        "cpu_usage": 0,
        "memory_usage": 0
    }))
}

pub async fn get_channel_groups(State(state): State<Arc<AppState>>) -> Json<Value> {
    let results = channel_group::Entity::find().all(&state.db).await.unwrap_or_default();
    Json(json!(results))
}

pub async fn get_channel_profiles(State(state): State<Arc<AppState>>) -> Json<Value> {
    let results = channel_profile::Entity::find().all(&state.db).await.unwrap_or_default();
    Json(json!(results))
}

pub async fn get_streams(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let page: u64 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u64 = 50; 
    let offset = (page.saturating_sub(1)) * page_size;

    let count = stream::Entity::find().count(&state.db).await.unwrap_or(0);

    let results = stream::Entity::find()
        .order_by_asc(stream::Column::Id)
        .limit(page_size)
        .offset(offset)
        .all(&state.db).await.unwrap_or_default();

    let has_next = (offset + page_size) < count;
    let next_page = if has_next { Some(format!("/api/channels/streams/?page={}", page + 1)) } else { None };
    let prev_page = if page > 1 { Some(format!("/api/channels/streams/?page={}", page - 1)) } else { None };

    // Standard paginator string mapping for DataGrids
    Json(json!({
        "count": count,
        "next": next_page,
        "previous": prev_page,
        "results": results
    }))
}

pub async fn get_ids_stub() -> Json<Value> { get_flat_array().await }

pub async fn get_m3u_accounts(State(state): State<Arc<AppState>>) -> Json<Value> {
    let accounts = match m3u_account::Entity::find().all(&state.db).await {
        Ok(a) => a,
        Err(_) => vec![],
    };
    Json(json!(accounts))
}

pub async fn add_m3u_account(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Undefined").to_string();
    let account_type = payload.get("account_type").and_then(|v| v.as_str()).unwrap_or("XC").to_string();
    
    let server_url = payload.get("server_url").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
    let username = payload.get("username").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
    let password = payload.get("password").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
    
    let max_streams = match payload.get("max_streams") {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(1) as i32,
        Some(Value::String(s)) => s.parse::<i32>().unwrap_or(1),
        _ => 1,
    };
    
    let is_active = match payload.get("is_active") {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s.to_lowercase() == "true" || s == "1",
        _ => true,
    };

    let new_account = m3u_account::ActiveModel {
        name: Set(name),
        account_type: Set(account_type),
        server_url: Set(server_url),
        username: Set(username),
        password: Set(password),
        max_streams: Set(max_streams),
        is_active: Set(is_active),
        created_at: Set(Utc::now().into()),
        status: Set("idle".to_string()),
        priority: Set(1),
        locked: Set(false),
        stale_stream_days: Set(3),
        refresh_interval: Set(24),
        ..Default::default()
    };

    match m3u_account::Entity::insert(new_account).exec(&state.db).await {
        Ok(_) => {
            let accounts = m3u_account::Entity::find().all(&state.db).await.unwrap_or_default();
            (StatusCode::CREATED, Json(json!(accounts)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    }
}

pub async fn get_m3u_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    match m3u_account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(acc)) => (StatusCode::OK, Json(json!(acc))),
        _ => (StatusCode::NOT_FOUND, Json(json!({"error": "Not Found"}))),
    }
}

pub async fn get_epg_sources(State(state): State<Arc<AppState>>) -> Json<Value> {
    let sources = match epg_source::Entity::find().all(&state.db).await {
        Ok(s) => s,
        Err(_) => vec![],
    };
    Json(json!(sources))
}

pub async fn get_epg_source(
    State(state): State<Arc<AppState>>,
    Path(source_id): Path<i64>,
) -> impl IntoResponse {
    match epg_source::Entity::find_by_id(source_id).one(&state.db).await {
        Ok(Some(src)) => (StatusCode::OK, Json(json!(src))),
        _ => (StatusCode::NOT_FOUND, Json(json!({"error": "Not Found"}))),
    }
}


pub async fn get_epgdata() -> Json<Value> { get_flat_array().await }

pub async fn refresh_m3u_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    let account = match m3u_account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(acc)) => acc,
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "Account not found"}))),
    };

    let url = if account.account_type == "XC" {
        format!("{}/get.php?username={}&password={}&type=m3u_plus&output=ts",
            account.server_url.as_deref().unwrap_or_default().trim_end_matches('/'),
            account.username.as_deref().unwrap_or_default(),
            account.password.as_deref().unwrap_or_default()
        )
    } else {
        account.server_url.clone().unwrap_or_default()
    };

    if !url.is_empty() {
        let db_clone = state.db.clone();
        tokio::spawn(async move {
            if let Err(e) = m3u::fetch_and_parse_m3u(&db_clone, &url, account_id).await {
                eprintln!("Failed to parse M3U Task: {}", e);
            }
        });
        (StatusCode::ACCEPTED, Json(json!({"status": "M3U refresh task started"})))
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "No server URL"})))
    }
}

pub async fn refresh_epg_source(
    State(state): State<Arc<AppState>>,
    Path(source_id): Path<i64>,
) -> impl IntoResponse {
    let source = match epg_source::Entity::find_by_id(source_id).one(&state.db).await {
        Ok(Some(src)) => src,
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "Source not found"}))),
    };

    if let Some(url) = source.url.clone() {
        let db_clone = state.db.clone();
        tokio::spawn(async move {
            if let Err(e) = epg::refresh_all_guides(&db_clone, &url, source_id).await {
                eprintln!("Failed to parse EPG Task: {}", e);
            }
        });
        (StatusCode::ACCEPTED, Json(json!({"status": "EPG refresh task started"})))
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "No server URL"})))
    }
}

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