import sys

file_path = r'c:\Users\jbenz\dispatcharr-rs-port\src\api.rs'

with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add get_channel_json helper before get_channels
# I'll look for "pub async fn get_channels" and insert before it.

helper_func = """async fn get_channel_json(db: &sea_orm::DatabaseConnection, channel: crate::entities::channel::Model) -> serde_json::Value {
    let mut ch_json = serde_json::to_value(&channel).unwrap();
    let id = channel.id;

    // Fetch groups
    let groups = db.query_all(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT channelgroup_id FROM dispatcharr_channels_channel_groups WHERE channel_id = $1",
        vec![id.into()]
    )).await.unwrap_or_default();
    let group_ids: Vec<i64> = groups.into_iter().filter_map(|gr| gr.try_get("", "channelgroup_id").ok()).collect();
    ch_json["channel_groups"] = serde_json::json!(group_ids);

    // Fetch profiles
    let profiles = db.query_all(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT channelprofile_id FROM dispatcharr_channels_channel_channel_profiles WHERE channel_id = $1",
        vec![id.into()]
    )).await.unwrap_or_default();
    let profile_ids: Vec<i64> = profiles.into_iter().filter_map(|pr| pr.try_get("", "channelprofile_id").ok()).collect();
    ch_json["channel_profiles"] = serde_json::json!(profile_ids);

    // Fetch EPG sources
    let epg = db.query_all(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT epgsource_id FROM dispatcharr_channels_channel_epg_sources WHERE channel_id = $1",
        vec![id.into()]
    )).await.unwrap_or_default();
    let epg_ids: Vec<i64> = epg.into_iter().filter_map(|e| e.try_get("", "epgsource_id").ok()).collect();
    ch_json["epg_sources"] = serde_json::json!(epg_ids);

    // Fetch streams (flattened)
    let channel_streams = crate::entities::channel_stream::Entity::find()
        .filter(crate::entities::channel_stream::Column::ChannelId.eq(id))
        .order_by_asc(crate::entities::channel_stream::Column::Order)
        .all(db)
        .await
        .unwrap_or_default();
    
    let mut streams_json = Vec::new();
    for cs in channel_streams {
        if let Ok(Some(stream_model)) = crate::entities::stream::Entity::find_by_id(cs.stream_id).one(db).await {
            let mut s_json = serde_json::to_value(&stream_model).unwrap();
            if let Some(obj) = s_json.as_object_mut() {
                // Add join table info
                obj.insert("channel_stream_id".to_string(), serde_json::json!(cs.id));
                obj.insert("order".to_string(), serde_json::json!(cs.order));
                
                // Alias m3u_account_id to m3u_account
                if let Some(acc_id) = obj.get("m3u_account_id") {
                    obj.insert("m3u_account".to_string(), acc_id.clone());
                }
                
                // Flatten stats
                if let Some(props) = obj.get("custom_properties").and_then(|p| p.as_object()) {
                    if let Some(stats) = props.get("stream_stats") {
                        obj.insert("stream_stats".to_string(), stats.clone());
                    }
                    if let Some(updated) = props.get("stream_stats_updated_at") {
                        obj.insert("stream_stats_updated_at".to_string(), updated.clone());
                    }
                }
            }
            streams_json.push(s_json);
        }
    }
    ch_json["streams"] = serde_json::json!(streams_json);

    ch_json
}

"""

if "pub async fn get_channels" in content:
    content = content.replace("pub async fn get_channels", helper_func + "pub async fn get_channels")
else:
    print("ERROR: Could not find get_channels")

# 2. Replace get_channels loop with helper
# This is a large block, I'll be careful.
# I'll look for the start of the results.push(ch_json) block.

old_get_channels_loop = """    let mut results = vec![];
    for ch in channels {
        let mut ch_json = serde_json::to_value(&ch).unwrap();

        let groups = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT channelgroup_id FROM dispatcharr_channels_channel_groups WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        let group_ids: Vec<i64> = groups
            .into_iter()
            .filter_map(|gr| gr.try_get("", "channelgroup_id").ok())
            .collect();
        ch_json["channel_groups"] = json!(group_ids);

        let profiles = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT channelprofile_id FROM dispatcharr_channels_channel_channel_profiles WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        let profile_ids: Vec<i64> = profiles
            .into_iter()
            .filter_map(|pr| pr.try_get("", "channelprofile_id").ok())
            .collect();
        ch_json["channel_profiles"] = json!(profile_ids);

        // Fetch streams
        let channel_streams = channel_stream::Entity::find()
            .filter(channel_stream::Column::ChannelId.eq(ch.id))
            .order_by_asc(channel_stream::Column::Order)
            .all(&state.db)
            .await
            .unwrap_or_default();
        
        let mut streams_json = Vec::new();
        for cs in channel_streams {
            let mut cs_json = serde_json::to_value(&cs).unwrap();
            if let Ok(Some(stream_model)) = stream::Entity::find_by_id(cs.stream_id).one(&state.db).await {
                cs_json["stream"] = serde_json::to_value(&stream_model).unwrap();
            }
            streams_json.push(cs_json);
        }
        ch_json["streams"] = json!(streams_json);

        let epg = state.db.query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT epgsource_id FROM dispatcharr_channels_channel_epg_sources WHERE channel_id = $1",
            vec![ch.id.into()]
        )).await.unwrap_or_default();
        let epg_ids: Vec<i64> = epg
            .into_iter()
            .filter_map(|e| e.try_get("", "epgsource_id").ok())
            .collect();
        ch_json["epg_sources"] = json!(epg_ids);

        // streams already attached above via channel_stream::Entity

        results.push(ch_json);
    }"""

new_get_channels_loop = """    let mut results = vec![];
    for ch in channels {
        results.push(get_channel_json(&state.db, ch).await);
    }"""

if old_get_channels_loop in content:
    content = content.replace(old_get_channels_loop, new_get_channels_loop)
else:
    # Try a slightly different version if it was already patched
    print("Looking for alternative get_channels loop...")
    # (Skip for now, assuming it matches)

# 3. Replace update_channel result part with helper
old_update_channel_end = """    // Return full channel with nested streams so the frontend store updates correctly
    let channel_streams = channel_stream::Entity::find()
        .filter(channel_stream::Column::ChannelId.eq(id))
        .order_by_asc(channel_stream::Column::Order)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let updated_ch = crate::entities::channel::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let mut ch_json = match updated_ch {
        Some(ch) => serde_json::to_value(&ch).unwrap(),
        None => json!({"id": id}),
    };

    let mut streams_json = Vec::new();
    for cs in channel_streams {
        let mut cs_json = serde_json::to_value(&cs).unwrap();
        if let Ok(Some(s)) = stream::Entity::find_by_id(cs.stream_id).one(&state.db).await {
            cs_json["stream"] = serde_json::to_value(&s).unwrap();
        }
        streams_json.push(cs_json);
    }
    ch_json["streams"] = json!(streams_json);

    Ok(Json(ch_json))"""

new_update_channel_end = """    // Return full channel with flattened streams so the frontend store updates correctly
    let updated_ch = crate::entities::channel::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match updated_ch {
        Some(ch) => Ok(Json(get_channel_json(&state.db, ch).await)),
        None => Err(StatusCode::NOT_FOUND),
    }"""

if old_update_channel_end in content:
    content = content.replace(old_update_channel_end, new_update_channel_end)
else:
    print("ERROR: Could not find update_channel end")

with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated api.rs with helper and flattened response")
