import os

code = """
// --- CHANNELS UPDATING ---

pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let channel_model_opt = crate::entities::channel::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let channel_model = match channel_model_opt {
        Some(c) => c,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let mut active: crate::entities::channel::ActiveModel = channel_model.into();
    let mut updated = false;

    if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
        active.name = sea_orm::Set(name.to_string());
        updated = true;
    }
    if let Some(num) = payload.get("channel_number") {
        if let Some(n) = num.as_f64() {
            active.channel_number = sea_orm::Set(n);
            updated = true;
        } else if let Some(n) = num.as_i64() {
            active.channel_number = sea_orm::Set(n as f64);
            updated = true;
        } else if let Some(n) = num.as_str().and_then(|s| s.parse::<f64>().ok()) {
            active.channel_number = sea_orm::Set(n);
            updated = true;
        }
    }
    if let Some(cg) = payload.get("channel_group_id") {
        let cg_id = if cg.is_null() { None } else { cg.as_i64() };
        active.channel_group_id = sea_orm::Set(cg_id);
        updated = true;
    }
    if let Some(sp) = payload.get("stream_profile_id") {
        let sp_id = if sp.is_null() { None } else { sp.as_i64() };
        active.stream_profile_id = sea_orm::Set(sp_id);
        updated = true;
    }
    if let Some(epg) = payload.get("epg_data_id") {
        let epg_id = if epg.is_null() { None } else { epg.as_i64() };
        active.epg_data_id = sea_orm::Set(epg_id);
        updated = true;
    }

    if updated {
        let _ = active.update(&state.db).await;
    }

    if let Some(streams) = payload.get("streams").and_then(|v| v.as_array()) {
        let _ = crate::entities::channel_stream::Entity::delete_many()
            .filter(crate::entities::channel_stream::Column::ChannelId.eq(id))
            .exec(&state.db)
            .await;

        let mut order = 0;
        let mut inserts = Vec::new();
        for stream_val in streams {
            if let Some(stream_id) = stream_val.as_i64() {
                inserts.push(crate::entities::channel_stream::ActiveModel {
                    channel_id: sea_orm::Set(id),
                    stream_id: sea_orm::Set(stream_id),
                    order: sea_orm::Set(order),
                    ..Default::default()
                });
                order += 1;
            }
        }
        if !inserts.is_empty() {
            let _ = crate::entities::channel_stream::Entity::insert_many(inserts)
                .exec(&state.db)
                .await;
        }
    }

    Ok(Json(serde_json::json!({"id": id, "success": true})))
}

pub async fn bulk_update_channels(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Vec<serde_json::Value>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    for channel_payload in payload {
        if let Some(id) = channel_payload.get("id").and_then(|v| v.as_i64()) {
            let channel_model_opt = crate::entities::channel::Entity::find_by_id(id)
                .one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if let Some(channel_model) = channel_model_opt {
                let mut active: crate::entities::channel::ActiveModel = channel_model.into();
                let mut updated = false;

                if let Some(name) = channel_payload.get("name").and_then(|v| v.as_str()) {
                    active.name = sea_orm::Set(name.to_string());
                    updated = true;
                }
                if let Some(num) = channel_payload.get("channel_number") {
                    if let Some(n) = num.as_f64() {
                        active.channel_number = sea_orm::Set(n);
                        updated = true;
                    } else if let Some(n) = num.as_i64() {
                        active.channel_number = sea_orm::Set(n as f64);
                        updated = true;
                    } else if let Some(n) = num.as_str().and_then(|s| s.parse::<f64>().ok()) {
                        active.channel_number = sea_orm::Set(n);
                        updated = true;
                    }
                }
                if let Some(cg) = channel_payload.get("channel_group_id") {
                    let cg_id = if cg.is_null() { None } else { cg.as_i64() };
                    active.channel_group_id = sea_orm::Set(cg_id);
                    updated = true;
                }
                if let Some(sp) = channel_payload.get("stream_profile_id") {
                    let sp_id = if sp.is_null() { None } else { sp.as_i64() };
                    active.stream_profile_id = sea_orm::Set(sp_id);
                    updated = true;
                }
                if let Some(epg) = channel_payload.get("epg_data_id") {
                    let epg_id = if epg.is_null() { None } else { epg.as_i64() };
                    active.epg_data_id = sea_orm::Set(epg_id);
                    updated = true;
                }

                if updated {
                    let _ = active.update(&state.db).await;
                }
                
                if let Some(streams) = channel_payload.get("streams").and_then(|v| v.as_array()) {
                    let _ = crate::entities::channel_stream::Entity::delete_many()
                        .filter(crate::entities::channel_stream::Column::ChannelId.eq(id))
                        .exec(&state.db)
                        .await;

                    let mut order = 0;
                    let mut inserts = Vec::new();
                    for stream_val in streams {
                        if let Some(stream_id) = stream_val.as_i64() {
                            inserts.push(crate::entities::channel_stream::ActiveModel {
                                channel_id: sea_orm::Set(id),
                                stream_id: sea_orm::Set(stream_id),
                                order: sea_orm::Set(order),
                                ..Default::default()
                            });
                            order += 1;
                        }
                    }
                    if !inserts.is_empty() {
                        let _ = crate::entities::channel_stream::Entity::insert_many(inserts)
                            .exec(&state.db)
                            .await;
                    }
                }
            }
        }
    }
    Ok(Json(serde_json::json!({"message": "Channels Updated Successfully"})))
}
"""

with open("src/api.rs", "a", encoding="utf-8") as f:
    f.write(code)

print("Appended API functions successfully.")
