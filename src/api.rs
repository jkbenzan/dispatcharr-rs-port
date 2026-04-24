use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
// --------------------------------------------------------
// DATA SHAPES FOR THE FRONTEND
// --------------------------------------------------------

/// 1. FLAT ARRAY: Solves the `TypeError: .reduce is not a function`
pub async fn get_flat_array() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_timezones() -> Json<Value> {
    Json(json!({
        "timezones": [
            {"value": "UTC", "label": "UTC/GMT"},
            {"value": "America/New_York", "label": "America/New_York"},
            {"value": "America/Chicago", "label": "America/Chicago"},
            {"value": "America/Denver", "label": "America/Denver"},
            {"value": "America/Los_Angeles", "label": "America/Los_Angeles"},
            {"value": "Europe/London", "label": "Europe/London"},
            {"value": "Europe/Berlin", "label": "Europe/Berlin"}
        ]
    }))
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
    // Django returns an array of {key, name, value} objects.
    // The frontend does settings.reduce((acc, s) => acc[s.key] = s) so we must match this format.
    Json(json!([
        { "key": "app_name", "name": "App Name", "value": "Dispatcharr" },
        { "key": "proxy_enabled", "name": "Proxy Enabled", "value": true },
        { "key": "registration_enabled", "name": "Registration Enabled", "value": false },
        { "key": "backend_url", "name": "Backend URL", "value": "" },
        { "key": "version", "name": "Version", "value": "0.22.1" },
        { "key": "maintenance_mode", "name": "Maintenance Mode", "value": false }
    ]))
}

use crate::{AppState, auth::{CurrentUser, generate_jwt, verify_password}};
use crate::entities::{user, channel, m3u_account, epg_source, channel_group, channel_profile, stream, core_settings};
use crate::{m3u, epg};
use axum::{
    extract::{Query, State, Path, Multipart},
    http::StatusCode,
    response::IntoResponse,
};
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter,
    ConnectionTrait, Statement, PaginatorTrait, QuerySelect, QueryOrder, ActiveModelTrait, Set
};
use std::sync::Arc;
use std::path::Path as StdPath;
use tokio::fs;



pub async fn get_core_version() -> Json<Value> {
    Json(json!({ "version": "0.22.1", "name": "Dispatcharr" }))
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

    let mut user = match user::Entity::find()
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

    let mut active_user: user::ActiveModel = user.clone().into();
    active_user.last_login = Set(Some(chrono::Utc::now().into()));
    if let Ok(updated) = active_user.update(&state.db).await {
        user = updated;
    }

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
    let mut public_ip = "Unknown".to_string();
    let mut country_code = "Unknown".to_string();
    let mut country_name = "Unknown".to_string();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    if let Ok(res) = client.get("http://ip-api.com/json/").send().await {
        if let Ok(json) = res.json::<serde_json::Value>().await {
            if let Some(ip) = json.get("query").and_then(|v| v.as_str()) {
                public_ip = ip.to_string();
            }
            if let Some(code) = json.get("countryCode").and_then(|v| v.as_str()) {
                country_code = code.to_string();
            }
            if let Some(name) = json.get("country").and_then(|v| v.as_str()) {
                country_name = name.to_string();
            }
        }
    }

    let local_ip = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    Json(json!({
        "authenticated": true,
        "public_ip": public_ip,
        "local_ip": local_ip,
        "country_code": country_code,
        "country_name": country_name,
        "env_mode": std::env::var("DISPATCHARR_ENV").unwrap_or_else(|_| "aio".to_string()),
        "redis_tls": {
            "enabled": false,
            "verify": true,
            "mtls": false
        },
        "postgres_tls": {
            "enabled": false,
            "ssl_mode": null,
            "mtls": false
        }
    }))
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
        ch_json["channel_groups"] = json!(group_ids);

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

pub async fn get_useragents() -> Json<Value> { get_flat_array().await }
pub async fn get_streamprofiles() -> Json<Value> { get_flat_array().await }
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
    let profiles = channel_profile::Entity::find().all(&state.db).await.unwrap_or_default();
    let mut results = vec![];
    for p in profiles {
        let mut p_json = serde_json::to_value(&p).unwrap();
        p_json["channel_groups"] = json!([]);
        p_json["auto_enable_new_groups_live"] = json!(true);
        p_json["auto_enable_new_groups_vod"] = json!(true);
        p_json["auto_enable_new_groups_series"] = json!(true);
        results.push(p_json);
    }
    Json(json!(results))
}

pub async fn get_streams(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let page: u64 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u64 = 50;
    let offset = (page.saturating_sub(1)) * page_size;

    let mut q = stream::Entity::find();
    if let Some(acc_id) = params.get("m3u_account").and_then(|p| p.parse::<i64>().ok()) {
        q = q.filter(stream::Column::M3uAccountId.eq(acc_id));
    }

    if let Some(cg) = params.get("channel_group") {
        let group_names: Vec<&str> = cg.split(',').collect();
        let groups = crate::entities::channel_group::Entity::find()
            .filter(crate::entities::channel_group::Column::Name.is_in(group_names))
            .all(&state.db).await.unwrap_or_default();
        let group_ids: Vec<i64> = groups.into_iter().map(|g| g.id).collect();
        if !group_ids.is_empty() {
            q = q.filter(stream::Column::ChannelGroupId.is_in(group_ids));
        } else {
            q = q.filter(stream::Column::Id.eq(-1));
        }
    }

    if let Some(name) = params.get("name") {
        q = q.filter(stream::Column::Name.contains(name));
    }
    
    if let Some(tvg) = params.get("tvg_id") {
        q = q.filter(stream::Column::TvgId.contains(tvg));
    }

    let count = q.clone().count(&state.db).await.unwrap_or(0);
    let streams = q.order_by_asc(stream::Column::Id)
        .limit(page_size)
        .offset(offset)
        .all(&state.db).await.unwrap_or_default();

    let mut results = vec![];
    for s in streams {
        let mut js = serde_json::to_value(&s).unwrap();
        if let Some(cg_id) = s.channel_group_id {
            js["channel_group"] = json!(cg_id);
        } else {
            js["channel_group"] = serde_json::Value::Null;
        }
        if let Some(m_id) = s.m3u_account_id {
            js["m3u_account"] = json!(m_id);
        } else {
            js["m3u_account"] = serde_json::Value::Null;
        }
        if let Some(p_id) = s.stream_profile_id {
            js["stream_profile"] = json!(p_id);
            js["stream_profile_id"] = json!(p_id);
        }
        results.push(js);
    }

    let has_next = (offset + page_size) < count;
    let next_page = if has_next { Some(format!("/api/channels/streams/?page={}", page + 1)) } else { None };
    let prev_page = if page > 1 { Some(format!("/api/channels/streams/?page={}", page - 1)) } else { None };

    Json(json!({
        "count": count,
        "next": next_page,
        "previous": prev_page,
        "results": results
    }))
}

