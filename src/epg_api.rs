use axum::{extract::{State, Json}, response::IntoResponse};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use serde_json::{json, Value};
use std::sync::Arc;
use chrono::{Utc, Duration, Timelike};
use crate::entities::{channel, epg_source, epg_program, epg_data};
use crate::AppState;

pub async fn get_epg_grid(State(state): State<Arc<AppState>>) -> Json<Value> {
    let now = Utc::now();
    let one_hour_ago = now - Duration::hours(1);
    let twenty_four_hours_later = now + Duration::hours(24);

    let programs = crate::entities::epg_program::Entity::find()
        .filter(crate::entities::epg_program::Column::EndTime.gt(one_hour_ago))
        .filter(crate::entities::epg_program::Column::StartTime.lt(twenty_four_hours_later))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut serialized_programs = vec![];
    for p in programs {
        let cp = p.custom_properties.unwrap_or(json!({}));
        let is_new = cp.get("new").and_then(|v: &Value| v.as_bool()).unwrap_or(false);
        let is_live = cp.get("live").and_then(|v: &Value| v.as_bool()).unwrap_or(false);
        let is_premiere = cp.get("premiere").and_then(|v: &Value| v.as_bool()).unwrap_or(false);
        let premiere_text = cp.get("premiere_text").and_then(|v: &Value| v.as_str()).unwrap_or("");
        let is_finale = premiere_text.to_lowercase().contains("finale");

        serialized_programs.push(json!({
            "id": p.id,
            "start_time": p.start_time.to_rfc3339(),
            "end_time": p.end_time.to_rfc3339(),
            "title": p.title,
            "sub_title": p.sub_title,
            "description": p.description,
            "tvg_id": p.tvg_id,
            "season": cp.get("season"),
            "episode": cp.get("episode"),
            "is_new": is_new,
            "is_live": is_live,
            "is_premiere": is_premiere,
            "is_finale": is_finale,
        }));
    }

    let channels = crate::entities::channel::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut dummy_programs = vec![];

    let time_descriptions: Vec<(u32, u32, Vec<&str>)> = vec![
        (0, 4, vec![
            "Late Night with {channel} - Where insomniacs unite!",
            "The 'Why Am I Still Awake?' Show on {channel}",
            "Counting Sheep - A {channel} production for the sleepless",
        ]),
        (4, 8, vec![
            "Dawn Patrol - Rise and shine with {channel}!",
            "Early Bird Special - Coffee not included",
            "Morning Zombies - Before coffee viewing on {channel}",
        ]),
        (8, 12, vec![
            "Mid-Morning Meetings - Pretend you're paying attention while watching {channel}",
            "The 'I Should Be Working' Hour on {channel}",
            "Productivity Killer - {channel}'s daytime programming",
        ]),
        (12, 16, vec![
            "Lunchtime Laziness with {channel}",
            "The Afternoon Slump - Brought to you by {channel}",
            "Post-Lunch Food Coma Theater on {channel}",
        ]),
        (16, 20, vec![
            "Rush Hour - {channel}'s alternative to traffic",
            "The 'What\\'s For Dinner?' Debate on {channel}",
            "Evening Escapism - {channel}'s remedy for reality",
        ]),
        (20, 24, vec![
            "Prime Time Pajamas on {channel}",
            "The 'Just One More Episode' Marathon on {channel}",
            "Nightly News of Nothing Much on {channel}",
        ]),
    ];

    for ch in channels {
        let mut needs_dummy = false;

        if ch.epg_data_id.is_none() {
            needs_dummy = true;
        } else if let Some(epg_id) = ch.epg_data_id {
            if let Ok(Some(epg_data_row)) = crate::entities::epg_data::Entity::find_by_id(epg_id).one(&state.db).await {
                if let Some(source_id) = epg_data_row.epg_source_id {
                    if let Ok(Some(source_row)) = crate::entities::epg_source::Entity::find_by_id(source_id).one(&state.db).await {
                        if source_row.source_type == "dummy" {
                            needs_dummy = true;
                        }
                    }
                }
            }
        }

        if needs_dummy {
            let dummy_tvg_id = ch.uuid.to_string();
            for hour_offset in (0..24).step_by(4) {
                let start_time = (now + Duration::hours(hour_offset as i64))
                    .with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
                let end_time = start_time + Duration::hours(4);
                let hour = start_time.hour();

                let mut description = format!("Placeholder program for {} - EPG data went on vacation", ch.name);
                for (start_range, end_range, descs) in &time_descriptions {
                    if hour >= *start_range && hour < *end_range {
                        let desc_idx = ((hour) as usize) % descs.len();
                        description = descs[desc_idx].replace("{channel}", &ch.name);
                        break;
                    }
                }

                dummy_programs.push(json!({
                    "id": format!("dummy-standard-{}-{}", ch.id, hour_offset),
                    "epg": {"tvg_id": dummy_tvg_id, "name": ch.name},
                    "start_time": start_time.to_rfc3339(),
                    "end_time": end_time.to_rfc3339(),
                    "title": ch.name,
                    "description": description,
                    "tvg_id": dummy_tvg_id,
                    "sub_title": None::<String>,
                    "custom_properties": None::<Value>,
                    "season": None::<Value>,
                    "episode": None::<Value>,
                    "is_new": false,
                    "is_live": false,
                    "is_premiere": false,
                    "is_finale": false,
                }));
            }
        }
    }

    serialized_programs.extend(dummy_programs);
    Json(json!({"data": serialized_programs}))
}

pub async fn get_current_programs(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let channel_uuids = payload.get("channel_uuids").and_then(|v: &Value| v.as_array());

    let mut query = crate::entities::channel::Entity::find()
        .filter(crate::entities::channel::Column::EpgDataId.is_not_null());

    if let Some(uuids) = channel_uuids {
        let uuids_str: Vec<String> = uuids.iter().filter_map(|v: &Value| v.as_str().map(|s: &str| s.to_string())).collect::<Vec<String>>();
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
    let mut current_programs = vec![];

    for ch in channels {
        if let Some(epg_id) = ch.epg_data_id {
            if let Ok(Some(program)) = crate::entities::epg_program::Entity::find()
                .filter(crate::entities::epg_program::Column::EpgId.eq(epg_id))
                .filter(crate::entities::epg_program::Column::StartTime.lte(now))
                .filter(crate::entities::epg_program::Column::EndTime.gt(now))
                .filter(crate::entities::epg_program::Column::TvgId.eq(ch.tvg_id.clone().unwrap_or_default()))
                .one(&state.db)
                .await
            {
                let cp = program.custom_properties.clone().unwrap_or(json!({}));
                let is_new = cp.get("new").and_then(|v: &Value| v.as_bool()).unwrap_or(false);
                let is_live = cp.get("live").and_then(|v: &Value| v.as_bool()).unwrap_or(false);
                let is_premiere = cp.get("premiere").and_then(|v: &Value| v.as_bool()).unwrap_or(false);
                let premiere_text = cp.get("premiere_text").and_then(|v: &Value| v.as_str()).unwrap_or("");
                let is_finale = premiere_text.to_lowercase().contains("finale");

                let mut prog_json = json!({
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
                    "channel_uuid": ch.uuid.to_string(),
                });

                if let Ok(Some(epg_data_row)) = crate::entities::epg_data::Entity::find_by_id(epg_id).one(&state.db).await {
                    prog_json["epg"] = json!({
                        "id": epg_data_row.id,
                        "tvg_id": epg_data_row.tvg_id,
                        "name": epg_data_row.name,
                    });
                }

                current_programs.push(prog_json);
            }
        }
    }

    Json(json!(current_programs))
}
