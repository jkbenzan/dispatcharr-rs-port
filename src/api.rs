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

pub async fn get_current_user() -> Json<Value> {
    Json(json!({
        "id": 1,
        "username": "admin",
        "email": "admin@example.com",
        "is_superuser": true,
        "is_active": true,
        "is_staff": true,
        "permissions": ["*"], 
        "profile": {
            "theme": "dark",
            "language": "en",
            "navigation_order": [], 
            "hidden_nav_items": []
        }
    }))
}

// --------------------------------------------------------
// STANDARD AUTH & SYSTEM STUBS
// --------------------------------------------------------

pub async fn get_core_version() -> Json<Value> {
    Json(json!({ "version": "0.22.1", "name": "Dispatcharr" }))
}

pub async fn check_superuser() -> Json<Value> {
    Json(json!({ "initialized": true }))
}

pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6ImFkbWluIiwiaXNfc3VwZXJ1c2VyIjp0cnVlfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c", 
        "refresh": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6ImFkbWluIiwiaXNfc3VwZXJ1c2VyIjp0cnVlfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        "user": { "id": 1, "username": "admin", "is_superuser": true, "is_staff": true }
    }))
}

pub async fn refresh_token() -> Json<Value> {
    auth_placeholder().await
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

pub async fn get_channels() -> Json<Value> {
    get_paginated_object().await
}

// Routes called by fetchChannelGroups, fetchChannelProfiles, fetchPlaylists, fetchEPGs
pub async fn get_channel_groups() -> Json<Value> { get_flat_array().await }
pub async fn get_profiles() -> Json<Value> { get_flat_array().await }
pub async fn get_ids_stub() -> Json<Value> { get_flat_array().await }
pub async fn get_m3u_accounts() -> Json<Value> { get_flat_array().await }
pub async fn get_epg_sources() -> Json<Value> { get_flat_array().await }

// Route called by getNotifications
pub async fn get_notifications() -> Json<Value> {
    get_paginated_object().await
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