pub async fn get_stream_ids(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let mut q = stream::Entity::find();
    if let Some(acc_id) = params.get("m3u_account").and_then(|p| p.parse::<i64>().ok()) {
        q = q.filter(stream::Column::M3uAccountId.eq(acc_id));
    }

    if let Some(cg) = params.get("channel_group") {
        let group_names: Vec<&str> = cg.split(',').collect();
        let groups = crate::entities::channel_group::Entity::find()
            .filter(crate::entities::channel_group::Column::Name.is_in(group_names))
            .all(&state.db).await.unwrap_or_default();
        let group_ids: Vec<i64> = groups.into_iter().map(|g| g.id).collect();
        if !group_ids.is_empty() {
            q = q.filter(stream::Column::ChannelGroupId.is_in(group_ids));
        } else {
            q = q.filter(stream::Column::Id.eq(-1));
        }
    }

    if let Some(name) = params.get("name") {
        q = q.filter(stream::Column::Name.contains(name));
    }
    
    if let Some(tvg) = params.get("tvg_id") {
        q = q.filter(stream::Column::TvgId.contains(tvg));
    }

    let ids = q.select_only()
        .column(stream::Column::Id)
        .order_by_asc(stream::Column::Id)
        .into_tuple::<i64>()
        .all(&state.db).await.unwrap_or_default();

    Json(json!(ids))
}

#[derive(serde::Deserialize)]
pub struct ByIdsRequest {
    pub ids: Vec<i64>,
}

pub async fn get_streams_by_ids(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ByIdsRequest>,
) -> Json<Value> {
    let streams = stream::Entity::find()
        .filter(stream::Column::Id.is_in(payload.ids))
        .all(&state.db).await.unwrap_or_default();

    let mut results = vec![];
    for s in streams {
        let mut js = serde_json::to_value(&s).unwrap();
        if let Some(cg_id) = s.channel_group_id {
            js["channel_group"] = json!(cg_id);
        } else {
            js["channel_group"] = serde_json::Value::Null;
        }
        if let Some(m_id) = s.m3u_account_id {
            js["m3u_account"] = json!(m_id);
        } else {
            js["m3u_account"] = serde_json::Value::Null;
        }
        if let Some(p_id) = s.stream_profile_id {
            js["stream_profile"] = json!(p_id);
            js["stream_profile_id"] = json!(p_id);
        }
        results.push(js);
    }
    Json(json!(results))
}

pub async fn get_stream_filter_options(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let mut q = stream::Entity::find();
    if let Some(acc_id) = params.get("m3u_account").and_then(|v| v.parse::<i64>().ok()) {
        q = q.filter(stream::Column::M3uAccountId.eq(acc_id));
    }

    let streams = q.all(&state.db).await.unwrap_or_default();
    let mut groups_set = std::collections::HashSet::new();

    for s in streams {
        if let Some(props) = s.custom_properties {
            if let Some(gt) = props.get("group_title").and_then(|v| v.as_str()) {
                if !gt.is_empty() {
                    groups_set.insert(gt.to_string());
                }
            }
        }
    }

    let mut groups: Vec<String> = groups_set.into_iter().collect();
    groups.sort();

    Json(json!({
        "groups": groups,
        "m3u_accounts": []
    }))
}

pub async fn get_channel_ids(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let ids = channel::Entity::find()
        .select_only()
        .column(channel::Column::Id)
        .into_tuple::<i64>()
        .all(&state.db)
        .await
        .unwrap_or_default();
    Json(json!(ids))
}

async fn get_channel_groups_for_account(account_id: i64, db: &sea_orm::DatabaseConnection) -> Vec<Value> {
    use crate::entities::channel_group_m3u_account;
    let mappings = channel_group_m3u_account::Entity::find()
        .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
        .all(db)
        .await
        .unwrap_or_default();

    mappings.into_iter().map(|m| {
        // The frontend uses channel_group as a plain integer key into the channelGroups store
        json!({
            "id": m.id,
            "channel_group": m.channel_group_id,
            "m3u_account_id": m.m3u_account_id,
            "enabled": m.enabled,
            "auto_channel_sync": m.auto_channel_sync,
            "is_stale": m.is_stale,
            "last_seen": m.last_seen,
        })
    }).collect()
}

async fn create_default_profile(account_id: i64, account_name: &str, max_streams: i32, db: &sea_orm::DatabaseConnection) {
    use crate::entities::m3u_account_profile;
    let profile_name = format!("{} Default", account_name);
    let new_profile = m3u_account_profile::ActiveModel {
        name: sea_orm::Set(profile_name),
        m3u_account_id: sea_orm::Set(account_id),
        is_default: sea_orm::Set(true),
        is_active: sea_orm::Set(true),
        max_streams: sea_orm::Set(max_streams),
            current_viewers: sea_orm::Set(0),
        search_pattern: sea_orm::Set("^(.*)$".to_string()),
        replace_pattern: sea_orm::Set("$1".to_string()),
        ..Default::default()
    };
    let _ = m3u_account_profile::Entity::insert(new_profile).exec(db).await;
}

