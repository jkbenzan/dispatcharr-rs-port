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

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}

pub async fn logout_stub() -> Json<Value> {
    Json(json!({ "detail": "Successfully logged out." }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_flat_array() {
        let Json(val) = get_flat_array().await;
        assert_eq!(val, json!([]));
    }

    #[tokio::test]
    async fn test_get_paginated_object() {
        let Json(val) = get_paginated_object().await;
        assert_eq!(val["count"], 0);
        assert_eq!(val["results"], json!([]));
        assert_eq!(val["next"], serde_json::Value::Null);
        assert_eq!(val["previous"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_get_core_settings() {
        let Json(val) = get_core_settings().await;
        assert_eq!(val["app_name"], "Dispatcharr");
        assert_eq!(val["proxy_enabled"], true);
        assert_eq!(val["version"], "0.22.1");
        assert_eq!(val["channel_profiles"], json!([]));
    }

    #[tokio::test]
    async fn test_get_current_user() {
        let Json(val) = get_current_user().await;
        assert_eq!(val["username"], "admin");
        assert_eq!(val["is_superuser"], true);
        assert_eq!(val["email"], "admin@example.com");
    }

    #[tokio::test]
    async fn test_get_core_version() {
        let Json(val) = get_core_version().await;
        assert_eq!(val["version"], "0.22.1");
        assert_eq!(val["name"], "Dispatcharr");
    }

    #[tokio::test]
    async fn test_check_superuser() {
        let Json(val) = check_superuser().await;
        assert_eq!(val["initialized"], true);
    }

    #[tokio::test]
    async fn test_auth_placeholder() {
        let Json(val) = auth_placeholder().await;
        assert!(val.get("access").is_some());
        assert!(val.get("refresh").is_some());
        assert_eq!(val["user"]["username"], "admin");
        assert_eq!(val["user"]["id"], 1);
    }

    #[tokio::test]
    async fn test_get_env_settings() {
        let Json(val) = get_env_settings().await;
        assert_eq!(val["ENV"], "production");
        assert_eq!(val["DEBUG"], "false");
    }

    #[tokio::test]
    async fn test_get_config() {
        let Json(val) = get_config().await;
        assert_eq!(val["theme"], "dark");
        assert_eq!(val["auth_enabled"], false);
        assert_eq!(val["base_url"], "/");
    }

    #[tokio::test]
    async fn test_logout_stub() {
        let Json(val) = logout_stub().await;
        assert_eq!(val["detail"], "Successfully logged out.");
    }
}