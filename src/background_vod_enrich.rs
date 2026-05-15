use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::AppState;
use crate::entities::{vod_movie, vod_series};
use crate::tmdb::{get_tmdb_settings, search_movie, search_tv};
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;
use sea_orm::QueryOrder;

pub async fn run_vod_enrichment_worker(state: Arc<AppState>) {
    tracing::info!("🔄 VOD Metadata Enrichment worker started.");
    
    // Create the image directory if it doesn't exist
    let image_dir = std::env::var("VOD_IMAGE_DIR").unwrap_or_else(|_| "./data/vod_images/".to_string());
    let path = PathBuf::from(&image_dir);
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(&path).await {
            tracing::error!("❌ Failed to create VOD image directory {}: {}", image_dir, e);
            return;
        }
    }

    let client = Client::new();

    loop {
        // Sleep for 2 seconds to be respectful of rate limits
        sleep(Duration::from_secs(2)).await;

        let settings = get_tmdb_settings(&state.db).await;
        if !settings.enabled || settings.api_key.trim().is_empty() {
            // Sleep longer if disabled or no key
            sleep(Duration::from_secs(60)).await;
            continue;
        }

        // Process one movie, then one series alternately
        let processed_movie = process_next_movie(&state.db, &client, &settings.api_key, &image_dir).await;
        if processed_movie {
            // if we processed a movie, sleep before processing the next item
            sleep(Duration::from_secs(2)).await;
        }

        let processed_series = process_next_series(&state.db, &client, &settings.api_key, &image_dir).await;
        if !processed_movie && !processed_series {
            // Nothing to process, sleep longer
            sleep(Duration::from_secs(30)).await;
        }
    }
}

async fn process_next_movie(
    db: &DatabaseConnection,
    client: &Client,
    api_key: &str,
    image_dir: &str,
) -> bool {
    // Find one movie where custom_properties doesn't have 'metadata_enriched: true'
    // To do this efficiently, we can use a raw query or just fetch a batch and find one in memory.
    // Given the lack of JSONB indexing in sqlite, let's just fetch the oldest updated_at movies and check.
    // But since this is Postgres, we can do a jsonb check. However, since the ORM setup is generic,
    // let's look for `tmdb_id` IS NULL as our primary indicator, or just check custom_properties.
    
    let movie = vod_movie::Entity::find()
        .filter(vod_movie::Column::TmdbId.is_null())
        .order_by_asc(vod_movie::Column::CreatedAt)
        .one(db)
        .await
        .unwrap_or_default();

    if let Some(m) = movie {
        let title = m.name.clone();
        let year = m.year;
        
        let mut active_m: vod_movie::ActiveModel = m.clone().into();
        
        match search_movie(client, api_key, &title, year).await {
            Ok(Some(result)) => {
                active_m.tmdb_id = Set(Some(result.id.to_string()));
                
                // Download poster if available
                if let Some(poster_path) = result.poster_path {
                    let local_filename = download_poster(client, &poster_path, image_dir).await;
                    if let Some(filename) = local_filename {
                        // Store filename in custom_properties
                        let mut cp = match m.custom_properties {
                            Some(serde_json::Value::Object(map)) => map.clone(),
                            _ => serde_json::Map::new(),
                        };
                        cp.insert("local_poster".to_string(), serde_json::Value::String(filename));
                        active_m.custom_properties = Set(Some(serde_json::Value::Object(cp)));
                    }
                }
            }
            Ok(None) => {
                // Not found on TMDB. Mark with a dummy ID to prevent re-querying forever.
                active_m.tmdb_id = Set(Some("-1".to_string()));
            }
            Err(e) => {
                tracing::error!("❌ TMDB Movie search error for {}: {}", title, e);
                return false;
            }
        }
        
        active_m.updated_at = Set(chrono::Utc::now().into());
        let _ = active_m.update(db).await;
        
        return true;
    }
    
    false
}

async fn process_next_series(
    db: &DatabaseConnection,
    client: &Client,
    api_key: &str,
    image_dir: &str,
) -> bool {
    let series = vod_series::Entity::find()
        .filter(vod_series::Column::TmdbId.is_null())
        .order_by_asc(vod_series::Column::CreatedAt)
        .one(db)
        .await
        .unwrap_or_default();

    if let Some(s) = series {
        let title = s.name.clone();
        let year = s.year;
        
        let mut active_s: vod_series::ActiveModel = s.clone().into();
        
        match search_tv(client, api_key, &title, year).await {
            Ok(Some(result)) => {
                active_s.tmdb_id = Set(Some(result.id.to_string()));
                
                // Download poster if available
                if let Some(poster_path) = result.poster_path {
                    let local_filename = download_poster(client, &poster_path, image_dir).await;
                    if let Some(filename) = local_filename {
                        let mut cp = match s.custom_properties {
                            Some(serde_json::Value::Object(map)) => map.clone(),
                            _ => serde_json::Map::new(),
                        };
                        cp.insert("local_poster".to_string(), serde_json::Value::String(filename));
                        active_s.custom_properties = Set(Some(serde_json::Value::Object(cp)));
                    }
                }
            }
            Ok(None) => {
                active_s.tmdb_id = Set(Some("-1".to_string()));
            }
            Err(e) => {
                tracing::error!("❌ TMDB Series search error for {}: {}", title, e);
                return false;
            }
        }
        
        active_s.updated_at = Set(chrono::Utc::now().into());
        let _ = active_s.update(db).await;
        
        return true;
    }
    
    false
}

async fn download_poster(client: &Client, poster_path: &str, image_dir: &str) -> Option<String> {
    let url = format!("https://image.tmdb.org/t/p/w500{}", poster_path);
    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(bytes) = resp.bytes().await {
                let filename = format!("{}.jpg", Uuid::new_v4());
                let mut path = PathBuf::from(image_dir);
                path.push(&filename);
                if fs::write(&path, bytes).await.is_ok() {
                    return Some(filename);
                }
            }
        }
    }
    None
}