fn extract_custom_props_to_root(acc: &crate::entities::m3u_account::Model, acc_json: &mut serde_json::Value) {
    if let Some(cp) = acc.custom_properties.as_ref() {
        if let Some(vod) = cp.get("enable_vod") {
            acc_json["enable_vod"] = vod.clone();
        } else {
            acc_json["enable_vod"] = serde_json::json!(false);
        }
        if let Some(auto_movie) = cp.get("auto_enable_new_groups_movies") {
            acc_json["auto_enable_new_groups_movies"] = auto_movie.clone();
        } else {
            acc_json["auto_enable_new_groups_movies"] = serde_json::json!(true);
        }
        if let Some(auto_series) = cp.get("auto_enable_new_groups_series") {
            acc_json["auto_enable_new_groups_series"] = auto_series.clone();
        } else {
            acc_json["auto_enable_new_groups_series"] = serde_json::json!(true);
        }
        if let Some(auto_live) = cp.get("auto_enable_new_groups_live") {
            acc_json["auto_enable_new_groups_live"] = auto_live.clone();
        } else {
            acc_json["auto_enable_new_groups_live"] = serde_json::json!(true);
        }
        if let Some(auto_vod) = cp.get("auto_enable_new_groups_vod") {
            acc_json["auto_enable_new_groups_vod"] = auto_vod.clone();
        } else {
            acc_json["auto_enable_new_groups_vod"] = serde_json::json!(true);
        }
    } else {
        acc_json["enable_vod"] = serde_json::json!(false);
        acc_json["auto_enable_new_groups_movies"] = serde_json::json!(true);
        acc_json["auto_enable_new_groups_series"] = serde_json::json!(true);
        acc_json["auto_enable_new_groups_live"] = serde_json::json!(true);
        acc_json["auto_enable_new_groups_vod"] = serde_json::json!(true);
    }
}

fn apply_custom_props_from_payload(payload: &serde_json::Value, active: &mut crate::entities::m3u_account::ActiveModel, existing_props: Option<&serde_json::Value>) -> Option<bool> {
    let mut custom = existing_props.cloned().unwrap_or(serde_json::json!({}));
    let mut updated = false;
    let mut enable_vod_ret = None;

    if let Some(v) = payload.get("enable_vod") {
        let b = if let Some(b) = v.as_bool() { Some(b) } else if let Some(s) = v.as_str() { Some(s == "true") } else { None };
        if let Some(b) = b {
            custom["enable_vod"] = serde_json::json!(b);
            enable_vod_ret = Some(b);
            updated = true;
        }
    }

    let keys = ["auto_enable_new_groups_live", "auto_enable_new_groups_vod", "auto_enable_new_groups_series", "auto_enable_new_groups_movies"];
    for k in keys {
        if let Some(v) = payload.get(k) {
            let b = if let Some(b) = v.as_bool() { Some(b) } else if let Some(s) = v.as_str() { Some(s == "true") } else { None };
            if let Some(b) = b {
                custom[k] = serde_json::json!(b);
                updated = true;
            }
        }
    }

    if updated {
        active.custom_properties = sea_orm::Set(Some(custom));
    }
    enable_vod_ret
}

pub async fn get_m3u_accounts(State(state): State<Arc<AppState>>) -> Json<Value> {
    use sea_orm::QueryOrder;
    let accounts = match m3u_account::Entity::find()
        .order_by_asc(m3u_account::Column::Priority)
        .all(&state.db).await {
        Ok(a) => a,
        Err(_) => vec![],
    };
    let mut results = vec![];
    for acc in accounts {
        let mut acc_json = serde_json::to_value(&acc).unwrap();
        extract_custom_props_to_root(&acc, &mut acc_json);
        acc_json["profiles"] = json!([]);
        acc_json["filters"] = json!([]);
        acc_json["groups"] = json!([]);
        acc_json["channel_groups"] = json!(get_channel_groups_for_account(acc.id, &state.db).await);
        acc_json["streams"] = json!([]);
        results.push(acc_json);
    }
    Json(json!(results))
}

fn sanitize_filename(filename: &str) -> String {
    let filename = filename.split(|c| c == '/' || c == '\\').last().unwrap_or(filename);
    let sanitized: String = filename.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect();
    if sanitized.is_empty() || sanitized.replace(".", "").is_empty() {
        "uploaded.m3u".to_string()
    } else {
        sanitized
    }
}

