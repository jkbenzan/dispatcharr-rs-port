use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, QueryOrder, QuerySelect, PaginatorTrait, ColumnTrait, QueryFilter};
use crate::entities::{core_systemevent, core_settings};
use chrono::Utc;
use serde_json::Value;

pub async fn record_event(
    db: &DatabaseConnection,
    event_type: &str,
    channel_name: Option<String>,
    details: Value,
) -> Result<(), sea_orm::DbErr> {
    // 1. Insert the event
    let new_event = core_systemevent::ActiveModel {
        event_type: Set(event_type.to_string()),
        channel_name: Set(channel_name),
        details: Set(details),
        timestamp: Set(Utc::now().into()),
        ..Default::default()
    };
    new_event.insert(db).await?;

    // 2. Enforce max_system_events limit
    // Fetch limit from settings
    let settings_opt = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("system_settings"))
        .one(db)
        .await?;

    let max_events = if let Some(settings) = settings_opt {
        settings.value.get("max_system_events")
            .and_then(|v| v.as_i64())
            .unwrap_or(100)
    } else {
        100
    };

    // Count existing events
    let count = core_systemevent::Entity::find().count(db).await?;
    if count > max_events as u64 {
        let to_delete = count - max_events as u64;
        
        // Find the IDs of the oldest events
        // Use a subquery or just fetch IDs to delete
        let oldest_ids = core_systemevent::Entity::find()
            .select_only()
            .column(core_systemevent::Column::Id)
            .order_by_asc(core_systemevent::Column::Id)
            .limit(to_delete)
            .into_tuple::<i64>()
            .all(db)
            .await?;
        
        if !oldest_ids.is_empty() {
             core_systemevent::Entity::delete_many()
                .filter(core_systemevent::Column::Id.is_in(oldest_ids))
                .exec(db)
                .await?;
        }
    }

    Ok(())
}
