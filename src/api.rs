use axum::Json;
use serde_json::{json, Value};

/// Core version info used by the UI's "About" or "System" sections.
pub async fn get_core_version() -> Json<Value> {
    Json(json!({ "version": "0.22.1", "name": "Dispatcharr" }))
}

/// Checks if the initial setup has been completed.
pub async fn check_superuser() -> Json<Value> {
    Json(json!({ "initialized": true }))
}

/// Provides the current user context. 
/// Critical: The 'profile' fields stop crashes in the sidebar and navigation.
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
            "hidden_nav_items": [],
            "notifications_enabled": true
        }
    }))
}

/// Mock JWT tokens for the frontend.
pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6ImFkbWluIiwiaXNfc3VwZXJ1c2VyIjp0cnVlfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c", 
        "refresh": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6ImFkbWluIiwiaXNfc3VwZXJ1c2VyIjp0cnVlfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        "user": {
            "id": 1,
            "username": "admin",
            "is_superuser": true,
            "is_staff": true
        }
    }))
}

/// Global core settings.
pub async fn get_core_settings() -> Json<Value> {
    Json(json!({
        "app_name": "Dispatcharr",
        "proxy_enabled": true,
        "registration_enabled": false,
        "channel_profiles": [],
        "stream_profiles": [],
        "user_agents": [],
        "notification_types": [],
        "version": "0.22.1",
        "maintenance_mode": false
    }))
}

/// Environmental configuration used by some UI toggles.
pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

/// The DRF Envelope: This is the specific "shape" the frontend expects for lists.
/// This prevents 'Cannot read properties of undefined (reading length/filter)' errors.
pub async fn get_drf_list() -> Json<Value> {
    Json(json!({
        "count": 0,
        "next": null,
        "previous": null,
        "results": []
    }))
}

/// Global UI/Theme configuration.
pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}

/// Standard success response for logout.
pub async fn logout_stub() -> Json<Value> {
    Json(json!({ "detail": "Successfully logged out." }))
}