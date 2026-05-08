use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraktSettings {
    pub enabled: bool,
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

pub async fn get_trakt_settings(db: &sea_orm::DatabaseConnection) -> TraktSettings {
    use crate::entities::core_settings;
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

    let s = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("trakt_settings"))
        .one(db)
        .await;

    if let Ok(Some(s)) = s {
        if let Ok(settings) = serde_json::from_value(s.value) {
            return settings;
        }
    }

    TraktSettings::default()
}

pub async fn save_trakt_settings(db: &sea_orm::DatabaseConnection, settings: TraktSettings) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::entities::core_settings;
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};

    let existing = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("trakt_settings"))
        .one(db)
        .await?;

    if let Some(s) = existing {
        let mut active: core_settings::ActiveModel = s.into();
        active.value = Set(serde_json::to_value(settings)?);
        active.update(db).await?;
    } else {
        let active = core_settings::ActiveModel {
            key: Set("trakt_settings".to_string()),
            name: Set("Trakt.tv Settings".to_string()),
            value: Set(serde_json::to_value(settings)?),
            ..Default::default()
        };
        active.insert(db).await?;
    }

    Ok(())
}

// Initial skeleton for background syncing
pub async fn run_trakt_sync(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let settings = get_trakt_settings(&state.db).await;
    if !settings.enabled || settings.access_token.is_none() {
        return Ok(());
    }

    tracing::info!("🔄 Trakt.tv sync started...");
    
    // TODO: Implement watchlist and collection sync
    
    tracing::info!("✅ Trakt.tv sync completed.");
    Ok(())
}
