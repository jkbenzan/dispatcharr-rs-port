use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, ModelTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use crate::auth::{hash_password, CurrentUser};
use crate::entities::{accounts_user_groups, auth_group, auth_group_permissions, auth_permission, user};
use crate::AppState;

// --- User CRUD ---

pub fn serialize_user(u: &user::Model, groups: Vec<i32>) -> Value {
    json!({
        "id": u.id,
        "username": u.username,
        "email": u.email,
        "first_name": u.first_name,
        "last_name": u.last_name,
        "is_superuser": u.is_superuser,
        "is_staff": u.is_staff,
        "is_active": u.is_active,
        "user_level": u.user_level,
        "api_key": u.api_key,
        "groups": groups,
        "channel_profiles": [],
        "custom_properties": u.custom_properties,
        "date_joined": u.date_joined,
        "last_login": u.last_login,
    })
}

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub username: String,
    pub password: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub user_level: Option<i32>,
    pub is_superuser: Option<bool>,
    pub is_staff: Option<bool>,
    pub groups: Option<Vec<i32>>,
}

#[derive(Deserialize)]
pub struct UpdateUserReq {
    pub username: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub user_level: Option<i32>,
    pub is_superuser: Option<bool>,
    pub is_staff: Option<bool>,
    pub groups: Option<Vec<i32>>,
    pub custom_properties: Option<Value>,
}

#[derive(Deserialize)]
pub struct UpdateMeReq {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub custom_properties: Option<Value>,
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    let users = user::Entity::find()
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for u in users {
        let groups: Vec<i32> = accounts_user_groups::Entity::find()
            .filter(accounts_user_groups::Column::UserId.eq(u.id))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.group_id)
            .collect();
            
