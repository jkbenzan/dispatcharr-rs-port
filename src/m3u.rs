use crate::entities::channel;
use regex::Regex;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::error::Error;
use uuid::Uuid;

pub async fn fetch_and_parse_m3u(
    db: &DatabaseConnection,
    url: &str,
    account_id: i64,
) -> Result<(), Box<dyn Error>> {
    println!("Fetching M3U from {}", url);
    
    // In a real scenario, use reqwest to fetch `url`. For now, we simulate the body payload.
    // let body = reqwest::get(url).await?.text().await?;
    let body = "#EXTM3U\n#EXTINF:-1 tvg-id=\"CNN\" tvg-logo=\"logo.png\" group-title=\"News\",CNN HD\nhttp://stream.url/cnn.ts";

    let extinf_re = Regex::new(r#"#EXTINF:[^\s]+(?:\s+tvg-id="([^"]*)")?(?:\s+tvg-logo="([^"]*)")?(?:\s+group-title="([^"]*)")?,(.+)"#)?;

    let mut current_extinf: Option<channel::ActiveModel> = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("#EXTINF") {
            if let Some(caps) = extinf_re.captures(line) {
                let name = caps.get(4).map_or("Unknown", |m| m.as_str()).to_string();
                let tvg_id = caps.get(1).map(|m| m.as_str().to_string());
                let logo_url = caps.get(2).map(|m| m.as_str().to_string());
                
                // We're skipping group-title linking to dispatcharr_channels_channelgroup to save space in this skeleton module,
                // but usually you would lookup GroupId.
                
                current_extinf = Some(channel::ActiveModel {
                    uuid: Set(Uuid::new_v4()),
                    name: Set(name),
                    tvg_id: Set(tvg_id),
                    channel_number: Set(0.0), // Assign programmatically or grab from tvg-chno
                    is_adult: Set(false),
                    auto_created: Set(true),
                    user_level: Set(0),
                    created_at: Set(chrono::Utc::now().into()),
                    updated_at: Set(chrono::Utc::now().into()),
                    ..Default::default() // stream_url etc requires stream mapping
                });
            }
        } else if !line.starts_with('#') {
            // It's a stream URL
            if let Some(mut model) = current_extinf.take() {
                // Here, you would typically link this URL to a Streams entity instead of the Channel
                // But this demonstrates the extraction pipeline.
                
                println!("Saving channel: {:?}", model.name);
                // model.insert(db).await?; // Commented out to prevent DB errors on empty schema
            }
        }
    }

    println!("M3U Parsing Complete");
    Ok(())
}
