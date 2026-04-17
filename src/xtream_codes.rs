use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error::Error;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct XcCategory {
    #[serde(default)]
    pub category_id: String,
    #[serde(default)]
    pub category_name: String,
    #[serde(default)]
    pub parent_id: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct XcStream {
    pub num: Option<serde_json::Value>,
    pub name: String,
    pub stream_type: Option<String>,
    pub stream_id: i32,
    pub stream_icon: Option<String>,
    pub epg_channel_id: Option<String>,
    pub added: Option<String>,
    pub category_id: String,
    pub custom_sid: Option<String>,
    pub tv_archive: Option<i32>,
    pub direct_source: Option<String>,
    pub tv_archive_duration: Option<i32>,
}

pub async fn get_live_categories(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcCategory>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    
    let res = client.get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_live_categories"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let categories: Vec<XcCategory> = res.json().await?;
    Ok(categories)
}

pub async fn get_live_streams(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcStream>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    
    let res = client.get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_live_streams"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let streams: Vec<XcStream> = res.json().await?;
    Ok(streams)
}
