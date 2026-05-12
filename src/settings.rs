use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::CurrentUser;
use crate::entities::core_settings;
use crate::AppState;
use sea_orm::{ColumnTrait, DatabaseConnection, QueryFilter};

pub const DEFAULT_PROVIDER_REFRESH_CONCURRENCY: usize = 1;
pub const MAX_PROVIDER_REFRESH_CONCURRENCY: usize = 8;
pub const PROVIDER_REFRESH_CONCURRENCY_ENV: &str = "DISPATCHARR_PROVIDER_REFRESH_CONCURRENCY";

pub async fn get_maintenance_settings(db: &DatabaseConnection) -> crate::background::MaintenanceSettings {
    let s = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("maintenance_settings"))
        .one(db)
        .await;

    if let Ok(Some(s)) = s {
        if let Ok(settings) = serde_json::from_value(s.value) {
            return settings;
        }
    }

    crate::background::MaintenanceSettings::default()
}


#[derive(Deserialize)]
pub struct SettingReq {
    pub key: String,
    pub name: String,
    pub value: Value,
}

pub async fn list_settings(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    let is_admin = current_user.is_admin();

    let settings = core_settings::Entity::find()
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for s in settings {
        // Hide sensitive settings from non-admin users
        if !is_admin && (s.key == "network_access" || s.key == "proxy_settings" || s.key == "user_limit_settings") {
            continue;
        }

        result.push(json!({
            "id": s.id,
            "key": s.key,
            "name": s.name,
            "value": s.value,
        }));
    }

    Ok(Json(json!(result)))
}

