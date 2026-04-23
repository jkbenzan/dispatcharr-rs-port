use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, Set, ModelTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::CurrentUser;
use crate::entities::core_settings;
use crate::AppState;

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
    if !current_user.0.is_superuser && !current_user.0.is_staff {
        return Err(StatusCode::FORBIDDEN);
    }
    let settings = core_settings::Entity::find()
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for s in settings {
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
    if !current_user.0.is_superuser && !current_user.0.is_staff {
        return Err(StatusCode::FORBIDDEN);
    }

    let new_setting = core_settings::ActiveModel {
        key: Set(payload.key),
        name: Set(payload.name),
        value: Set(payload.value),
        ..Default::default()
    };

    let inserted = new_setting.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    if !current_user.0.is_superuser && !current_user.0.is_staff {
        return Err(StatusCode::FORBIDDEN);
    }

    let s = core_settings::Entity::find_by_id(id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

pub async fn update_setting(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    current_user: CurrentUser,
    Json(payload): Json<SettingReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser && !current_user.0.is_staff {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut s: core_settings::ActiveModel = core_settings::Entity::find_by_id(id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?.into();

    s.key = Set(payload.key);
    s.name = Set(payload.name);
    s.value = Set(payload.value);

    let updated = s.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "id": updated.id,
        "key": updated.key,
        "name": updated.name,
        "value": updated.value,
    })))
}