pub async fn add_m3u_account(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let content_type = req.headers().get(axum::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");

    let mut payload = json!({});
    let mut file_path: Option<String> = None;

    if content_type.starts_with("multipart/form-data") {
        use axum::extract::FromRequest;
        if let Ok(mut multipart) = axum::extract::Multipart::from_request(req, &state).await {
            while let Ok(Some(field)) = multipart.next_field().await {
                let name = field.name().unwrap_or("").to_string();
                if name == "file" {
                    if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                        if let Ok(data) = field.bytes().await {
                            let sanitized = sanitize_filename(&file_name);
                            let path = format!("./data/uploads/m3us/{}", sanitized);
                            let _ = std::fs::create_dir_all("./data/uploads/m3us");
                            let _ = std::fs::write(&path, data);
                            file_path = Some(path);
                        }
                    }
                } else {
                    if let Ok(text) = field.text().await {
                        if text == "true" {
                            payload[&name] = json!(true);
                        } else if text == "false" {
                            payload[&name] = json!(false);
                        } else if let Ok(num) = text.parse::<i64>() {
                            payload[&name] = json!(num);
                        } else {
                            payload[&name] = json!(text);
                        }
                    }
                }
            }
        }
    } else {
        if let Ok(bytes) = axum::body::to_bytes(req.into_body(), usize::MAX).await {
            payload = serde_json::from_slice(&bytes).unwrap_or(json!({}));
        }
    }

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

    let mut new_acc = m3u_account::ActiveModel {
        name: sea_orm::Set(name),
        account_type: sea_orm::Set(account_type),
        server_url: sea_orm::Set(server_url),
        file_path: sea_orm::Set(file_path.clone()),
        username: sea_orm::Set(username),
        password: sea_orm::Set(password),
        max_streams: sea_orm::Set(max_streams),
        is_active: sea_orm::Set(true),
        status: sea_orm::Set("pending".to_string()),
        created_at: sea_orm::Set(chrono::Utc::now().into()),
        updated_at: sea_orm::Set(Some(chrono::Utc::now().into())),
        stale_stream_days: sea_orm::Set(7),
        locked: sea_orm::Set(false),
        priority: sea_orm::Set(1),
        refresh_interval: sea_orm::Set(24),
        ..Default::default()
    };

    let enable_vod = apply_custom_props_from_payload(&payload, &mut new_acc, None).unwrap_or(false);

    match m3u_account::Entity::insert(new_acc).exec(&state.db).await {
        Ok(res) => {
            let account_id = res.last_insert_id;
            if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(&state.db).await {
                let url = if acc.account_type == "XC" {
                    format!("{}/get.php?username={}&password={}&type=m3u_plus&output=ts",
                        acc.server_url.as_deref().unwrap_or_default().trim_end_matches('/'),
                        acc.username.as_deref().unwrap_or_default(),
                        acc.password.as_deref().unwrap_or_default()
                    )
                } else {
                    acc.server_url.clone().unwrap_or_default()
                };

                let _ = state.ws_sender.send(json!({
                    "channel": "updates",
                    "event": "update",
                    "data": {
                        "type": "playlist_created",
                        "playlist_id": account_id
                    }
                }));


                if !url.is_empty() || file_path.is_some() {
                    let db_clone = state.db.clone();
                    let is_xc = acc.account_type == "XC";
                    let ws_clone = state.ws_sender.clone();
                    let final_url = url.clone();
                    let file_path_clone = file_path.clone();
                    
                    tokio::spawn(async move {
                        let error_msg = if is_xc {
                            let mut err = None;
                            let is_success = match crate::m3u::fetch_and_parse_xc_categories(&db_clone, account_id, Some(ws_clone)).await {
                                Err(e) => {
                                    err = Some(format!("Failed to parse XC categories: {}", e));
                                    false
                                },
                                Ok(_) => true,
                            };
                            if is_success && enable_vod {
                                let _ = crate::m3u::fetch_and_parse_xc_vod(&db_clone, account_id).await;
                                let _ = crate::m3u::fetch_and_parse_xc_series(&db_clone, account_id).await;
                            }
                            err
                        } else {
                            let parse_url = file_path_clone.unwrap_or(final_url);
                            match crate::m3u::fetch_and_parse_m3u(&db_clone, &parse_url, account_id, true, Some(ws_clone)).await {
                                Err(e) => Some(format!("Failed to parse M3U: {}", e)),
                                Ok(_) => None,
                            }
                        };

                        if let Some(msg) = error_msg {
                            eprintln!("{}", msg);
                            if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(&db_clone).await {
                                let mut active: m3u_account::ActiveModel = acc.into();
                                active.status = sea_orm::Set("failed".to_string());
                                active.last_message = sea_orm::Set(Some(msg.chars().take(255).collect()));
                                let _ = active.update(&db_clone).await;
                            }
                        }
                    });
                }

                // Create default profile (mirrors Django post_save signal)
                create_default_profile(acc.id, &acc.name, acc.max_streams, &state.db).await;

                let mut acc_json = serde_json::to_value(&acc).unwrap();
                extract_custom_props_to_root(&acc, &mut acc_json);
                acc_json["profiles"] = json!([]);
                acc_json["filters"] = json!([]);
                acc_json["groups"] = json!([]);
                acc_json["channel_groups"] = json!(get_channel_groups_for_account(acc.id, &state.db).await);
                acc_json["streams"] = json!([]);
                (StatusCode::OK, Json(acc_json))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to retrieve saved account"})))
            }
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    }
}

pub async fn get_m3u_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    match m3u_account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(acc)) => {
            let mut acc_json = serde_json::to_value(&acc).unwrap();
            extract_custom_props_to_root(&acc, &mut acc_json);
            acc_json["profiles"] = json!([]);
            acc_json["filters"] = json!([]);
            acc_json["groups"] = json!([]);
            acc_json["channel_groups"] = json!(get_channel_groups_for_account(acc.id, &state.db).await);
            acc_json["streams"] = json!([]);
            (StatusCode::OK, Json(acc_json))
        },
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
        let is_xc = account.account_type == "XC";
        let ws_clone_outer = state.ws_sender.clone();
        tokio::spawn(async move {
            let error_msg = if is_xc {
                let ws_clone = ws_clone_outer.clone();
                let xc_err = if let Err(e) = m3u::fetch_and_parse_xc(&db_clone, account_id, Some(ws_clone)).await {
                    Some(format!("Failed to parse XC API: {}", e))
                } else {
                    None
                };
                if xc_err.is_none() {
                    let _ = m3u::fetch_and_parse_xc_vod(&db_clone, account_id).await;
                    let _ = m3u::fetch_and_parse_xc_series(&db_clone, account_id).await;
                }
                xc_err
            } else {
                let ws_clone = ws_clone_outer.clone();
                match m3u::fetch_and_parse_m3u(&db_clone, &url, account_id, false, Some(ws_clone)).await {
                    Err(e) => Some(format!("Failed to parse M3U: {}", e)),
                    Ok(_) => None,
                }
            };

            if let Some(msg) = error_msg {
                eprintln!("{}", msg);
                if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(&db_clone).await {
                    let mut active: m3u_account::ActiveModel = acc.into();
                    active.status = sea_orm::Set("failed".to_string());
                    active.last_message = sea_orm::Set(Some(msg.chars().take(255).collect()));
                    let _ = active.update(&db_clone).await;
                }
            }
        });
        (StatusCode::ACCEPTED, Json(json!({"status": "M3U refresh task started"})))
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "No server URL"})))
    }
}

