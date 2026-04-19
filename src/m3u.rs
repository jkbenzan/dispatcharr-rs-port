use crate::entities::{
    stream, m3u_account, channel_group, channel_group_m3u_account,
    vod_category, vod_movie, vod_series,
    vod_m3uvodcategoryrelation, vod_m3umovierelation, vod_m3useriesrelation,
    core_settings, core_useragent
};
use regex::Regex;
use sea_orm::{DatabaseConnection, Set, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use std::collections::HashSet;
use std::error::Error;
use sha2::{Sha256, Digest};
use chrono::Utc;
use uuid::Uuid;
use tokio::sync::broadcast::Sender;
use serde_json::Value;

pub fn broadcast_progress(
    ws_sender: &Option<Sender<Value>>,
    account_id: i64,
    status: &str,
    action: &str,
    progress: i32,
    message: &str,
) {
    if let Some(sender) = ws_sender {
        let payload = serde_json::json!({
            "type": "m3u_refresh",
            "account": account_id,
            "status": status,
            "action": action,
            "progress": progress,
            "message": message,
        });
        let _ = sender.send(payload);
    }
}

pub async fn get_user_agent_string(db: &DatabaseConnection, account_user_agent_id: Option<i64>) -> String {
    let mut ua_id = account_user_agent_id;
    
    if ua_id.is_none() {
        if let Ok(Some(setting)) = core_settings::Entity::find()
            .filter(core_settings::Column::Key.eq("stream_settings"))
            .one(db).await {
            if let Some(default_id) = setting.value.get("default_user_agent").and_then(|v| v.as_i64()) {
                ua_id = Some(default_id);
            }
        }
    }

    if let Some(id) = ua_id {
        if let Ok(Some(ua)) = core_useragent::Entity::find_by_id(id).one(db).await {
            return ua.user_agent;
        }
    }

    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36".to_string()
}

pub async fn fetch_and_parse_m3u(
    db: &DatabaseConnection,
    url: &str,
    account_id: i64,
    is_initial: bool,
    ws_sender: Option<Sender<Value>>,
) -> Result<(), Box<dyn Error>> {
    let mut ua_id = None;
    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        ua_id = acc.user_agent_id;
        let mut active: m3u_account::ActiveModel = acc.into();
        active.status = Set("fetching".to_string());
        active.last_message = Set(Some("Downloading & parsing M3U...".to_string()));
        let _ = active.update(db).await;
        broadcast_progress(&ws_sender, account_id, "fetching", "downloading", 10, "Downloading M3U...");
    }

    println!("Fetching M3U from {}", url);
    let client = reqwest::Client::builder()
        .user_agent(get_user_agent_string(db, ua_id).await)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let body = client.get(url).send().await?.error_for_status()?.text().await?;

    let existing_records = stream::Entity::find()
        .filter(stream::Column::M3uAccountId.eq(account_id))
        .all(db)
        .await
        .unwrap_or_default();
        
    let mut hash_set: HashSet<String> = HashSet::new();
    for rec in existing_records {
        if let Some(h) = rec.stream_hash {
            hash_set.insert(h);
        }
    }

    let attr_re = Regex::new(r#"([a-zA-Z0-9_-]+)="([^"]*)""#)?;
    let mut current_extinf: Option<stream::ActiveModel> = None;
    let mut streams_batch = vec![];
    let mut unique_groups: HashSet<String> = HashSet::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        if line.starts_with("#EXTINF") {
            let mut name = "Unknown".to_string();
            let mut tvg_id = None;
            let mut logo_url = None;
            let mut group_title = None;
            
            if let Some((attrs_str, name_str)) = line.split_once(',') {
                name = name_str.trim().to_string();
                for cap in attr_re.captures_iter(attrs_str) {
                    if let (Some(key), Some(val)) = (cap.get(1), cap.get(2)) {
                        match key.as_str() {
                            "tvg-id" => tvg_id = Some(val.as_str().to_string()),
                            "tvg-logo" => logo_url = Some(val.as_str().to_string()),
                            "group-title" => group_title = Some(val.as_str().to_string()),
                            _ => {}
                        }
                    }
                }
            }

            let mut cp = serde_json::Map::new();
            if let Some(gt) = group_title {
                if !gt.is_empty() {
                    unique_groups.insert(gt.clone());
                }
                cp.insert("group_title".to_string(), serde_json::Value::String(gt));
            }
            
            current_extinf = Some(stream::ActiveModel {
                name: Set(name),
                tvg_id: Set(tvg_id),
                logo_url: Set(logo_url),
                custom_properties: Set(Some(serde_json::Value::Object(cp))),
                m3u_account_id: Set(Some(account_id)),
                is_custom: Set(false),
                current_viewers: Set(0),
                updated_at: Set(Utc::now().into()),
                last_seen: Set(Utc::now().into()),
                ..Default::default()
            });
        } else if line.starts_with("#EXTGRP:") {
            if let Some(mut stream_model) = current_extinf.take() {
                let group_title = line.trim_start_matches("#EXTGRP:").trim().to_string();
                
                let mut cp = match stream_model.custom_properties.take() {
                    Some(Some(serde_json::Value::Object(map))) => map,
                    _ => serde_json::Map::new(),
                };
                if !group_title.is_empty() {
                    unique_groups.insert(group_title.clone());
                }
                cp.insert("group_title".to_string(), serde_json::Value::String(group_title));
                stream_model.custom_properties = Set(Some(serde_json::Value::Object(cp)));
                
                current_extinf = Some(stream_model);
            }
        } else if !line.starts_with('#') {
            if let Some(mut stream_model) = current_extinf.take() {
                stream_model.url = Set(Some(line.to_string()));
                
                let mut hasher = Sha256::new();
                hasher.update(line.as_bytes());
                hasher.update(&account_id.to_be_bytes());
                let result = hex::encode(hasher.finalize());
                
                if !hash_set.contains(&result) {
                    stream_model.stream_hash = Set(Some(result.clone()));
                    hash_set.insert(result);
                    streams_batch.push(stream_model);

                    if streams_batch.len() >= 500 {
                        let chunk = std::mem::take(&mut streams_batch);
                        let _ = stream::Entity::insert_many(chunk).exec(db).await;
                    }
                }
            }
        }
    }

    if !streams_batch.is_empty() {
        let _ = stream::Entity::insert_many(streams_batch).exec(db).await;
    }

    for group_name in unique_groups {
        let cg = match channel_group::Entity::find()
            .filter(channel_group::Column::Name.eq(&group_name))
            .one(db).await.unwrap_or(None) {
            Some(g) => g,
            None => {
                let new_cg = channel_group::ActiveModel {
                    name: Set(group_name.clone()),
                    ..Default::default()
                };
                if let Ok(res) = channel_group::Entity::insert(new_cg).exec(db).await {
                    channel_group::Model { id: res.last_insert_id, name: group_name.clone() }
                } else {
                    continue;
                }
            }
        };

        let existing_mapping = channel_group_m3u_account::Entity::find()
            .filter(channel_group_m3u_account::Column::ChannelGroupId.eq(cg.id))
            .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
            .one(db).await.unwrap_or(None);

        if existing_mapping.is_none() {
            let new_mapping = channel_group_m3u_account::ActiveModel {
                enabled: Set(false),
                channel_group_id: Set(cg.id),
                m3u_account_id: Set(account_id),
                auto_channel_sync: Set(false),
                is_stale: Set(false),
                last_seen: Set(Utc::now().into()),
                ..Default::default()
            };
            let _ = channel_group_m3u_account::Entity::insert(new_mapping).exec(db).await;
        }
    }

    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut active: m3u_account::ActiveModel = acc.into();
        if is_initial {
            active.status = Set("pending_setup".to_string());
            active.last_message = Set(Some("M3U groups loaded. Please select groups to complete setup.".to_string()));
            let _ = active.clone().update(db).await;
            broadcast_progress(&ws_sender, account_id, "pending_setup", "processing_groups", 100, "M3U groups loaded. Please select groups to complete setup.");
        } else {
            let _ = crate::channel_sync::sync_channels_for_account(db, account_id).await;
            active.status = Set("success".to_string());
            active.last_message = Set(Some("Successfully synced!".to_string()));
            let _ = active.clone().update(db).await;
            broadcast_progress(&ws_sender, account_id, "success", "completed", 100, "Successfully synced!");
        }
        active.updated_at = Set(Some(Utc::now().into()));
        let _ = active.update(db).await;
    }

    println!("M3U Parsing Complete for M3U Account {}", account_id);
    Ok(())
}
pub async fn fetch_and_parse_xc(
    db: &DatabaseConnection,
    account_id: i64,
    ws_sender: Option<Sender<Value>>,
) -> Result<(), Box<dyn Error>> {
    let acc = match m3u_account::Entity::find_by_id(account_id).one(db).await {
        Ok(Some(a)) => a,
        _ => return Err("Account not found".into()),
    };

    let mut active: m3u_account::ActiveModel = acc.clone().into();
    active.status = Set("fetching".to_string());
    active.last_message = Set(Some("Fetching XC API categories...".to_string()));
    let _ = active.update(db).await;
    broadcast_progress(&ws_sender, account_id, "fetching", "downloading", 10, "Fetching XC API categories...");

    let mut server_url_raw = acc.server_url.clone().unwrap_or_default();
    server_url_raw = server_url_raw.trim_end_matches('/').to_string();
    
    // Remove any path after domain
    let server_url = if let Some(idx) = server_url_raw.find("://") {
        let protocol = &server_url_raw[..idx];
        let rest = &server_url_raw[idx + 3..];
        let domain = rest.split('/').next().unwrap_or(rest);
        format!("{}://{}", protocol, domain)
    } else {
        let domain = server_url_raw.split('/').next().unwrap_or(&server_url_raw);
        format!("http://{}", domain)
    };
    let username = acc.username.clone().unwrap_or_default();
    let password = acc.password.clone().unwrap_or_default();

    eprintln!("[XC] Connecting to server: {}", server_url);

    let client = reqwest::Client::builder()
        .user_agent(get_user_agent_string(db, acc.user_agent_id).await)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let categories = crate::xtream_codes::get_live_categories(&client, &server_url, &username, &password).await?;
    
    let mut active2: m3u_account::ActiveModel = acc.clone().into();
    active2.last_message = Set(Some("Fetching XC API streams...".to_string()));
    let _ = active2.update(db).await;
    
    let xc_streams = crate::xtream_codes::get_live_streams(&client, &server_url, &username, &password).await?;

    let mut streams_batch = Vec::new();
    let mut unique_groups = HashSet::new();
    let mut hash_set = HashSet::new();

    // XC API categories mapping
    let mut category_map = std::collections::HashMap::new();
    for cat in categories {
        category_map.insert(cat.category_id.clone(), cat.category_name.clone());
        unique_groups.insert(cat.category_name);
    }

    for s in xc_streams {
        let group_title = category_map.get(&s.category_id).cloned().unwrap_or_else(|| "Unknown Category".to_string());
        
        let url = format!("{}/live/{}/{}/{}.ts", server_url.trim_end_matches('/'), username, password, s.stream_id);
        
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hasher.update(&account_id.to_be_bytes());
        let result = hex::encode(hasher.finalize());

        if !hash_set.contains(&result) {
            hash_set.insert(result.clone());

            let mut cp = serde_json::Map::new();
            cp.insert("group_title".to_string(), serde_json::Value::String(group_title.clone()));
            if let Some(num) = s.num {
                cp.insert("channel-number".to_string(), num);
            }

            let stream_model = stream::ActiveModel {
                m3u_account_id: Set(Some(account_id)),
                name: Set(s.name),
                url: Set(Some(url)),
                logo_url: Set(s.stream_icon),
                tvg_id: Set(s.epg_channel_id),
                is_custom: Set(false),
                last_seen: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                stream_hash: Set(Some(result)),
                custom_properties: Set(Some(serde_json::Value::Object(cp))),
                ..Default::default()
            };
            streams_batch.push(stream_model);

            if streams_batch.len() >= 500 {
                let chunk = std::mem::take(&mut streams_batch);
                let _ = stream::Entity::insert_many(chunk).exec(db).await;
            }
        }
    }

    if !streams_batch.is_empty() {
        let _ = stream::Entity::insert_many(streams_batch).exec(db).await;
    }

    // Insert new ChannelGroups and mappings
    for group_name in unique_groups {
        let cg = match channel_group::Entity::find()
            .filter(channel_group::Column::Name.eq(&group_name))
            .one(db).await.unwrap_or(None) {
            Some(g) => g,
            None => {
                let new_cg = channel_group::ActiveModel {
                    name: Set(group_name.clone()),
                    ..Default::default()
                };
                if let Ok(res) = channel_group::Entity::insert(new_cg).exec(db).await {
                    channel_group::Model { id: res.last_insert_id, name: group_name.clone() }
                } else {
                    continue;
                }
            }
        };

        let existing_mapping = channel_group_m3u_account::Entity::find()
            .filter(channel_group_m3u_account::Column::ChannelGroupId.eq(cg.id))
            .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
            .one(db).await.unwrap_or(None);

        if existing_mapping.is_none() {
            let new_mapping = channel_group_m3u_account::ActiveModel {
                enabled: Set(false),
                channel_group_id: Set(cg.id),
                m3u_account_id: Set(account_id),
                auto_channel_sync: Set(false),
                is_stale: Set(false),
                last_seen: Set(Utc::now().into()),
                ..Default::default()
            };
            let _ = channel_group_m3u_account::Entity::insert(new_mapping).exec(db).await;
        }
    }

    let _ = crate::channel_sync::sync_channels_for_account(db, account_id).await;
    let mut final_active: m3u_account::ActiveModel = acc.into();
    final_active.status = Set("success".to_string());
    final_active.last_message = Set(Some("Groups mapped successfully".to_string()));
    let _ = final_active.update(db).await;
    broadcast_progress(&ws_sender, account_id, "success", "completed", 100, "Groups mapped successfully");

    Ok(())
}


