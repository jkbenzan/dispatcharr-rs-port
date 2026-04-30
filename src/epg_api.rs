use crate::AppState;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::{Duration, Timelike, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

fn serialize_program(program: crate::entities::epg_program::Model) -> Value {
    let cp = program.custom_properties.clone().unwrap_or(json!({}));
    let is_new = cp
        .get("new")
        .and_then(|v: &Value| v.as_bool())
        .unwrap_or(false);
    let is_live = cp
        .get("live")
        .and_then(|v: &Value| v.as_bool())
        .unwrap_or(false);
    let is_premiere = cp
        .get("premiere")
        .and_then(|v: &Value| v.as_bool())
        .unwrap_or(false);
    let premiere_text = cp
        .get("premiere_text")
        .and_then(|v: &Value| v.as_str())
        .unwrap_or("");
    let is_finale = premiere_text.to_lowercase().contains("finale");

    json!({
        "id": program.id,
        "start_time": program.start_time.to_rfc3339(),
        "end_time": program.end_time.to_rfc3339(),
        "title": program.title,
        "sub_title": program.sub_title,
        "description": program.description,
        "tvg_id": program.tvg_id,
        "season": cp.get("season"),
        "episode": cp.get("episode"),
        "is_new": is_new,
        "is_live": is_live,
        "is_premiere": is_premiere,
        "is_finale": is_finale,
    })
}

pub async fn get_epg_grid(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let started_at = Instant::now();
    tracing::info!("EPG grid request started with params: {:?}", params);

    let now = Utc::now();
    
    // Parse start/end from query or default to [Now - 1h, Now + 4h]
    let start_time = params.get("start")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| now - Duration::hours(1));
        
    let end_time = params.get("end")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| start_time + Duration::hours(5));

    tracing::info!("EPG grid range: {} to {}", start_time, end_time);

    let programs = crate::entities::epg_program::Entity::find()
        .filter(crate::entities::epg_program::Column::EndTime.gt(start_time))
        .filter(crate::entities::epg_program::Column::StartTime.lt(end_time))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut serialized_programs = Vec::with_capacity(programs.len());
    for p in programs {
        serialized_programs.push(serialize_program(p));
    }

    let channels = crate::entities::channel::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    let channel_count = channels.len();

    let epg_data_ids: Vec<i64> = channels.iter().filter_map(|ch| ch.epg_data_id).collect();
    let epg_data_by_id: HashMap<i64, crate::entities::epg_data::Model> = if epg_data_ids.is_empty() {
        HashMap::new()
    } else {
        crate::entities::epg_data::Entity::find()
            .filter(crate::entities::epg_data::Column::Id.is_in(epg_data_ids))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row.id, row))
            .collect()
    };

    let source_ids: Vec<i64> = epg_data_by_id
        .values()
        .filter_map(|row| row.epg_source_id)
        .collect();
    let dummy_source_ids: HashSet<i64> = if source_ids.is_empty() {
        HashSet::new()
    } else {
        crate::entities::epg_source::Entity::find()
            .filter(crate::entities::epg_source::Column::Id.is_in(source_ids))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.source_type == "dummy")
            .map(|row| row.id)
            .collect()
    };

    let mut dummy_programs = vec![];

    let time_descriptions: Vec<(u32, u32, Vec<&str>)> = vec![
        (0, 4, vec!["Late Night with {channel}", "The 'Why Am I Still Awake?' Show on {channel}"]),
        (4, 8, vec!["Dawn Patrol - Rise and shine with {channel}!", "Early Bird Special"]),
        (8, 12, vec!["Mid-Morning Meetings on {channel}", "The 'I Should Be Working' Hour"]),
        (12, 16, vec!["Lunchtime Laziness with {channel}", "The Afternoon Slump"]),
        (16, 20, vec!["Rush Hour on {channel}", "Evening Escapism"]),
        (20, 24, vec!["Prime Time Pajamas on {channel}", "Just One More Episode"]),
    ];

    // Determine how many hours we need dummy data for
    let duration_hours = (end_time - start_time).num_hours().max(1);

    for ch in channels {
        let needs_dummy = match ch.epg_data_id {
            None => true,
            Some(epg_id) => epg_data_by_id
                .get(&epg_id)
                .and_then(|row| row.epg_source_id)
                .map(|source_id| dummy_source_ids.contains(&source_id))
                .unwrap_or(false),
        };

        if needs_dummy {
            let dummy_tvg_id = ch.uuid.to_string();
            // Generate dummy blocks within the requested range
            let mut current_block_start = start_time;
            while current_block_start < end_time {
                let block_start = current_block_start;
                let block_end = (block_start + Duration::hours(4)).min(end_time);
                let hour = block_start.hour();

                let mut description = format!("Placeholder program for {}", ch.name);
                for (start_range, end_range, descs) in &time_descriptions {
                    if hour >= *start_range && hour < *end_range {
                        description = descs[hour as usize % descs.len()].replace("{channel}", &ch.name);
                        break;
                    }
                }

                dummy_programs.push(json!({
                    "id": format!("dummy-{}-{}-{}", ch.id, block_start.timestamp(), block_end.timestamp()),
                    "epg": {"tvg_id": dummy_tvg_id, "name": ch.name},
                    "start_time": block_start.to_rfc3339(),
                    "end_time": block_end.to_rfc3339(),
                    "title": ch.name,
                    "description": description,
                    "tvg_id": dummy_tvg_id,
                    "sub_title": None::<String>,
                    "is_new": false, "is_live": false, "is_premiere": false, "is_finale": false,
                }));
                current_block_start = block_end;
            }
        }
    }

    serialized_programs.extend(dummy_programs);
    tracing::info!(
        program_count = serialized_programs.len(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "EPG grid request completed"
    );
    Json(json!({"data": serialized_programs}))
}