pub async fn refresh_vod(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    let account = match m3u_account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(acc)) => acc,
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "Account not found"}))),
    };

    if account.account_type != "XC" {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "VOD refresh is only available for XtreamCodes accounts"})));
    }

    let mut vod_enabled = false;
    if let Some(props) = &account.custom_properties {
        if let Some(enabled) = props.get("enable_vod").and_then(|v| v.as_bool()) {
            vod_enabled = enabled;
        }
    }

    if !vod_enabled {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "VOD is not enabled for this account"})));
    }

    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::m3u::fetch_and_parse_xc_vod(&db_clone, account_id).await {
            eprintln!("Failed to refresh VOD: {}", e);
        }
        if let Err(e) = crate::m3u::fetch_and_parse_xc_series(&db_clone, account_id).await {
            eprintln!("Failed to refresh Series: {}", e);
        }
    });

    (StatusCode::ACCEPTED, Json(json!({"message": format!("VOD refresh initiated for account {}", account.name)})))
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

pub async fn update_m3u_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let content_type = req.headers().get(axum::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");

    let mut payload = json!({});
    let mut file_path: Option<String> = None;

    if content_type.starts_with("multipart/form-data") {
        use axum::extract::FromRequest;
        if let Ok(mut multipart) = axum::extract::Multipart::from_request(req, &state).await {
            while let Ok(Some(field)) = multipart.next_field().await {
                let name = field.name().unwrap_or("").to_string();
                if name == "file" {
                    if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                        if let Ok(data) = field.bytes().await {
                            let sanitized = sanitize_filename(&file_name);
                            let path = format!("./data/uploads/m3us/{}", sanitized);
                            let _ = std::fs::create_dir_all("./data/uploads/m3us");
                            let _ = std::fs::write(&path, data);
                            file_path = Some(path);
                        }
                    }
                } else {
                    if let Ok(text) = field.text().await {
                        if text == "true" {
                            payload[&name] = json!(true);
                        } else if text == "false" {
                            payload[&name] = json!(false);
                        } else if let Ok(num) = text.parse::<i64>() {
                            payload[&name] = json!(num);
                        } else {
                            payload[&name] = json!(text);
                        }
                    }
                }
            }
        }
    } else {
        if let Ok(bytes) = axum::body::to_bytes(req.into_body(), usize::MAX).await {
            payload = serde_json::from_slice(&bytes).unwrap_or(json!({}));
        }
    }

    let acc = match m3u_account::Entity::find_by_id(account_id).one(&state.db).await {
        Ok(Some(a)) => a,
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "Account not found"}))),
    };

    let mut active: m3u_account::ActiveModel = acc.clone().into();

    if let Some(is_active) = payload.get("is_active").and_then(|v| v.as_bool()) {
        active.is_active = sea_orm::Set(is_active);
    }

    if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
        active.name = sea_orm::Set(name.to_string());
    }

    if let Some(url) = payload.get("server_url").and_then(|v| v.as_str()) {
        active.server_url = sea_orm::Set(Some(url.to_string()));
    }

    if let Some(user) = payload.get("username").and_then(|v| v.as_str()) {
        active.username = sea_orm::Set(Some(user.to_string()));
    }

    if let Some(pass) = payload.get("password").and_then(|v| v.as_str()) {
        active.password = sea_orm::Set(Some(pass.to_string()));
    }

    if let Some(max_streams) = payload.get("max_streams") {
        if let Some(n) = max_streams.as_i64() {
            active.max_streams = sea_orm::Set(n as i32);
        } else if let Some(s) = max_streams.as_str() {
            if let Ok(n) = s.parse::<i32>() {
                active.max_streams = sea_orm::Set(n);
            }
        }
    }

    if let Some(account_type) = payload.get("account_type").and_then(|v| v.as_str()) {
        active.account_type = sea_orm::Set(account_type.to_string());
    }

    if let Some(path) = file_path {
        active.file_path = sea_orm::Set(Some(path));
    }

    let enable_vod_opt = apply_custom_props_from_payload(&payload, &mut active, acc.custom_properties.as_ref());

    if let Ok(updated) = active.update(&state.db).await {
        if enable_vod_opt == Some(true) && updated.account_type == "XC" {
            let db_clone = state.db.clone();
            tokio::spawn(async move {
                let _ = crate::m3u::fetch_and_parse_xc_vod(&db_clone, account_id).await;
                let _ = crate::m3u::fetch_and_parse_xc_series(&db_clone, account_id).await;
            });
        }

        let mut acc_json = serde_json::to_value(&updated).unwrap();
        extract_custom_props_to_root(&updated, &mut acc_json);
        acc_json["profiles"] = json!([]);
        acc_json["filters"] = json!([]);
        acc_json["groups"] = json!([]);
        acc_json["channel_groups"] = json!(get_channel_groups_for_account(updated.id, &state.db).await);
        acc_json["streams"] = json!([]);
        (StatusCode::OK, Json(acc_json))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to update"})))
    }
}

