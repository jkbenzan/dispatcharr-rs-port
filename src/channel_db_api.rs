use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::channel_match::{calculate_match_score, parse_channel_name, StationForScoring};
use crate::entities::{channel, logo};
use crate::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn err_503(msg: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": msg })),
    )
}

fn err_500(e: String) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": e })),
    )
}

fn err_400(msg: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": msg })),
    )
}

// ---------------------------------------------------------------------------
// Metadata & Status
// ---------------------------------------------------------------------------

pub async fn health(State(state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() {
        return Err(err_503("Channel data database is not available"));
    }
    
    let meta = db.get_metadata().await.map_err(err_500)?;
    Ok(Json(json!({
        "status": "ok",
        "station_count": meta.station_count,
        "version": meta.version
    })))
}

pub async fn get_metadata(State(state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    let meta = db.get_metadata().await.map_err(err_500)?;
    Ok(Json(serde_json::to_value(meta).unwrap()))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    let stats = db.get_stats().await.map_err(err_500)?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

pub async fn get_filter_options(State(state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    let opts = db.get_filter_options().await.map_err(err_500)?;
    Ok(Json(serde_json::to_value(opts).unwrap()))
}

// ---------------------------------------------------------------------------
// Search & Browse
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct StationSearchQuery {
    q: Option<String>,
    country: Option<String>,
    quality: Option<String>,
    limit: Option<usize>,
}

pub async fn search_stations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StationSearchQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    
    let q = query.q.unwrap_or_default();
    if q.is_empty() {
        return Ok(Json(json!([])));
    }
    
    tracing::info!("🔍 Invoking channel_data.db for search query: '{}'", q);
    
    let results = db.search_stations(&q, query.country.as_deref(), query.quality.as_deref(), query.limit.unwrap_or(20)).await.map_err(err_500)?;
    
    Ok(Json(serde_json::to_value(results).unwrap()))
}

pub async fn get_station(
    State(state): State<Arc<AppState>>,
    Path(station_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    
    let station = db.get_station(&station_id).await.map_err(err_500)?;
    if let Some(s) = station {
        Ok(Json(serde_json::to_value(s).unwrap()))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Station not found" }))))
    }
}

// ---------------------------------------------------------------------------
// Lineups
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LineupSearchQuery {
    zip: Option<String>,
    country: Option<String>,
}

pub async fn search_lineups_by_zip(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LineupSearchQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    
    let zip = query.zip.unwrap_or_default();
    let country = query.country.unwrap_or_else(|| "USA".to_string());
    
    if zip.is_empty() {
        return Err(err_400("zip code is required"));
    }
    
    let results = db.search_lineups_by_zip(&country, &zip).await.map_err(err_500)?;
    Ok(Json(serde_json::to_value(results).unwrap()))
}

pub async fn preview_lineup(
    State(state): State<Arc<AppState>>,
    Path(lineup_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    
    let preview = db.preview_lineup(&lineup_id).await.map_err(err_500)?;
    if let Some(p) = preview {
        Ok(Json(serde_json::to_value(p).unwrap()))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Lineup not found" }))))
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ImportLineupPayload {
    include_sd: bool,
    include_hd: bool,
    include_uhd: bool,
    include_unknown: bool,
    number_offset: Option<f64>,
    conflict_resolution: Option<String>, // "skip", "overwrite", "move"
    group_id: Option<i64>,
}

pub async fn import_lineup(
    State(_state): State<Arc<AppState>>,
    Path(_lineup_id): Path<String>,
    Json(_payload): Json<ImportLineupPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Basic wrapper, actual logic would integrate heavily with entities/channels
    // For now, return a 501 Not Implemented or minimal logic
    Ok(Json(json!({ "message": "Import lineup functionality needs complex Postgres logic implementation." })))
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SuggestQuery {
    channel_name: String,
    channel_id: Option<i64>,
    #[allow(dead_code)]
    existing_station_id: Option<String>,
    filter_country: Option<String>,
    filter_resolutions: Option<Vec<String>>,
}

pub async fn suggest_matches(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SuggestQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }
    
    if payload.channel_name.is_empty() {
        return Err(err_400("channel_name is required"));
    }
    
    let parsed = parse_channel_name(&payload.channel_name);
    
    tracing::info!(
        "🔍 Channel Data Lookup: original='{}' clean='{}' country={:?} resolution={:?}",
        payload.channel_name, parsed.clean_name, parsed.country, parsed.resolution
    );
    
    // Primary search: use the parsed clean_name (with country/resolution/noise stripped)
    let mut results = db.search_for_matching(
        &parsed.clean_name, 
        payload.filter_country.as_deref(), 
        payload.filter_resolutions.as_deref(), 
        20
    ).await.map_err(err_500)?;
    
    // Fallback: if parsing produced a different (shorter) clean name and got 0 results,
    // retry with the original channel_name. This prevents overly aggressive parsing
    // from eliminating valid matches (e.g. if a station name happens to contain a
    // country code or resolution keyword).
    if results.is_empty() && parsed.clean_name.to_uppercase() != payload.channel_name.to_uppercase() {
        tracing::info!(
            "🔄 Channel Data Lookup: clean_name '{}' returned 0 results, retrying with original '{}'",
            parsed.clean_name, payload.channel_name
        );
        results = db.search_for_matching(
            &payload.channel_name,
            payload.filter_country.as_deref(),
            payload.filter_resolutions.as_deref(),
            20
        ).await.map_err(err_500)?;
    }
    
    let mut matches = Vec::new();
    for res in results {
        let scoring_station = StationForScoring {
            name: res.name.as_deref(),
            call_sign: res.call_sign.as_deref(),
            video_types: res.video_types.as_deref(),
            country: None,
            has_logo: res.logo_uri.is_some() && !res.logo_uri.as_ref().unwrap().is_empty(),
        };
        
        let score = calculate_match_score(&payload.channel_name, &scoring_station, Some(&parsed));
        
        matches.push(json!({
            "station": res,
            "score": score,
            "confidence": if score > 0.8 { "high" } else if score > 0.5 { "medium" } else { "low" }
        }));
    }
    
    matches.sort_by(|a, b| {
        let score_a = a["score"].as_f64().unwrap_or(0.0);
        let score_b = b["score"].as_f64().unwrap_or(0.0);
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let top_matches: Vec<_> = matches.into_iter().take(5).collect();
    
    Ok(Json(json!({
        "channel_id": payload.channel_id,
        "channel_name": payload.channel_name,
        "parsed": parsed,
        "matches": top_matches,
    })))
}

#[derive(Deserialize)]
pub struct ApplyMatchPayload {
    channel_id: i64,
    station_id: String,
    apply_station_id: bool,
    apply_channel_name: bool,
    apply_tvg_id: bool,
    apply_logo: bool,
}

pub async fn apply_match(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ApplyMatchPayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.channel_db.read().await;
    if !db.is_available() { return Err(err_503("Not available")); }

    let station = db.get_station(&payload.station_id).await.map_err(err_500)?;
    if station.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Station not found" }))));
    }
    let station = station.unwrap();

    let channel_model = channel::Entity::find_by_id(payload.channel_id)
        .one(&state.db)
        .await
        .map_err(|e| err_500(e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Channel not found" })),
            )
        })?;

    let mut active: channel::ActiveModel = channel_model.into();
    let mut applied_fields = Vec::new();

    if payload.apply_station_id {
        active.tvc_guide_stationid = Set(Some(station.station_id.clone()));
        applied_fields.push("tvc_guide_stationid");
    }

    if payload.apply_channel_name {
        if let Some(name) = station.name.as_ref().filter(|name| !name.trim().is_empty()) {
            active.name = Set(name.trim().to_string());
            applied_fields.push("name");
        }
    }

    if payload.apply_tvg_id {
        let tvg_id = station
            .call_sign
            .as_ref()
            .filter(|call_sign| !call_sign.trim().is_empty())
            .or_else(|| station.name.as_ref().filter(|name| !name.trim().is_empty()))
            .map(|value| value.trim().to_string());

        if let Some(tvg_id) = tvg_id {
            active.tvg_id = Set(Some(tvg_id));
            applied_fields.push("tvg_id");
        }
    }

    let mut applied_logo = None;
    if payload.apply_logo {
        if let Some(logo_uri) = station.logo_uri.as_ref().filter(|uri| !uri.trim().is_empty()) {
            let logo_uri = logo_uri.trim().to_string();
            let existing_logo = logo::Entity::find()
                .filter(logo::Column::Url.eq(&logo_uri))
                .one(&state.db)
                .await
                .map_err(|e| err_500(e.to_string()))?;

            let logo_model = if let Some(existing_logo) = existing_logo {
                existing_logo
            } else {
                logo::ActiveModel {
                    name: Set(station.name.clone().unwrap_or_else(|| station.station_id.clone())),
                    url: Set(logo_uri),
                    ..Default::default()
                }
                .insert(&state.db)
                .await
                .map_err(|e| err_500(e.to_string()))?
            };

            active.logo_id = Set(Some(logo_model.id));
            applied_logo = Some(json!({
                "id": logo_model.id,
                "name": logo_model.name,
                "url": logo_model.url,
            }));
            applied_fields.push("logo_id");
        }
    }

    active.updated_at = Set(chrono::Utc::now().into());
    let updated_channel = active
        .update(&state.db)
        .await
        .map_err(|e| err_500(e.to_string()))?;

    Ok(Json(json!({
        "success": true,
        "applied_fields": applied_fields,
        "channel": updated_channel,
        "logo": applied_logo,
        "station": station
    })))
}

pub async fn batch_match(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({ "message": "Batch match placeholder" })))
}

// ---------------------------------------------------------------------------
// Remote Updates
// ---------------------------------------------------------------------------

pub async fn check_update(State(_state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({ "message": "Check update placeholder" })))
}

pub async fn download_update(State(_state): State<Arc<AppState>>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({ "message": "Download update placeholder" })))
}