pub async fn fetch_and_parse_xc_vod(
    db: &DatabaseConnection,
    account_id: i64,
) -> Result<(), Box<dyn Error>> {
    let acc = match m3u_account::Entity::find_by_id(account_id).one(db).await {
        Ok(Some(a)) => a,
        _ => return Err("Account not found".into()),
    };

    let mut server_url_raw = acc.server_url.clone().unwrap_or_default();
    server_url_raw = server_url_raw.trim_end_matches('/').to_string();
    let server_url = if let Some(idx) = server_url_raw.find("://") {
        let protocol = &server_url_raw[..idx];
        let rest = &server_url_raw[idx + 3..];
        let domain = rest.split('/').next().unwrap_or(rest);
        format!("{}://{}", protocol, domain)
    } else {
        let domain = server_url_raw.split('/').next().unwrap_or(&server_url_raw);
        format!("http://{}", domain)
    };
    let username = acc.username.clone().unwrap_or_default();
    let password = acc.password.clone().unwrap_or_default();

    let client = reqwest::Client::builder()
        .user_agent(get_user_agent_string(db, acc.user_agent_id).await)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // 1. Fetch VOD Categories
    let mut active: m3u_account::ActiveModel = acc.clone().into();
    active.last_message = Set(Some("Fetching XC VOD categories...".to_string()));
    let _ = active.update(db).await;

    if let Some(vod_cats) = crate::xtream_codes::get_vod_categories(&client, &server_url, &username, &password).await.ok() {
        for cat in vod_cats {
            let vc = match vod_category::Entity::find()
                .filter(vod_category::Column::Name.eq(&cat.category_name))
                .filter(vod_category::Column::CategoryType.eq("movie"))
                .one(db).await.unwrap_or(None) {
                Some(c) => c,
                None => {
                    let new_vc = vod_category::ActiveModel {
                        name: Set(cat.category_name.clone()),
                        category_type: Set("movie".to_string()),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    if let Ok(res) = vod_category::Entity::insert(new_vc).exec(db).await {
                        vod_category::Model {
                            id: res.last_insert_id,
                            name: cat.category_name.clone(),
                            category_type: "movie".to_string(),
                            created_at: Utc::now().into(),
                            updated_at: Utc::now().into(),
                        }
                    } else {
                        continue;
                    }
                }
            };

            let relation = vod_m3uvodcategoryrelation::Entity::find()
                .filter(vod_m3uvodcategoryrelation::Column::CategoryId.eq(vc.id))
                .filter(vod_m3uvodcategoryrelation::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);
            if relation.is_none() {
                let new_rel = vod_m3uvodcategoryrelation::ActiveModel {
                    enabled: Set(true),
                    m3u_account_id: Set(account_id),
                    category_id: Set(vc.id),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                let _ = vod_m3uvodcategoryrelation::Entity::insert(new_rel).exec(db).await;
            }
        }
    }

    // 2. Fetch VOD Streams
    let mut active: m3u_account::ActiveModel = acc.clone().into();
    active.last_message = Set(Some("Fetching XC VOD streams...".to_string()));
    let _ = active.update(db).await;

    if let Some(vod_streams) = crate::xtream_codes::get_vod_streams(&client, &server_url, &username, &password).await.ok() {
        for s in vod_streams.into_iter() {
            let rel = vod_m3umovierelation::Entity::find()
                .filter(vod_m3umovierelation::Column::StreamId.eq(s.stream_id.to_string()))
                .filter(vod_m3umovierelation::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);
            
            if rel.is_none() {
                let new_movie = vod_movie::ActiveModel {
                    uuid: Set(Uuid::new_v4()),
                    name: Set(s.name.clone()),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                if let Ok(res) = vod_movie::Entity::insert(new_movie).exec(db).await {
                    let new_rel = vod_m3umovierelation::ActiveModel {
                        stream_id: Set(s.stream_id.to_string()),
                        container_extension: Set(s.container_extension.clone()),
                        m3u_account_id: Set(account_id),
                        movie_id: Set(res.last_insert_id),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    let _ = vod_m3umovierelation::Entity::insert(new_rel).exec(db).await;
                }
            }
        }
    }

    // Update Status
    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut final_active: m3u_account::ActiveModel = acc.into();
        final_active.status = Set("success".to_string());
        final_active.last_message = Set(Some("Successfully synced VOD!".to_string()));
        final_active.updated_at = Set(Some(Utc::now().into()));
        let _ = final_active.update(db).await;
    }

    Ok(())
}

pub async fn fetch_and_parse_xc_series(
    db: &DatabaseConnection,
    account_id: i64,
) -> Result<(), Box<dyn Error>> {
    let acc = match m3u_account::Entity::find_by_id(account_id).one(db).await {
        Ok(Some(a)) => a,
        _ => return Err("Account not found".into()),
    };

    let mut server_url_raw = acc.server_url.clone().unwrap_or_default();
    server_url_raw = server_url_raw.trim_end_matches('/').to_string();
    let server_url = if let Some(idx) = server_url_raw.find("://") {
        let protocol = &server_url_raw[..idx];
        let rest = &server_url_raw[idx + 3..];
        let domain = rest.split('/').next().unwrap_or(rest);
        format!("{}://{}", protocol, domain)
    } else {
        let domain = server_url_raw.split('/').next().unwrap_or(&server_url_raw);
        format!("http://{}", domain)
    };
    let username = acc.username.clone().unwrap_or_default();
    let password = acc.password.clone().unwrap_or_default();

    let client = reqwest::Client::builder()
        .user_agent(get_user_agent_string(db, acc.user_agent_id).await)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut active: m3u_account::ActiveModel = acc.clone().into();
    active.last_message = Set(Some("Fetching XC Series categories...".to_string()));
    let _ = active.update(db).await;

    if let Some(series_cats) = crate::xtream_codes::get_series_categories(&client, &server_url, &username, &password).await.ok() {
        for cat in series_cats {
            let vc = match vod_category::Entity::find()
                .filter(vod_category::Column::Name.eq(&cat.category_name))
                .filter(vod_category::Column::CategoryType.eq("series"))
                .one(db).await.unwrap_or(None) {
                Some(c) => c,
                None => {
                    let new_vc = vod_category::ActiveModel {
                        name: Set(cat.category_name.clone()),
                        category_type: Set("series".to_string()),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    if let Ok(res) = vod_category::Entity::insert(new_vc).exec(db).await {
                        vod_category::Model {
                            id: res.last_insert_id,
                            name: cat.category_name.clone(),
                            category_type: "series".to_string(),
                            created_at: Utc::now().into(),
                            updated_at: Utc::now().into(),
                        }
                    } else {
                        continue;
                    }
                }
            };

            let relation = vod_m3uvodcategoryrelation::Entity::find()
                .filter(vod_m3uvodcategoryrelation::Column::CategoryId.eq(vc.id))
                .filter(vod_m3uvodcategoryrelation::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);
            if relation.is_none() {
                let new_rel = vod_m3uvodcategoryrelation::ActiveModel {
                    enabled: Set(true),
                    m3u_account_id: Set(account_id),
                    category_id: Set(vc.id),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                let _ = vod_m3uvodcategoryrelation::Entity::insert(new_rel).exec(db).await;
            }
        }
    }

    let mut active: m3u_account::ActiveModel = acc.clone().into();
    active.last_message = Set(Some("Fetching XC Series...".to_string()));
    let _ = active.update(db).await;

    if let Some(series_list) = crate::xtream_codes::get_series(&client, &server_url, &username, &password).await.ok() {
        for s in series_list.into_iter() {
            let rel = vod_m3useriesrelation::Entity::find()
                .filter(vod_m3useriesrelation::Column::ExternalSeriesId.eq(s.series_id.to_string()))
                .filter(vod_m3useriesrelation::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);
            
            if rel.is_none() {
                let new_series = vod_series::ActiveModel {
                    uuid: Set(Uuid::new_v4()),
                    name: Set(s.name.clone()),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                if let Ok(res) = vod_series::Entity::insert(new_series).exec(db).await {
                    let new_rel = vod_m3useriesrelation::ActiveModel {
                        external_series_id: Set(s.series_id.to_string()),
                        m3u_account_id: Set(account_id),
                        series_id: Set(res.last_insert_id),
                        last_seen: Set(Utc::now().into()),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    let _ = vod_m3useriesrelation::Entity::insert(new_rel).exec(db).await;
                }
            }
        }
    }

    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut final_active: m3u_account::ActiveModel = acc.into();
        final_active.status = Set("success".to_string());
        final_active.last_message = Set(Some("Successfully synced Series!".to_string()));
        final_active.updated_at = Set(Some(Utc::now().into()));
        let _ = final_active.update(db).await;
    }

    Ok(())
}

pub async fn fetch_and_parse_xc_categories(
    db: &DatabaseConnection,
    account_id: i64,
    ws_sender: Option<Sender<Value>>,
) -> Result<(), Box<dyn Error>> {
    let acc = match m3u_account::Entity::find_by_id(account_id).one(db).await {
        Ok(Some(a)) => a,
        _ => return Err("Account not found".into()),
    };

    let mut server_url_raw = acc.server_url.clone().unwrap_or_default();
    server_url_raw = server_url_raw.trim_end_matches('/').to_string();
    
    let server_url = if let Some(idx) = server_url_raw.find("://") {
        let protocol = &server_url_raw[..idx];
        let rest = &server_url_raw[idx + 3..];
        let domain = rest.split('/').next().unwrap_or(rest);
        format!("{}://{}", protocol, domain)
    } else {
        let domain = server_url_raw.split('/').next().unwrap_or(&server_url_raw);
        format!("http://{}", domain)
    };
    let username = acc.username.clone().unwrap_or_default();
    let password = acc.password.clone().unwrap_or_default();

    let client = reqwest::Client::builder()
        .user_agent(get_user_agent_string(db, acc.user_agent_id).await)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // Fetch Live Categories
    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut active: m3u_account::ActiveModel = acc.into();
        active.status = Set("fetching".to_string());
        active.last_message = Set(Some("Fetching XC Live categories...".to_string()));
        let _ = active.update(db).await;
    }

    let live_categories = match crate::xtream_codes::get_live_categories(&client, &server_url, &username, &password).await {
        Ok(c) => c,
        Err(_) => vec![],
    };

    for cat in live_categories {
            let group_name = cat.category_name;
            let cg = match channel_group::Entity::find()
                .filter(channel_group::Column::Name.eq(&group_name))
                .one(db).await.unwrap_or(None) {
                Some(g) => g,
                None => {
                    let new_cg = channel_group::ActiveModel {
                        name: Set(group_name.clone()),
                        ..Default::default()
                    };
                    if let Ok(res) = channel_group::Entity::insert(new_cg).exec(db).await {
                        channel_group::Model { id: res.last_insert_id, name: group_name.clone() }
                    } else {
                        continue;
                    }
                }
            };

            let existing_mapping = channel_group_m3u_account::Entity::find()
                .filter(channel_group_m3u_account::Column::ChannelGroupId.eq(cg.id))
                .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);

            if existing_mapping.is_none() {
                let new_mapping = channel_group_m3u_account::ActiveModel {
                    enabled: Set(false),
                    channel_group_id: Set(cg.id),
                    m3u_account_id: Set(account_id),
                    auto_channel_sync: Set(false),
                    is_stale: Set(false),
                    last_seen: Set(Utc::now().into()),
                    ..Default::default()
                };
                let _ = channel_group_m3u_account::Entity::insert(new_mapping).exec(db).await;
            }
        }
    // Fetch VOD Categories
    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut active: m3u_account::ActiveModel = acc.into();
        active.last_message = Set(Some("Fetching XC VOD categories...".to_string()));
        let _ = active.update(db).await;
    }

    let vod_categories = match crate::xtream_codes::get_vod_categories(&client, &server_url, &username, &password).await {
        Ok(c) => c,
        Err(_) => vec![],
    };

    for cat in vod_categories {
            let vc = match vod_category::Entity::find()
                .filter(vod_category::Column::Name.eq(&cat.category_name))
                .filter(vod_category::Column::CategoryType.eq("movie"))
                .one(db).await.unwrap_or(None) {
                Some(c) => c,
                None => {
                    let new_vc = vod_category::ActiveModel {
                        name: Set(cat.category_name.clone()),
                        category_type: Set("movie".to_string()),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    if let Ok(res) = vod_category::Entity::insert(new_vc).exec(db).await {
                        vod_category::Model {
                            id: res.last_insert_id,
                            name: cat.category_name.clone(),
                            category_type: "movie".to_string(),
                            created_at: Utc::now().into(),
                            updated_at: Utc::now().into(),
                        }
                    } else {
                        continue;
                    }
                }
            };

            let relation = vod_m3uvodcategoryrelation::Entity::find()
                .filter(vod_m3uvodcategoryrelation::Column::CategoryId.eq(vc.id))
                .filter(vod_m3uvodcategoryrelation::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);
            if relation.is_none() {
                let new_rel = vod_m3uvodcategoryrelation::ActiveModel {
                    enabled: Set(false),
                    m3u_account_id: Set(account_id),
                    category_id: Set(vc.id),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                let _ = vod_m3uvodcategoryrelation::Entity::insert(new_rel).exec(db).await;
            }
        }


    // Fetch Series Categories
    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut active: m3u_account::ActiveModel = acc.into();
        active.last_message = Set(Some("Fetching XC Series categories...".to_string()));
        let _ = active.update(db).await;
    }

    let series_categories = match crate::xtream_codes::get_series_categories(&client, &server_url, &username, &password).await {
        Ok(c) => c,
        Err(_) => vec![],
    };

    for cat in series_categories {
            let vc = match vod_category::Entity::find()
                .filter(vod_category::Column::Name.eq(&cat.category_name))
                .filter(vod_category::Column::CategoryType.eq("series"))
                .one(db).await.unwrap_or(None) {
                Some(c) => c,
                None => {
                    let new_vc = vod_category::ActiveModel {
                        name: Set(cat.category_name.clone()),
                        category_type: Set("series".to_string()),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    if let Ok(res) = vod_category::Entity::insert(new_vc).exec(db).await {
                        vod_category::Model {
                            id: res.last_insert_id,
                            name: cat.category_name.clone(),
                            category_type: "series".to_string(),
                            created_at: Utc::now().into(),
                            updated_at: Utc::now().into(),
                        }
                    } else {
                        continue;
                    }
                }
            };

            let relation = vod_m3uvodcategoryrelation::Entity::find()
                .filter(vod_m3uvodcategoryrelation::Column::CategoryId.eq(vc.id))
                .filter(vod_m3uvodcategoryrelation::Column::M3uAccountId.eq(account_id))
                .one(db).await.unwrap_or(None);
            if relation.is_none() {
                let new_rel = vod_m3uvodcategoryrelation::ActiveModel {
                    enabled: Set(false),
                    m3u_account_id: Set(account_id),
                    category_id: Set(vc.id),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };
                let _ = vod_m3uvodcategoryrelation::Entity::insert(new_rel).exec(db).await;
            }
        }


    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut final_active: m3u_account::ActiveModel = acc.into();
        final_active.status = Set("pending_setup".to_string());
        final_active.last_message = Set(Some("Groups loaded. Please select groups to complete setup.".to_string()));
        final_active.updated_at = Set(Some(Utc::now().into()));
        let _ = final_active.update(db).await;
        broadcast_progress(&ws_sender, account_id, "pending_setup", "processing_groups", 100, "Groups loaded. Please select groups to complete setup.");
    }

    Ok(())
}
