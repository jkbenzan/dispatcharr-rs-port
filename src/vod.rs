use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::entities::{vod_category, vod_movie, vod_series, vod_m3umovierelation, vod_m3useriesrelation, vod_episode, vod_m3uepisoderelation};
use crate::AppState;
use axum::extract::Path;

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub search: Option<String>,
    pub category_id: Option<i64>,
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
            sea_orm::Condition::any().add(
                sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(
                    sea_orm::sea_query::Expr::col(vod_movie::Column::Name),
                ))
                .like(format!("%{}%", search)),
            ),
        );
        series_q = series_q.filter(
            sea_orm::Condition::any().add(
                sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(
                    sea_orm::sea_query::Expr::col(vod_series::Column::Name),
                ))
                .like(format!("%{}%", search)),
            ),
        );
    }

    if let Some(cat_id) = params.category_id {
        movies_q = movies_q.filter(
            vod_movie::Column::Id.in_subquery(
                vod_m3umovierelation::Entity::find()
                    .select_only()
                    .column(vod_m3umovierelation::Column::MovieId)
                    .filter(vod_m3umovierelation::Column::CategoryId.eq(cat_id))
                    .into_query(),
            ),
        );
        series_q = series_q.filter(
            vod_series::Column::Id.in_subquery(
                vod_m3useriesrelation::Entity::find()
                    .select_only()
                    .column(vod_m3useriesrelation::Column::SeriesId)
                    .filter(vod_m3useriesrelation::Column::CategoryId.eq(cat_id))
                    .into_query(),
            ),
        );
    }

    // For a unified view, fetch up to limit from both, combine, sort, and slice
    let movies = movies_q
        .limit(limit)
        .all(&state.db)
        .await
        .unwrap_or_default();
    let series = series_q
        .limit(limit)
        .all(&state.db)
        .await
        .unwrap_or_default();

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

    let categories = vod_category::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    let relations = vod_m3uvodcategoryrelation::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut rel_map: std::collections::HashMap<i64, Vec<Value>> = std::collections::HashMap::new();
    for r in relations {
        let entry = rel_map.entry(r.category_id).or_default();
        entry.push(json!({
            "m3u_account": r.m3u_account_id,
            "enabled": r.enabled,
            "stream_count": r.custom_properties.as_ref()
                .and_then(|cp| cp.get("stream_count"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
        }));
    }

    let mut results = Vec::new();
    for c in categories {
        results.push(json!({
            "id": c.id,
            "name": c.name,
            "category_type": c.category_type,
            "m3u_accounts": rel_map.get(&c.id).unwrap_or(&Vec::new()),
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
    State(state): State<Arc<AppState>>,
    Query(params): Query<Pagination>,
) -> Result<Json<Value>, StatusCode> {
    let search = params.search.unwrap_or_default().to_lowercase();
    let limit = params.page_size.unwrap_or(24);
    let page = params.page.unwrap_or(1);
    let offset = (page.saturating_sub(1)) * limit;

    let mut q = vod_movie::Entity::find();

    if !search.is_empty() {
        q = q.filter(
            sea_orm::Condition::any().add(
                sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(
                    sea_orm::sea_query::Expr::col(vod_movie::Column::Name),
                ))
                .like(format!("%{}%", search)),
            ),
        );
    }

    if let Some(cat_id) = params.category_id {
        q = q.filter(
            vod_movie::Column::Id.in_subquery(
                vod_m3umovierelation::Entity::find()
                    .select_only()
                    .column(vod_m3umovierelation::Column::MovieId)
                    .filter(vod_m3umovierelation::Column::CategoryId.eq(cat_id))
                    .into_query(),
            ),
        );
    }

    let count = q.clone().count(&state.db).await.unwrap_or(0);

    let mut q_paged = q
        .order_by_asc(vod_movie::Column::Id)
        .limit(limit)
        .offset(offset);

    q_paged = q_paged.group_by(vod_movie::Column::Name).group_by(vod_movie::Column::Year);

    let movies = q_paged
        .all(&state.db)
        .await
        .unwrap_or_default();

    // Fetch providers for these movies
    // Fetch M3U relations for all movies to provide multiple stream options
    
    // We need all relations for these movie names and years to group providers.
    // Or simpler: just fetch relations for these specific movie IDs for now.
    // Wait, if we group by name, we only get ONE movie ID per group.
    // To get all providers for a movie name/year, we need to find all movie IDs with those names/years.
    let mut names = Vec::new();
    for m in &movies {
        names.push(m.name.clone());
    }

    let all_matching_movies = vod_movie::Entity::find()
        .filter(vod_movie::Column::Name.is_in(names))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_matching_movie_ids: Vec<i64> = all_matching_movies.iter().map(|m| m.id).collect();

    let relations = vod_m3umovierelation::Entity::find()
        .filter(vod_m3umovierelation::Column::MovieId.is_in(all_matching_movie_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();

    // Group relations by Name + Year
    let mut providers_by_name_year: std::collections::HashMap<(String, Option<i32>), Vec<serde_json::Value>> = std::collections::HashMap::new();

    let id_to_movie: std::collections::HashMap<i64, &crate::entities::vod_movie::Model> = all_matching_movies.iter().map(|m| (m.id, m)).collect();

    for r in relations {
        if let Some(m) = id_to_movie.get(&r.movie_id) {
            let key = (m.name.clone(), m.year);
            let entry = providers_by_name_year.entry(key).or_default();
            entry.push(json!({
                "stream_id": r.stream_id,
                "m3u_account_id": r.m3u_account_id,
                "container_extension": r.container_extension,
                "movie_id": r.movie_id
            }));
        }
    }

    let mut results = Vec::new();
    for m in movies {
        let mut val = serde_json::to_value(&m).unwrap_or(json!({}));
        let key = (m.name.clone(), m.year);
        val["providers"] = json!(providers_by_name_year.get(&key).unwrap_or(&Vec::new()));
        results.push(val);
    }

    Ok(Json(json!({
        "count": count,
        "results": results
    })))
}

pub async fn get_vod_series(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Pagination>,
) -> Result<Json<Value>, StatusCode> {
    use sea_orm::{PaginatorTrait, QueryOrder};
    let search = params.search.unwrap_or_default().to_lowercase();
    let page = params.page.unwrap_or(1);
    let limit = params.page_size.unwrap_or(50);
    let offset = (page.saturating_sub(1)) * limit;

    let mut query = vod_series::Entity::find();

    if !search.is_empty() {
        query = query.filter(
            sea_orm::Condition::any().add(
                sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(
                    sea_orm::sea_query::Expr::col(vod_series::Column::Name),
                ))
                .like(format!("%{}%", search)),
            ),
        );
    }

    if let Some(cat_id) = params.category_id {
        query = query.filter(
            vod_series::Column::Id.in_subquery(
                vod_m3useriesrelation::Entity::find()
                    .select_only()
                    .column(vod_m3useriesrelation::Column::SeriesId)
                    .filter(vod_m3useriesrelation::Column::CategoryId.eq(cat_id))
                    .into_query(),
            ),
        );
    }

    let count = query.clone().count(&state.db).await.unwrap_or(0);

    let mut query_paged = query
        .order_by_asc(vod_series::Column::Id)
        .limit(limit)
        .offset(offset);

    query_paged = query_paged.group_by(vod_series::Column::Name).group_by(vod_series::Column::Year);

    let series = query_paged
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut names = Vec::new();
    for s in &series {
        names.push(s.name.clone());
    }

    let all_matching_series = vod_series::Entity::find()
        .filter(vod_series::Column::Name.is_in(names))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let all_matching_series_ids: Vec<i64> = all_matching_series.iter().map(|s| s.id).collect();

    let relations = vod_m3useriesrelation::Entity::find()
        .filter(vod_m3useriesrelation::Column::SeriesId.is_in(all_matching_series_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut providers_by_name_year: std::collections::HashMap<(String, Option<i32>), Vec<serde_json::Value>> = std::collections::HashMap::new();

    let id_to_series: std::collections::HashMap<i64, &crate::entities::vod_series::Model> = all_matching_series.iter().map(|s| (s.id, s)).collect();

    for r in relations {
        if let Some(s) = id_to_series.get(&r.series_id) {
            let key = (s.name.clone(), s.year);
            let entry = providers_by_name_year.entry(key).or_default();
            entry.push(json!({
                "m3u_account_id": r.m3u_account_id,
                "series_id": r.series_id
            }));
        }
    }

    let mut results = Vec::new();
    for s in series {
        let mut val = json!({
            "id": s.id,
            "uuid": s.uuid,
            "name": s.name,
            "description": s.description,
            "year": s.year,
            "rating": s.rating,
            "genre": s.genre,
            "tmdb_id": s.tmdb_id,
            "imdb_id": s.imdb_id,
            "custom_properties": s.custom_properties,
            "created_at": s.created_at,
            "updated_at": s.updated_at,
            "logo_id": s.logo_id,
        });
        
        let key = (s.name.clone(), s.year);
        val["providers"] = json!(providers_by_name_year.get(&key).unwrap_or(&Vec::new()));
        
        results.push(val);
    }

    Ok(Json(json!({
        "count": count,
        "next": if (offset + limit) < count { Some(page + 1) } else { None },
        "previous": if page > 1 { Some(page - 1) } else { None },
        "results": results
    })))
}

pub async fn get_vod_episodes(
    State(state): State<Arc<AppState>>,
    Path(series_id): Path<i64>,
) -> Result<Json<Value>, StatusCode> {
    let series = vod_series::Entity::find_by_id(series_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut query = vod_series::Entity::find()
        .filter(vod_series::Column::Name.eq(series.name.clone()));
    
    if let Some(y) = series.year {
        query = query.filter(vod_series::Column::Year.eq(y));
    } else {
        query = query.filter(vod_series::Column::Year.is_null());
    }

    let all_matching_series = query
        .all(&state.db)
        .await
        .unwrap_or_default();

    let series_ids: Vec<i64> = all_matching_series.iter().map(|s| s.id).collect();

    let episodes = vod_episode::Entity::find()
        .filter(vod_episode::Column::SeriesId.is_in(series_ids))
        .order_by_asc(vod_episode::Column::SeasonNumber)
        .order_by_asc(vod_episode::Column::EpisodeNumber)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let episode_ids: Vec<i64> = episodes.iter().map(|e| e.id).collect();
    let relations = vod_m3uepisoderelation::Entity::find()
        .filter(vod_m3uepisoderelation::Column::EpisodeId.is_in(episode_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut providers_by_episode: std::collections::HashMap<i64, Vec<serde_json::Value>> = std::collections::HashMap::new();
    for r in relations {
        let entry = providers_by_episode.entry(r.episode_id).or_default();
        entry.push(json!({
            "stream_id": r.stream_id,
            "m3u_account_id": r.m3u_account_id,
            "container_extension": r.container_extension,
            "episode_id": r.episode_id
        }));
    }

    let mut grouped_episodes: std::collections::HashMap<(Option<i32>, Option<i32>), Value> = std::collections::HashMap::new();

    for e in episodes {
        let key = (e.season_number, e.episode_number);
        
        let mut ep_json = serde_json::to_value(&e).unwrap_or(json!({}));
        let mut current_providers = providers_by_episode.remove(&e.id).unwrap_or_default();
        
        if let std::collections::hash_map::Entry::Occupied(mut entry) = grouped_episodes.entry(key) {
            let existing = entry.get_mut();
            if let Some(existing_providers) = existing.get_mut("providers").and_then(|v| v.as_array_mut()) {
                existing_providers.append(&mut current_providers);
            }
        } else {
            ep_json["providers"] = json!(current_providers);
            grouped_episodes.insert(key, ep_json);
        }
    }

    let mut results: Vec<Value> = grouped_episodes.into_values().collect();
    results.sort_by(|a, b| {
        let s_a = a.get("season_number").and_then(|v| v.as_i64()).unwrap_or(0);
        let s_b = b.get("season_number").and_then(|v| v.as_i64()).unwrap_or(0);
        if s_a == s_b {
            let e_a = a.get("episode_number").and_then(|v| v.as_i64()).unwrap_or(0);
            let e_b = b.get("episode_number").and_then(|v| v.as_i64()).unwrap_or(0);
            e_a.cmp(&e_b)
        } else {
            s_a.cmp(&s_b)
        }
    });

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn get_enrich_progress(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let movies_remaining = vod_movie::Entity::find()
        .filter(vod_movie::Column::TmdbId.is_null())
        .count(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let series_remaining = vod_series::Entity::find()
        .filter(vod_series::Column::TmdbId.is_null())
        .count(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_remaining = movies_remaining + series_remaining;

    Ok(Json(json!({
        "movies_remaining": movies_remaining,
        "series_remaining": series_remaining,
        "total_remaining": total_remaining
    })))
}