pub async fn create_setting(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(payload): Json<SettingReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    let new_setting = core_settings::ActiveModel {
        key: Set(payload.key),
        name: Set(payload.name),
        value: Set(payload.value),
        ..Default::default()
    };

    let inserted = new_setting
        .insert(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log setting creation for the Activity page
    let _ = crate::events::record_event(
        &state.db,
        "settings_created",
        None,
        serde_json::json!({
            "status": "info",
            "event": "Setting Created",
            "setting_key": inserted.key,
            "setting_name": inserted.name
        }),
    ).await;

    Ok(Json(json!({
        "id": inserted.id,
        "key": inserted.key,
        "name": inserted.name,
        "value": inserted.value,
    })))
}

pub async fn get_setting(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    let s = core_settings::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(s) = s {
        Ok(Json(json!({
            "id": s.id,
            "key": s.key,
            "name": s.name,
            "value": s.value,
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn get_setting_by_key(db: &sea_orm::DatabaseConnection, key: &str) -> Option<serde_json::Value> {
    use crate::entities::core_settings;
    use sea_orm::EntityTrait;
    use sea_orm::QueryFilter;
    use sea_orm::ColumnTrait;

    core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq(key))
        .one(db)
        .await
        .unwrap_or_default()
        .map(|s| s.value)
}

pub async fn update_setting(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    current_user: CurrentUser,
    Json(payload): Json<SettingReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut s: core_settings::ActiveModel = core_settings::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?
        .into();

    s.key = Set(payload.key.clone());
    s.name = Set(payload.name.clone());
    s.value = Set(payload.value.clone());

    tracing::info!("💾 DB SAVE: Setting '{}' (id: {})", payload.key, id);

    let updated = s
        .update(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("❌ DB SAVE FAILED: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log settings update for the Activity page (values omitted for security)
    let _ = crate::events::record_event(
        &state.db,
        "settings_updated",
        None,
        serde_json::json!({
            "status": "info",
            "event": "Settings Updated",
            "setting_key": updated.key,
            "setting_name": updated.name
        }),
    ).await;

    Ok(Json(json!({
        "id": updated.id,
        "key": updated.key,
        "name": updated.name,
        "value": updated.value,
    })))
}




pub async fn initialize_core_settings(db: &sea_orm::DatabaseConnection) {
    let defaults = vec![
        (
            "ui_settings",
            "UI Settings",
            serde_json::json!({
                "time_format": "12h",
                "date_format": "mdy",
                "table_size": "default",
                "time_zone": "UTC"
            }),
        ),
        (
            "dvr_settings",
            "DVR Settings",
            serde_json::json!({
                "comskip_enabled": false,
                "comskip_custom_path": "",
                "pre_offset_minutes": 0,
                "post_offset_minutes": 0,
                "tv_template": "TV_Shows/{show}/S{season:02d}E{episode:02d}.mkv",
                "tv_fallback_template": "TV_Shows/{show}/{start}.mkv",
                "movie_template": "Movies/{title} ({year}).mkv",
                "movie_fallback_template": "Movies/{start}.mkv"
            }),
        ),
        (
            "stream_settings",
            "Stream Settings",
            serde_json::json!({
                "buffer_size": 1024,
                "retry_count": 3,
                "stream_checker_parallel_providers": 1,
                "provider_refresh_concurrency": DEFAULT_PROVIDER_REFRESH_CONCURRENCY,
                "default_user_agent": "TiviMate/5.0.4 (Linux;Android 11) iPTV-Client"
            }),
        ),
        (
            "maintenance_settings",
            "Maintenance Settings",
            serde_json::json!({
                "stream_check_frequency_days": 7,
                "off_hours_start": 2,
                "off_hours_end": 6,
                "idle_threshold_minutes": 30,
                "batch_size": 50,
                "extended_test_enabled": false,
                "extended_test_duration_seconds": 60,
                "auto_prune_failed_count": 3
            }),
        ),
        (
            "system_settings",
            "System Settings",
            serde_json::json!({
                "time_zone": "UTC",
                "max_system_events": 100
            }),
        ),
        (
            "network_access",
            "Network Access",
            serde_json::json!({
                "M3U_EPG": "127.0.0.0/8,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,::1/128,fc00::/7,fe80::/10",
                "STREAMS": "0.0.0.0/0,::/0",
                "XC_API": "0.0.0.0/0,::/0",
                "UI": "0.0.0.0/0,::/0"
            }),
        ),
        (
            "proxy_settings",
            "Proxy Settings",
            serde_json::json!({
                "buffering_timeout": 15,
                "buffering_speed": 1.0,
                "redis_chunk_ttl": 60,
                "channel_shutdown_delay": 0,
                "channel_init_grace_period": 5,
                "new_client_behind_seconds": 5,
                "http_proxy_enabled": false,
                "http_proxy_url": ""
            }),
        ),
        (
            "user_limit_settings",
            "User Limits",
            serde_json::json!({
                "terminate_on_limit_exceeded": true,
                "prioritize_single_client_channels": true,
                "ignore_same_channel_connections": false,
                "terminate_oldest": true,
                "max_streams": 1
            }),
        ),
        (
            "channel_db_settings",
            "Channel DB Settings",
            serde_json::json!({
                "download_url": "https://github.com/djpongh/channelidentifiarr/releases/latest/download/channelidentifiarr.db",
                "auto_check": false
            }),
        ),
    ];

    tracing::info!("🔍 Checking core settings defaults...");

    for (key, name, value) in defaults {
        let existing = crate::entities::core_settings::Entity::find()
            .filter(crate::entities::core_settings::Column::Key.eq(key))
            .one(db)
            .await
            .unwrap_or_default();

        if let Some(setting) = existing {
            if key == "stream_settings" && setting.value.get("provider_refresh_concurrency").is_none() {
                let mut value = setting.value.clone();
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "provider_refresh_concurrency".to_string(),
                        serde_json::json!(DEFAULT_PROVIDER_REFRESH_CONCURRENCY),
                    );
                    let mut active: crate::entities::core_settings::ActiveModel = setting.into();
                    active.value = sea_orm::Set(value);
                    if active.update(db).await.is_ok() {
                        tracing::info!("âœ¨ Added default stream setting: provider_refresh_concurrency");
                    }
                }
            } else if key == "maintenance_settings" {
                let mut value = setting.value.clone();
                let mut changed = false;
                if let Some(obj) = value.as_object_mut() {
                    for (field, default_value) in [
                        ("stream_check_frequency_days", serde_json::json!(7)),
                        ("off_hours_start", serde_json::json!(2)),
                        ("off_hours_end", serde_json::json!(6)),
                        ("idle_threshold_minutes", serde_json::json!(30)),
                        ("batch_size", serde_json::json!(50)),
                        ("extended_test_enabled", serde_json::json!(false)),
                        ("extended_test_duration_seconds", serde_json::json!(60)),
                        ("auto_prune_failed_count", serde_json::json!(3)),
                    ] {
                        if obj.get(field).is_none() {
                            obj.insert(field.to_string(), default_value);
                            changed = true;
                        }
                    }
                }
                if changed {
                    let mut active: crate::entities::core_settings::ActiveModel = setting.into();
                    active.value = sea_orm::Set(value);
                    if active.update(db).await.is_ok() {
                        tracing::info!("Added missing maintenance settings defaults");
                    }
                }
            }
        } else {
            let _ = crate::entities::core_settings::ActiveModel {
                key: sea_orm::Set(key.to_string()),
                name: sea_orm::Set(name.to_string()),
                value: sea_orm::Set(value),
                ..Default::default()
            }
            .insert(db)
            .await;
            tracing::info!("✨ Created default setting: {}", key);
        }
    }
    tracing::info!("✅ Core settings check complete.");

    // Initialize default stream profiles
    initialize_stream_profiles(db).await;
}

fn parse_provider_refresh_concurrency_value(value: &serde_json::Value) -> Option<usize> {
    match value {
        serde_json::Value::Number(n) => n.as_u64().map(|v| v as usize),
        serde_json::Value::String(s) => s.trim().parse::<usize>().ok(),
        _ => None,
    }
    .filter(|value| *value > 0)
    .map(|value| value.min(MAX_PROVIDER_REFRESH_CONCURRENCY))
}

pub fn parse_provider_refresh_concurrency_env(value: Option<&str>) -> usize {
    value
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
        .map(|v| v.min(MAX_PROVIDER_REFRESH_CONCURRENCY))
        .unwrap_or(DEFAULT_PROVIDER_REFRESH_CONCURRENCY)
}

pub async fn get_provider_refresh_concurrency(db: &sea_orm::DatabaseConnection) -> usize {
    if let Ok(value) = std::env::var(PROVIDER_REFRESH_CONCURRENCY_ENV) {
        return parse_provider_refresh_concurrency_env(Some(&value));
    }

    let setting = crate::entities::core_settings::Entity::find()
        .filter(crate::entities::core_settings::Column::Key.eq("stream_settings"))
        .one(db)
        .await
        .unwrap_or_default();

    setting
        .and_then(|s| s.value.get("provider_refresh_concurrency").cloned())
        .as_ref()
        .and_then(parse_provider_refresh_concurrency_value)
        .unwrap_or(DEFAULT_PROVIDER_REFRESH_CONCURRENCY)
}

pub async fn initialize_stream_profiles(db: &sea_orm::DatabaseConnection) {
    use crate::entities::core_streamprofile;

    let defaults = vec![
        (
            "Direct (Proxy)",
            "",
            "",
            true,
        ),
        (
            "FFmpeg (Remux)",
            "ffmpeg",
            "-re -i {input} -vcodec copy -acodec copy -f mpegts pipe:1",
            true,
        ),
    ];

    tracing::info!("🔍 Checking default stream profiles...");

    for (name, command, parameters, locked) in defaults {
        let existing = core_streamprofile::Entity::find()
            .filter(core_streamprofile::Column::Name.eq(name))
            .one(db)
            .await
            .unwrap_or_default();

        if existing.is_none() {
            let _ = core_streamprofile::ActiveModel {
                name: sea_orm::Set(name.to_string()),
                command: sea_orm::Set(command.to_string()),
                parameters: sea_orm::Set(parameters.to_string()),
                is_active: sea_orm::Set(true),
                locked: sea_orm::Set(locked),
                ..Default::default()
            }
                .insert(db)
                .await;
            tracing::info!("✨ Created default stream profile: {}", name);
        }
    }
    tracing::info!("✅ Stream profiles check complete.");
}

pub async fn get_http_client(db: &sea_orm::DatabaseConnection) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(10)); // connection timeout is fine, but not body timeout

    let setting = crate::entities::core_settings::Entity::find()
        .filter(crate::entities::core_settings::Column::Key.eq("proxy_settings"))
        .one(db)
        .await
        .unwrap_or_default();

    if let Some(s) = setting {
        if let Some(enabled) = s.value.get("http_proxy_enabled").and_then(|v| v.as_bool()) {
            if enabled {
                if let Some(url) = s.value.get("http_proxy_url").and_then(|v| v.as_str()) {
                    if !url.trim().is_empty() {
                        if let Ok(proxy) = reqwest::Proxy::all(url) {
                            builder = builder.proxy(proxy);
                            tracing::info!("HTTP Client configured with proxy: {}", url);
                        }
                    }
                }
            }
        }
    }

    builder.build().expect("Failed to build reqwest client")
}

#[derive(Debug, Clone)]
pub struct ProxySettings {
    pub buffering_timeout: u64,
    pub buffering_speed: f64,
    pub channel_shutdown_delay: u64,
    pub max_retries: u32,
    pub chunk_size: usize,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            buffering_timeout: 10,
            buffering_speed: 1.0,
            channel_shutdown_delay: 60,
            max_retries: 3,
            chunk_size: 1024 * 1024, // 1MB
        }
    }
}

pub async fn get_proxy_settings(db: &sea_orm::DatabaseConnection) -> ProxySettings {
    let setting = crate::entities::core_settings::Entity::find()
        .filter(crate::entities::core_settings::Column::Key.eq("proxy_settings"))
        .one(db)
        .await
        .unwrap_or_default();

    let mut proxy_settings = ProxySettings::default();

    if let Some(s) = setting {
        if let Some(timeout) = s.value.get("buffering_timeout").and_then(|v| v.as_u64()) {
            proxy_settings.buffering_timeout = timeout;
        }
        if let Some(speed) = s.value.get("buffering_speed").and_then(|v| v.as_f64()) {
            proxy_settings.buffering_speed = speed;
        }
        if let Some(delay) = s.value.get("channel_shutdown_delay").and_then(|v| v.as_u64()) {
            proxy_settings.channel_shutdown_delay = delay;
        }
        if let Some(retries) = s.value.get("max_retries").and_then(|v| v.as_u64()) {
            proxy_settings.max_retries = retries as u32;
        }
        if let Some(chunk) = s.value.get("chunk_size").and_then(|v| v.as_u64()) {
            proxy_settings.chunk_size = chunk as usize;
        }
    }

    proxy_settings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_provider_refresh_concurrency_env_defaults_on_invalid_values() {
        assert_eq!(parse_provider_refresh_concurrency_env(None), 1);
        assert_eq!(parse_provider_refresh_concurrency_env(Some("")), 1);
        assert_eq!(parse_provider_refresh_concurrency_env(Some("abc")), 1);
        assert_eq!(parse_provider_refresh_concurrency_env(Some("0")), 1);
    }

    #[test]
    fn test_provider_refresh_concurrency_env_clamps_high_values() {
        assert_eq!(parse_provider_refresh_concurrency_env(Some("2")), 2);
        assert_eq!(
            parse_provider_refresh_concurrency_env(Some("99")),
            MAX_PROVIDER_REFRESH_CONCURRENCY
        );
    }

    #[test]
    fn test_provider_refresh_concurrency_setting_accepts_number_or_string() {
        assert_eq!(parse_provider_refresh_concurrency_value(&json!(3)), Some(3));
        assert_eq!(parse_provider_refresh_concurrency_value(&json!("4")), Some(4));
        assert_eq!(parse_provider_refresh_concurrency_value(&json!(0)), None);
        assert_eq!(parse_provider_refresh_concurrency_value(&json!("nope")), None);
        assert_eq!(
            parse_provider_refresh_concurrency_value(&json!(99)),
            Some(MAX_PROVIDER_REFRESH_CONCURRENCY)
        );
    }
}