pub async fn delete_m3u_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, ConnectionTrait};
    use sea_orm::Statement;

    // channelstream using raw sql
    if let Err(e) = state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM dispatcharr_channels_channelstream WHERE stream_id IN (SELECT id FROM dispatcharr_channels_stream WHERE m3u_account_id = $1)",
        vec![account_id.into()]
    )).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete channelstream by stream: {}", e)})));
    }

    // channelstream by channel
    if let Err(e) = state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM dispatcharr_channels_channelstream WHERE channel_id IN (SELECT id FROM dispatcharr_channels_channel WHERE auto_created_by_id = $1)",
        vec![account_id.into()]
    )).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete channelstream by channel: {}", e)})));
    }

    // channelprofilemembership
    if let Err(e) = state.db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM dispatcharr_channels_channelprofilemembership WHERE channel_id IN (SELECT id FROM dispatcharr_channels_channel WHERE auto_created_by_id = $1)",
        vec![account_id.into()]
    )).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete channelprofilemembership: {}", e)})));
    }

    // channel
    if let Err(e) = crate::entities::channel::Entity::delete_many()
        .filter(crate::entities::channel::Column::AutoCreatedById.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete auto-created channels: {}", e)})));
    }

    // stream
    if let Err(e) = crate::entities::stream::Entity::delete_many()
        .filter(crate::entities::stream::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete stream: {}", e)})));
    }

    // channelgroupm3uaccount
    if let Err(e) = crate::entities::channel_group_m3u_account::Entity::delete_many()
        .filter(crate::entities::channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete channel_group_m3u_account: {}", e)})));
    }

    // m3u_account_profile
    if let Err(e) = crate::entities::m3u_account_profile::Entity::delete_many()
        .filter(crate::entities::m3u_account_profile::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete m3u_account_profile: {}", e)})));
    }

    // m3u_filter
    if let Err(e) = crate::entities::m3u_filter::Entity::delete_many()
        .filter(crate::entities::m3u_filter::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete m3u_filter: {}", e)})));
    }

    // vod_m3uepisoderelation
    if let Err(e) = crate::entities::vod_m3uepisoderelation::Entity::delete_many()
        .filter(crate::entities::vod_m3uepisoderelation::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete vod_m3uepisoderelation: {}", e)})));
    }

    // vod_m3umovierelation
    if let Err(e) = crate::entities::vod_m3umovierelation::Entity::delete_many()
        .filter(crate::entities::vod_m3umovierelation::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete vod_m3umovierelation: {}", e)})));
    }

    // vod_m3useriesrelation
    if let Err(e) = crate::entities::vod_m3useriesrelation::Entity::delete_many()
        .filter(crate::entities::vod_m3useriesrelation::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete vod_m3useriesrelation: {}", e)})));
    }

    // vod_m3uvodcategoryrelation
    if let Err(e) = crate::entities::vod_m3uvodcategoryrelation::Entity::delete_many()
        .filter(crate::entities::vod_m3uvodcategoryrelation::Column::M3uAccountId.eq(account_id))
        .exec(&state.db).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete vod_m3uvodcategoryrelation: {}", e)})));
    }

    match m3u_account::Entity::delete_by_id(account_id).exec(&state.db).await {
        Ok(_) => (StatusCode::NO_CONTENT, Json(json!({}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    }
}

pub async fn update_m3u_group_settings(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::channel_group_m3u_account;
    use crate::entities::vod_m3uvodcategoryrelation;

    if let Some(group_settings) = payload.get("group_settings").and_then(|v| v.as_array()) {
        for setting in group_settings {
            let cg_id = setting.get("channel_group").and_then(|v| {
                if let Some(obj) = v.as_object() {
                    obj.get("id").and_then(|i| i.as_i64())
                } else {
                    v.as_i64()
                }
            });

            if let Some(cg_id) = cg_id {
                if let Ok(Some(mapping)) = channel_group_m3u_account::Entity::find()
                    .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
                    .filter(channel_group_m3u_account::Column::ChannelGroupId.eq(cg_id))
                    .one(&state.db).await
                {
                    let mut active: channel_group_m3u_account::ActiveModel = mapping.into();
                    if let Some(enabled) = setting.get("enabled").and_then(|v| v.as_bool()) {
                        active.enabled = sea_orm::Set(enabled);
                    }
                    if let Some(auto_sync) = setting.get("auto_channel_sync").and_then(|v| v.as_bool()) {
                        active.auto_channel_sync = sea_orm::Set(auto_sync);
                    }
                    let _ = active.update(&state.db).await;
                }
            }
        }
    }

    if let Some(category_settings) = payload.get("category_settings").and_then(|v| v.as_array()) {
        for setting in category_settings {
            let cat_id = setting.get("id").and_then(|v| v.as_i64());

            if let Some(cat_id) = cat_id {
                if let Ok(Some(mapping)) = vod_m3uvodcategoryrelation::Entity::find()
                    .filter(vod_m3uvodcategoryrelation::Column::M3uAccountId.eq(account_id))
                    .filter(vod_m3uvodcategoryrelation::Column::CategoryId.eq(cat_id))
                    .one(&state.db).await
                {
                    let mut active: vod_m3uvodcategoryrelation::ActiveModel = mapping.into();
                    if let Some(enabled) = setting.get("enabled").and_then(|v| v.as_bool()) {
                        active.enabled = sea_orm::Set(enabled);
                    }
                    let _ = active.update(&state.db).await;
                }
            }
        }
    }

    (StatusCode::OK, Json(json!({"success": true})))
}

// --- Profiles ---
pub async fn get_m3u_profiles(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    use crate::entities::m3u_account_profile;
    let profiles = m3u_account_profile::Entity::find()
        .filter(m3u_account_profile::Column::M3uAccountId.eq(account_id))
        .all(&state.db)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(profiles))
}

pub async fn create_m3u_profile(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::m3u_account_profile;
    let active = m3u_account_profile::ActiveModel {
        m3u_account_id: sea_orm::Set(account_id),
        name: sea_orm::Set(payload.get("name").and_then(|v| v.as_str()).unwrap_or("New Profile").to_string()),
        is_default: sea_orm::Set(payload.get("is_default").and_then(|v| v.as_bool()).unwrap_or(false)),
        max_streams: sea_orm::Set(payload.get("max_streams").and_then(|v| v.as_i64()).unwrap_or(1) as i32),
        is_active: sea_orm::Set(payload.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true)),
        search_pattern: sea_orm::Set(payload.get("search_pattern").and_then(|v| v.as_str()).unwrap_or("^(.*)$").to_string()),
        replace_pattern: sea_orm::Set(payload.get("replace_pattern").and_then(|v| v.as_str()).unwrap_or("$1").to_string()),
        current_viewers: sea_orm::Set(0),
        ..Default::default()
    };
    if let Ok(inserted) = m3u_account_profile::Entity::insert(active).exec_with_returning(&state.db).await {
        (StatusCode::CREATED, Json(json!(inserted)))
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to create profile"})))
    }
}

