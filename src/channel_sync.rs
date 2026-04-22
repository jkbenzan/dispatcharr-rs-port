use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use crate::entities::{channel, stream, channel_group_m3u_account, channel_stream};
use std::error::Error;
use uuid::Uuid;
use chrono::Utc;

pub async fn sync_channels_for_account(
    db: &DatabaseConnection,
    account_id: i64,
) -> Result<(), Box<dyn Error>> {
    println!("[Channel Sync] Starting for account {}", account_id);
    
    let mappings = channel_group_m3u_account::Entity::find()
        .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
        .filter(channel_group_m3u_account::Column::AutoChannelSync.eq(true))
        .filter(channel_group_m3u_account::Column::Enabled.eq(true))
        .all(db)
        .await?;
        
    let enabled_group_ids: Vec<i64> = mappings.into_iter().map(|m| m.channel_group_id as i64).collect();
    
    if enabled_group_ids.is_empty() {
        println!("[Channel Sync] No auto-sync groups enabled for account {}", account_id);
        return Ok(());
    }

    let streams = stream::Entity::find()
        .filter(stream::Column::M3uAccountId.eq(account_id))
        .filter(stream::Column::ChannelGroupId.is_in(enabled_group_ids.clone()))
        .all(db)
        .await?;
        
    println!("[Channel Sync] Found {} streams to sync", streams.len());

    let stream_ids: Vec<i64> = streams.iter().map(|s| s.id).collect();

    let existing_mappings = channel_stream::Entity::find()
        .filter(channel_stream::Column::StreamId.is_in(stream_ids))
        .all(db)
        .await?;

    let existing_stream_ids: std::collections::HashSet<i64> = existing_mappings
        .into_iter()
        .map(|m| m.stream_id)
        .collect();

    for stream in streams {
        if !existing_stream_ids.contains(&stream.id) {
            let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
            
            let new_channel = channel::ActiveModel {
                channel_number: Set(0.0),
                name: Set(stream.name.clone()),
                tvg_id: Set(stream.tvg_id.clone()),
                channel_group_id: Set(stream.channel_group_id),
                uuid: Set(Uuid::new_v4()),
                auto_created: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
                is_adult: Set(false),
                user_level: Set(0),
                ..Default::default()
            };
            
            match new_channel.insert(db).await {
                Ok(inserted_channel) => {
                    let new_mapping = channel_stream::ActiveModel {
                        channel_id: Set(inserted_channel.id),
                        stream_id: Set(stream.id),
                        order: Set(0),
                        ..Default::default()
                    };
                    let _ = new_mapping.insert(db).await;
                }
                Err(e) => {
                    eprintln!("[Channel Sync] Error inserting channel for stream {}: {}", stream.name, e);
                }
            }
        }
    }
    
    println!("[Channel Sync] Completed for account {}", account_id);
    Ok(())
}