pub async fn get_current_programs(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let channel_uuids = payload
        .get("channel_uuids")
        .and_then(|v: &Value| v.as_array());

    let mut query = crate::entities::channel::Entity::find()
        .filter(crate::entities::channel::Column::EpgDataId.is_not_null());

    if let Some(uuids) = channel_uuids {
        let uuids_str: Vec<String> = uuids
            .iter()
            .filter_map(|v: &Value| v.as_str().map(|s: &str| s.to_string()))
            .collect::<Vec<String>>();
        let mut uuid_vals = vec![];
        for u_str in uuids_str.into_iter() {
            if let Ok(u) = uuid::Uuid::parse_str(&u_str) {
                uuid_vals.push(u);
            }
        }
        query = query.filter(crate::entities::channel::Column::Uuid.is_in(uuid_vals));
    }

    let channels = query.all(&state.db).await.unwrap_or_default();
    let now = Utc::now();
    let epg_ids: Vec<i64> = channels
        .iter()
        .filter_map(|ch| ch.epg_data_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if epg_ids.is_empty() {
        return Json(json!([]));
    }

    let epg_rows = crate::entities::epg_data::Entity::find()
        .filter(crate::entities::epg_data::Column::Id.is_in(epg_ids.clone()))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let epg_by_id: HashMap<i64, crate::entities::epg_data::Model> =
        epg_rows.into_iter().map(|row| (row.id, row)).collect();

    let programs = crate::entities::epg_program::Entity::find()
        .filter(crate::entities::epg_program::Column::EpgId.is_in(epg_ids))
        .filter(crate::entities::epg_program::Column::StartTime.lte(now))
        .filter(crate::entities::epg_program::Column::EndTime.gt(now))
        .order_by_desc(crate::entities::epg_program::Column::StartTime)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut program_by_epg_id = HashMap::new();
    for program in programs {
        program_by_epg_id.entry(program.epg_id).or_insert(program);
    }

    let mut current_programs = Vec::new();

    for ch in channels {
        let Some(epg_id) = ch.epg_data_id else {
            continue;
        };
        let Some(program) = program_by_epg_id.get(&epg_id) else {
            continue;
        };

        let mut prog_json = serialize_program(program.clone());
        prog_json["channel_uuid"] = json!(ch.uuid.to_string());

        if let Some(epg_data_row) = epg_by_id.get(&epg_id) {
            prog_json["epg"] = json!({
                "id": epg_data_row.id,
                "tvg_id": epg_data_row.tvg_id,
                "name": epg_data_row.name,
            });
        }

        current_programs.push(prog_json);
    }

    Json(json!(current_programs))
}

pub async fn get_program_detail(
    State(state): State<Arc<AppState>>,
    Path(program_id): Path<i64>,
) -> Result<Json<Value>, StatusCode> {
    let program = crate::entities::epg_program::Entity::find_by_id(program_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let cp = program.custom_properties.clone().unwrap_or(json!({}));
    let credits = cp.get("credits").cloned().unwrap_or_else(|| json!({}));
    let video = cp.get("video").cloned().unwrap_or_else(|| json!({}));
    let audio = cp.get("audio").cloned().unwrap_or_else(|| json!({}));
    let previously_shown = cp
        .get("previously_shown_details")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let premiere_text = cp
        .get("premiere_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    Ok(Json(json!({
        "id": program.id,
        "start_time": program.start_time.to_rfc3339(),
        "end_time": program.end_time.to_rfc3339(),
        "title": program.title,
        "sub_title": program.sub_title,
        "description": program.description,
        "tvg_id": program.tvg_id,
        "season": cp.get("season").cloned(),
        "episode": cp.get("episode").cloned(),
        "is_new": cp.get("new").and_then(|v| v.as_bool()).unwrap_or(false),
        "is_live": cp.get("live").and_then(|v| v.as_bool()).unwrap_or(false),
        "is_premiere": cp.get("premiere").and_then(|v| v.as_bool()).unwrap_or(false),
        "is_finale": !premiere_text.is_empty() && premiere_text.to_lowercase().contains("finale"),
        "categories": cp.get("categories").cloned().unwrap_or_else(|| json!([])),
        "rating": cp.get("rating").cloned(),
        "rating_system": cp.get("rating_system").cloned(),
        "star_ratings": cp.get("star_ratings").cloned().unwrap_or_else(|| json!([])),
        "credits": {
            "actors": credits.get("actor").cloned().unwrap_or_else(|| json!([])),
            "directors": credits.get("director").cloned().unwrap_or_else(|| json!([])),
            "writers": credits.get("writer").cloned().unwrap_or_else(|| json!([])),
            "producers": credits.get("producer").cloned().unwrap_or_else(|| json!([])),
            "presenters": credits.get("presenter").cloned().unwrap_or_else(|| json!([])),
        },
        "video_quality": video.get("quality").cloned(),
        "aspect_ratio": video.get("aspect").cloned(),
        "stereo": audio.get("stereo").cloned(),
        "is_previously_shown": cp.get("previously_shown").is_some(),
        "country": cp.get("country").cloned(),
        "language": cp.get("language").cloned(),
        "production_date": cp.get("date").cloned(),
        "original_air_date": previously_shown.get("start").cloned(),
        "imdb_id": cp.get("imdb.com_id").cloned(),
        "tmdb_id": cp.get("themoviedb.org_id").cloned(),
        "tvdb_id": cp.get("thetvdb.com_id").cloned(),
        "tmdb_media_type": cp.get("tmdb_media_type").cloned(),
        "icon": cp.get("icon").cloned(),
        "images": cp.get("images").cloned().unwrap_or_else(|| json!([])),
    })))
}