pub async fn update_m3u_profile(
    State(state): State<Arc<AppState>>,
    Path((_account_id, profile_id)): Path<(i64, i64)>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::m3u_account_profile;
    if let Ok(Some(profile)) = m3u_account_profile::Entity::find_by_id(profile_id).one(&state.db).await {
        let mut active: m3u_account_profile::ActiveModel = profile.into();

        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            active.name = sea_orm::Set(name.to_string());
        }
        if let Some(is_default) = payload.get("is_default").and_then(|v| v.as_bool()) {
            active.is_default = sea_orm::Set(is_default);
        }
        if let Some(max_streams) = payload.get("max_streams").and_then(|v| v.as_i64()) {
            active.max_streams = sea_orm::Set(max_streams as i32);
        }
        if let Some(is_active) = payload.get("is_active").and_then(|v| v.as_bool()) {
            active.is_active = sea_orm::Set(is_active);
        }
        if let Some(search_pattern) = payload.get("search_pattern").and_then(|v| v.as_str()) {
            active.search_pattern = sea_orm::Set(search_pattern.to_string());
        }
        if let Some(replace_pattern) = payload.get("replace_pattern").and_then(|v| v.as_str()) {
            active.replace_pattern = sea_orm::Set(replace_pattern.to_string());
        }

        if let Ok(updated) = active.update(&state.db).await {
            return (StatusCode::OK, Json(json!(updated)));
        }
    }
    (StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to update profile"})))
}

pub async fn delete_m3u_profile(
    State(state): State<Arc<AppState>>,
    Path((_account_id, profile_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    use crate::entities::m3u_account_profile;
    let _ = m3u_account_profile::Entity::delete_by_id(profile_id).exec(&state.db).await;
    (StatusCode::NO_CONTENT, Json(json!({})))
}

// --- Filters ---
pub async fn get_m3u_filters(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    use crate::entities::m3u_filter;
    let filters = m3u_filter::Entity::find()
        .filter(m3u_filter::Column::M3uAccountId.eq(account_id))
        .all(&state.db)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(filters))
}

pub async fn create_m3u_filter(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<i64>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::m3u_filter;
    let active = m3u_filter::ActiveModel {
        m3u_account_id: sea_orm::Set(account_id),
        filter_type: sea_orm::Set(payload.get("filter_type").and_then(|v| v.as_str()).unwrap_or("regex").to_string()),
        regex_pattern: sea_orm::Set(payload.get("regex_pattern").and_then(|v| v.as_str()).unwrap_or("").to_string()),
        exclude: sea_orm::Set(payload.get("exclude").and_then(|v| v.as_bool()).unwrap_or(true)),
        order: sea_orm::Set(payload.get("order").and_then(|v| v.as_i64()).unwrap_or(0) as i32),
        ..Default::default()
    };
    if let Ok(inserted) = m3u_filter::Entity::insert(active).exec_with_returning(&state.db).await {
        (StatusCode::CREATED, Json(json!(inserted)))
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to create filter"})))
    }
}

pub async fn update_m3u_filter(
    State(state): State<Arc<AppState>>,
    Path((_account_id, filter_id)): Path<(i64, i64)>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::m3u_filter;
    if let Ok(Some(filter)) = m3u_filter::Entity::find_by_id(filter_id).one(&state.db).await {
        let mut active: m3u_filter::ActiveModel = filter.into();

        if let Some(ftype) = payload.get("filter_type").and_then(|v| v.as_str()) {
            active.filter_type = sea_orm::Set(ftype.to_string());
        }
        if let Some(pattern) = payload.get("regex_pattern").and_then(|v| v.as_str()) {
            active.regex_pattern = sea_orm::Set(pattern.to_string());
        }
        if let Some(exclude) = payload.get("exclude").and_then(|v| v.as_bool()) {
            active.exclude = sea_orm::Set(exclude);
        }
        if let Some(order) = payload.get("order").and_then(|v| v.as_i64()) {
            active.order = sea_orm::Set(order as i32);
        }

        if let Ok(updated) = active.update(&state.db).await {
            return (StatusCode::OK, Json(json!(updated)));
        }
    }
    (StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to update filter"})))
}

pub async fn delete_m3u_filter(
    State(state): State<Arc<AppState>>,
    Path((_account_id, filter_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    use crate::entities::m3u_filter;
    let _ = m3u_filter::Entity::delete_by_id(filter_id).exec(&state.db).await;
    (StatusCode::NO_CONTENT, Json(json!({})))
}

// --- Server Groups ---
pub async fn get_server_groups(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    use crate::entities::server_group;
    let groups = server_group::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(groups))
}

pub async fn create_server_group(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::server_group;
    let active = server_group::ActiveModel {
        name: sea_orm::Set(payload.get("name").and_then(|v| v.as_str()).unwrap_or("New Group").to_string()),
        ..Default::default()
    };
    if let Ok(inserted) = server_group::Entity::insert(active).exec_with_returning(&state.db).await {
        (StatusCode::CREATED, Json(json!(inserted)))
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to create server group"})))
    }
}

pub async fn update_server_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    use crate::entities::server_group;
    if let Ok(Some(group)) = server_group::Entity::find_by_id(group_id).one(&state.db).await {
        let mut active: server_group::ActiveModel = group.into();

        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
            active.name = sea_orm::Set(name.to_string());
        }

        if let Ok(updated) = active.update(&state.db).await {
            return (StatusCode::OK, Json(json!(updated)));
        }
    }
    (StatusCode::BAD_REQUEST, Json(json!({"error": "Failed to update server group"})))
}

pub async fn delete_server_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
) -> impl IntoResponse {
    use crate::entities::server_group;
    let _ = server_group::Entity::delete_by_id(group_id).exec(&state.db).await;
    (StatusCode::NO_CONTENT, Json(json!({})))
}

