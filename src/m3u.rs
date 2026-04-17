use crate::entities::{stream, m3u_account};
use regex::Regex;
use sea_orm::{DatabaseConnection, Set, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use std::collections::HashSet;
use std::error::Error;
use sha2::{Sha256, Digest};
use chrono::Utc;

pub async fn fetch_and_parse_m3u(
    db: &DatabaseConnection,
    url: &str,
    account_id: i64,
) -> Result<(), Box<dyn Error>> {
    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut active: m3u_account::ActiveModel = acc.into();
        active.status = Set("fetching".to_string());
        active.last_message = Set(Some("Downloading & parsing M3U...".to_string()));
        let _ = active.update(db).await;
    }

    println!("Fetching M3U from {}", url);
    let client = reqwest::Client::builder()
        .user_agent("Dispatcharr/1.0")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let body = client.get(url).send().await?.text().await?;

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

    if let Ok(Some(acc)) = m3u_account::Entity::find_by_id(account_id).one(db).await {
        let mut active: m3u_account::ActiveModel = acc.into();
        active.status = Set("success".to_string());
        active.last_message = Set(Some("Successfully synced!".to_string()));
        active.updated_at = Set(Some(Utc::now().into()));
        let _ = active.update(db).await;
    }

    println!("M3U Parsing Complete for M3U Account {}", account_id);
    Ok(())
}
