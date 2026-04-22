use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QuerySelect,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use crate::entities::{vod_category, vod_movie, vod_series};

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub search: Option<String>,
}

pub async fn get_vod_all(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Pagination>,
) -> Result<Json<Value>, StatusCode> {
    let search = params.search.unwrap_or_default().to_lowercase();
    let limit = params.page_size.unwrap_or(24);
    
    let mut movies_q = vod_movie::Entity::find();
    let mut series_q = vod_series::Entity::find();
    
    if !search.is_empty() {
        movies_q = movies_q.filter(
            sea_orm::Condition::any()
                .add(sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(sea_orm::sea_query::Expr::col(vod_movie::Column::Name))).like(format!("%{}%", search)))
        );
        series_q = series_q.filter(
            sea_orm::Condition::any()
                .add(sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(sea_orm::sea_query::Expr::col(vod_series::Column::Name))).like(format!("%{}%", search)))
        );
    }
    
    // For a unified view, fetch up to limit from both, combine, sort, and slice
    let movies = movies_q.limit(limit).all(&state.db).await.unwrap_or_default();
    let series = series_q.limit(limit).all(&state.db).await.unwrap_or_default();
    
    let mut results = Vec::new();
    
    for m in movies {
        results.push(json!({
            "id": m.id,
            "name": m.name,
            "type": "movie",
            "year": m.year,
            "rating": m.rating,
            "description": m.description,
            "created_at": m.created_at,
            "logo_id": m.logo_id,
        }));
    }
    
    for s in series {
        results.push(json!({
            "id": s.id,
            "name": s.name,
            "type": "series",
            "year": s.year,
            "rating": s.rating,
            "description": s.description,
            "created_at": s.created_at,
            "logo_id": s.logo_id,
        }));
    }
    
    // Sort by name
    results.sort_by(|a, b| {
        let name_a = a["name"].as_str().unwrap_or("");
        let name_b = b["name"].as_str().unwrap_or("");
        name_a.cmp(name_b)
    });
    
    // Truncate to limit to respect pagination roughly
    results.truncate(limit as usize);
    
    Ok(Json(json!({
        "count": results.len(),
        "next": null,
        "previous": null,
        "results": results
    })))
}

pub async fn get_vod_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    use crate::entities::vod_m3uvodcategoryrelation;
    
    let categories_with_relations = vod_category::Entity::find()
        .find_with_related(vod_m3uvodcategoryrelation::Entity)
        .all(&state.db)
        .await
        .unwrap_or_default();
    
    let mut results = Vec::new();
    for (c, relations) in categories_with_relations {
        let m3u_accounts: Vec<Value> = relations.into_iter().map(|r| json!({"m3u_account": r.m3u_account_id, "enabled": r.enabled})).collect();
        results.push(json!({
            "id": c.id,
            "name": c.name,
            "category_type": c.category_type,
            "m3u_accounts": m3u_accounts,
        }));
    }
    
    Ok(Json(json!({
        "count": results.len(),
        "next": null,
        "previous": null,
        "results": results
    })))
}

pub async fn get_vod_movies(
    State(_state): State<Arc<AppState>>,
    Query(_params): Query<Pagination>,
) -> Result<Json<Value>, StatusCode> {
    // Stub for now
    Ok(Json(json!({"count": 0, "results": []})))
}

pub async fn get_vod_series(
    State(_state): State<Arc<AppState>>,
    Query(_params): Query<Pagination>,
) -> Result<Json<Value>, StatusCode> {
    // Stub for now
    Ok(Json(json!({"count": 0, "results": []})))
}