// --- Refresh Endpoints ---
pub async fn refresh_m3u_all(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
    let accounts = match m3u_account::Entity::find().filter(m3u_account::Column::IsActive.eq(true)).all(&state.db).await {
        Ok(a) => a,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    for account in accounts {
        let db_clone = state.db.clone();
        let account_id = account.id;
        let is_xc = account.account_type == "XC";
        let url = if is_xc {
            format!("{}/get.php?username={}&password={}&type=m3u_plus&output=ts",
                account.server_url.as_deref().unwrap_or_default().trim_end_matches('/'),
                account.username.as_deref().unwrap_or_default(),
                account.password.as_deref().unwrap_or_default()
            )
        } else {
            account.server_url.clone().unwrap_or_default()
        };

        if !url.is_empty() {
            let ws_clone_outer = state.ws_sender.clone();
            tokio::spawn(async move {
                let error_msg = if is_xc {
                    let ws_clone = ws_clone_outer.clone();
                    let xc_err = if let Err(e) = crate::m3u::fetch_and_parse_xc(&db_clone, account_id, Some(ws_clone)).await {
                        Some(format!("Failed to parse XC API: {}", e))
                    } else {
                        None
                    };
                    if xc_err.is_none() {
                        let _ = crate::m3u::fetch_and_parse_xc_vod(&db_clone, account_id).await;
                        let _ = crate::m3u::fetch_and_parse_xc_series(&db_clone, account_id).await;
                    }
                    xc_err
                } else {
                    let ws_clone = ws_clone_outer.clone();
                    match crate::m3u::fetch_and_parse_m3u(&db_clone, &url, account_id, false, Some(ws_clone)).await {
                        Err(e) => Some(format!("Failed to parse M3U: {}", e)),
                        Ok(_) => None,
                    }
                };

                if let Some(msg) = error_msg {
                    eprintln!("{}", msg);
                    if let Ok(Some(acc)) = crate::entities::m3u_account::Entity::find_by_id(account_id).one(&db_clone).await {
                        use sea_orm::ActiveModelTrait;
                        let mut active: crate::entities::m3u_account::ActiveModel = acc.into();
                        active.status = sea_orm::Set("failed".to_string());
                        active.last_message = sea_orm::Set(Some(msg.chars().take(255).collect()));
                        let _ = active.update(&db_clone).await;
                    }
                }
            });
        }
    }

    (StatusCode::ACCEPTED, Json(json!({"success": true, "message": "M3U refresh initiated."})))
}

pub async fn get_comskip_config(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser && !current_user.0.is_staff {
        return Err(StatusCode::FORBIDDEN);
    }

    let settings = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("dvr_settings"))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let path = if let Some(s) = settings {
        s.value.get("comskip_custom_path").and_then(|v| v.as_str()).unwrap_or("").to_string()
    } else {
        "".to_string()
    };

    let exists = !path.is_empty() && StdPath::new(&path).exists();

    Ok(Json(json!({
        "path": path,
        "exists": exists
    })))
}

pub async fn upload_comskip_ini(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser && !current_user.0.is_staff {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut file_saved = false;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "comskip_ini" {
            let file_name = field.file_name().unwrap_or("").to_lowercase();
            if !file_name.ends_with(".ini") {
                return Err(StatusCode::BAD_REQUEST);
            }

            let data = field.bytes().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let save_path = "comskip.ini";

            fs::write(save_path, &data).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let settings = core_settings::Entity::find()
                .filter(core_settings::Column::Key.eq("dvr_settings"))
                .one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut value = if let Some(ref s) = settings {
                s.value.clone()
            } else {
                json!({})
            };

            if let Some(obj) = value.as_object_mut() {
                obj.insert("comskip_custom_path".to_string(), json!(save_path));
            }

            if let Some(s) = settings {
                let mut active_s: core_settings::ActiveModel = s.into();
                active_s.value = Set(value);
                let _ = active_s.update(&state.db).await;
            } else {
                let new_setting = core_settings::ActiveModel {
                    key: Set("dvr_settings".to_string()),
                    name: Set("DVR Settings".to_string()),
                    value: Set(value),
                    ..Default::default()
                };
                let _ = new_setting.insert(&state.db).await;
            }

            file_saved = true;
            break;
        }
    }

    if file_saved {
        Ok(Json(json!({"message": "File uploaded successfully"})))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub async fn refresh_m3u_account_info(
    State(_state): State<Arc<AppState>>,
    Path(profile_id): Path<i64>,
) -> impl IntoResponse {
    // In a full implementation, spawn fetch for specific account metadata
    (StatusCode::ACCEPTED, Json(json!({"success": true, "message": format!("Account info refresh initiated for profile {}", profile_id)})))
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

    #[tokio::test]
    async fn test_get_core_settings() {
        let response = get_core_settings().await;
        let body = response.0;

        assert!(body.is_array());
        let settings = body.as_array().unwrap();

        assert_eq!(settings.len(), 6);

        let get_val = |key: &str| settings.iter().find(|s| s["key"] == key).unwrap()["value"].clone();

        assert_eq!(get_val("app_name"), "Dispatcharr");
        assert_eq!(get_val("proxy_enabled"), true);
        assert_eq!(get_val("registration_enabled"), false);
        assert_eq!(get_val("backend_url"), "");
        assert_eq!(get_val("version"), "0.22.1");
        assert_eq!(get_val("maintenance_mode"), false);
    }
}