        result.push(serialize_user(&u, groups));
    }
    
    Ok(Json(json!(result)))
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(payload): Json<CreateUserReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    let hashed = if let Some(p) = payload.password {
        hash_password(&p)
    } else {
        hash_password("default123")
    };
    
    let now = Utc::now().into();
    let new_user = user::ActiveModel {
        username: Set(payload.username),
        password: Set(hashed),
        email: Set(payload.email.unwrap_or_default()),
        first_name: Set(payload.first_name.unwrap_or_default()),
        last_name: Set(payload.last_name.unwrap_or_default()),
        is_superuser: Set(payload.is_superuser.unwrap_or(false)),
        is_staff: Set(payload.is_staff.unwrap_or(false)),
        is_active: Set(true),
        user_level: Set(payload.user_level.unwrap_or(1)),
        date_joined: Set(now),
        stream_limit: Set(0),
        ..Default::default()
    };
    
    let inserted = new_user.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut final_groups = Vec::new();
    if let Some(groups) = payload.groups {
        for gid in &groups {
            let user_group = accounts_user_groups::ActiveModel {
                user_id: Set(inserted.id),
                group_id: Set(*gid),
                ..Default::default()
            };
            let _ = user_group.insert(&state.db).await;
        }
        final_groups = groups;
    }
    
    Ok(Json(serialize_user(&inserted, final_groups)))
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    let u = user::Entity::find_by_id(id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(u) = u {
        let groups: Vec<i32> = accounts_user_groups::Entity::find()
            .filter(accounts_user_groups::Column::UserId.eq(u.id))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.group_id)
            .collect();
            
        Ok(Json(serialize_user(&u, groups)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    current_user: CurrentUser,
    Json(payload): Json<UpdateUserReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut u: user::ActiveModel = user::Entity::find_by_id(id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?.into();
    
    if let Some(v) = payload.username { u.username = Set(v); }
    if let Some(v) = payload.email { u.email = Set(v); }
    if let Some(v) = payload.first_name { u.first_name = Set(v); }
    if let Some(v) = payload.last_name { u.last_name = Set(v); }
    if let Some(v) = payload.is_superuser { u.is_superuser = Set(v); }
    if let Some(v) = payload.is_staff { u.is_staff = Set(v); }
    if let Some(v) = payload.user_level { u.user_level = Set(v); }
    if let Some(v) = payload.custom_properties { u.custom_properties = Set(Some(v)); }
    if let Some(p) = payload.password {
        if !p.is_empty() {
            u.password = Set(hash_password(&p));
        }
    }
    
    let updated = u.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(groups) = payload.groups {
        let _ = accounts_user_groups::Entity::delete_many()
            .filter(accounts_user_groups::Column::UserId.eq(id))
            .exec(&state.db).await;
            
        for gid in groups {
            let user_group = accounts_user_groups::ActiveModel {
                user_id: Set(updated.id),
                group_id: Set(gid),
                ..Default::default()
            };
            let _ = user_group.insert(&state.db).await;
        }
    }
    
    let groups: Vec<i32> = accounts_user_groups::Entity::find()
        .filter(accounts_user_groups::Column::UserId.eq(updated.id))
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|g| g.group_id)
        .collect();
    
    Ok(Json(serialize_user(&updated, groups)))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    current_user: CurrentUser,
) -> Result<StatusCode, StatusCode> {
    if !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    user::Entity::delete_by_id(id).exec(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_me(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    let u = current_user.0;
    let groups: Vec<i32> = accounts_user_groups::Entity::find()
        .filter(accounts_user_groups::Column::UserId.eq(u.id))
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|g| g.group_id)
        .collect();
        
    Ok(Json(serialize_user(&u, groups)))
}

pub async fn update_me(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(payload): Json<UpdateMeReq>,
) -> Result<Json<Value>, StatusCode> {
    let mut u: user::ActiveModel = current_user.0.into();
    
    if let Some(v) = payload.first_name { u.first_name = Set(v); }
    if let Some(v) = payload.last_name { u.last_name = Set(v); }
    if let Some(v) = payload.email { u.email = Set(v); }
    if let Some(v) = payload.custom_properties {
        // Handle custom properties merging/filtering here if needed
        u.custom_properties = Set(Some(v));
    }
    if let Some(p) = payload.password {
        if !p.is_empty() {
            u.password = Set(hash_password(&p));
        }
    }
    
    let updated = u.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let groups: Vec<i32> = accounts_user_groups::Entity::find()
        .filter(accounts_user_groups::Column::UserId.eq(updated.id))
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|g| g.group_id)
        .collect();
        
    Ok(Json(serialize_user(&updated, groups)))
}

// --- Groups CRUD ---

#[derive(Deserialize)]
pub struct GroupReq {
    pub name: String,
    pub permissions: Option<Vec<i32>>,
}

pub async fn list_groups(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser { return Err(StatusCode::FORBIDDEN); }
    
    let groups = auth_group::Entity::find().all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut result = Vec::new();
    for g in groups {
        let perms: Vec<i32> = auth_group_permissions::Entity::find()
            .filter(auth_group_permissions::Column::GroupId.eq(g.id))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.permission_id)
            .collect();
            
        result.push(json!({
            "id": g.id,
            "name": g.name,
            "permissions": perms
        }));
    }
    
    Ok(Json(json!(result)))
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(payload): Json<GroupReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser { return Err(StatusCode::FORBIDDEN); }
    
    let new_group = auth_group::ActiveModel {
        name: Set(payload.name),
        ..Default::default()
    };
    
    let inserted = new_group.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(perms) = payload.permissions {
        for pid in perms {
            let gp = auth_group_permissions::ActiveModel {
                group_id: Set(inserted.id),
                permission_id: Set(pid),
                ..Default::default()
            };
            let _ = gp.insert(&state.db).await;
        }
    }
    
    Ok(Json(json!({"id": inserted.id, "name": inserted.name})))
}

pub async fn get_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser { return Err(StatusCode::FORBIDDEN); }
    
    let g = auth_group::Entity::find_by_id(id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(g) = g {
        let perms: Vec<i32> = auth_group_permissions::Entity::find()
            .filter(auth_group_permissions::Column::GroupId.eq(g.id))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.permission_id)
            .collect();
            
        Ok(Json(json!({
            "id": g.id,
            "name": g.name,
            "permissions": perms
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn update_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    current_user: CurrentUser,
    Json(payload): Json<GroupReq>,
) -> Result<Json<Value>, StatusCode> {
    if !current_user.0.is_superuser { return Err(StatusCode::FORBIDDEN); }
    
    let mut g: auth_group::ActiveModel = auth_group::Entity::find_by_id(id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?.into();
    
    g.name = Set(payload.name);
    let updated = g.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Some(perms) = payload.permissions {
        let _ = auth_group_permissions::Entity::delete_many()
            .filter(auth_group_permissions::Column::GroupId.eq(id))
            .exec(&state.db).await;
            
        for pid in perms {
            let gp = auth_group_permissions::ActiveModel {
                group_id: Set(updated.id),
                permission_id: Set(pid),
                ..Default::default()
            };
            let _ = gp.insert(&state.db).await;
        }
    }
    
    Ok(Json(json!({"id": updated.id, "name": updated.name})))
}

pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    current_user: CurrentUser,
) -> Result<StatusCode, StatusCode> {
    if !current_user.0.is_superuser { return Err(StatusCode::FORBIDDEN); }
    auth_group::Entity::delete_by_id(id).exec(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Permissions List ---

pub async fn list_permissions(
    State(state): State<Arc<AppState>>,
    _current_user: CurrentUser,
) -> Result<Json<Value>, StatusCode> {
    let perms = auth_permission::Entity::find().all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result: Vec<Value> = perms.into_iter().map(|p| json!({
        "id": p.id,
        "name": p.name,
        "codename": p.codename,
        "content_type": p.content_type_id
    })).collect();
    
    Ok(Json(Value::Array(result)))
}

// --- API Keys ---

pub async fn get_api_key(current_user: CurrentUser) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({"key": current_user.0.api_key})))
}

#[derive(Deserialize)]
pub struct ApiKeyReq {
    pub user_id: Option<i64>,
}

pub async fn generate_api_key(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(payload): Json<ApiKeyReq>,
) -> Result<Json<Value>, StatusCode> {
    let target_user_id = payload.user_id.unwrap_or(current_user.0.id);
    if target_user_id != current_user.0.id && !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let mut u: user::ActiveModel = user::Entity::find_by_id(target_user_id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?.into();
    
    use uuid::Uuid;
    let key = Uuid::new_v4().simple().to_string();
        
    u.api_key = Set(Some(key.clone()));
    u.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({"key": key})))
}

pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(payload): Json<ApiKeyReq>,
) -> Result<Json<Value>, StatusCode> {
    let target_user_id = payload.user_id.unwrap_or(current_user.0.id);
    if target_user_id != current_user.0.id && !current_user.0.is_superuser {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let mut u: user::ActiveModel = user::Entity::find_by_id(target_user_id).one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?.into();
    
    u.api_key = Set(None);
    u.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({"success": true})))
}

// --- Initialize Superuser ---

pub async fn check_superuser(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    // Check if any admin/superuser exists
    let has_superuser = user::Entity::find()
        .filter(user::Column::UserLevel.gte(10))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    Ok(Json(json!({ "superuser_exists": has_superuser })))
}

#[derive(Deserialize)]
pub struct InitSuperuserReq {
    pub username: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
}

pub async fn init_superuser(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InitSuperuserReq>,
) -> Result<Json<Value>, StatusCode> {
    let has_superuser = user::Entity::find()
        .filter(user::Column::UserLevel.gte(10))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if has_superuser {
        return Ok(Json(json!({"superuser_exists": true})));
    }

    let username = match payload.username {
        Some(u) if !u.is_empty() => u,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let password = match payload.password {
        Some(p) if !p.is_empty() => p,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let email = payload.email.unwrap_or_default();

    let hashed = hash_password(&password);
    let now = Utc::now().into();
    
    let new_user = user::ActiveModel {
        username: Set(username),
        password: Set(hashed),
        email: Set(email),
        first_name: Set(String::new()),
        last_name: Set(String::new()),
        is_superuser: Set(true),
        is_staff: Set(true),
        is_active: Set(true),
        user_level: Set(10),
        date_joined: Set(now),
        stream_limit: Set(0),
        ..Default::default()
    };

    new_user.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"superuser_exists": true})))
}
