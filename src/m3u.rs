use crate::entities::stream;
use regex::Regex;
use sea_orm::{DatabaseConnection, Set, EntityTrait, QueryFilter, ColumnTrait};
use std::collections::HashSet;
use std::error::Error;
use sha2::{Sha256, Digest};
use chrono::Utc;

pub async fn fetch_and_parse_m3u(
    db: &DatabaseConnection,
    url: &str,
    account_id: i64,
) -> Result<(), Box<dyn Error>> {
    println!("Fetching M3U from {}", url);
    let body = reqwest::get(url).await?.text().await?;

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

    let extinf_re = Regex::new(r#"#EXTINF:[^\s]+(?:\s+tvg-id="([^"]*)")?(?:\s+tvg-logo="([^"]*)")?(?:\s+group-title="([^"]*)")?,(.+)"#)?;
    let mut current_extinf: Option<stream::ActiveModel> = None;
    let mut streams_batch = vec![];

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        if line.starts_with("#EXTINF") {
            if let Some(caps) = extinf_re.captures(line) {
                let name = caps.get(4).map_or("Unknown", |m| m.as_str()).to_string();
                let tvg_id = caps.get(1).map(|m| m.as_str().to_string());
                let logo_url = caps.get(2).map(|m| m.as_str().to_string());
                
                current_extinf = Some(stream::ActiveModel {
                    name: Set(name),
                    tvg_id: Set(tvg_id),
                    logo_url: Set(logo_url),
                    m3u_account_id: Set(Some(account_id)),
                    is_custom: Set(false),
                    current_viewers: Set(0),
                    updated_at: Set(Utc::now().into()),
                    last_seen: Set(Utc::now().into()),
                    ..Default::default()
                });
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

    println!("M3U Parsing Complete for M3U Account {}", account_id);
    Ok(())
}
