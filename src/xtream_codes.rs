use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

// Helper functions to handle XC API type inconsistencies (strings vs numbers)
fn get_string(val: &serde_json::Value, key: &str) -> Option<String> {
    val.get(key).and_then(|v| {
        if let Some(s) = v.as_str() {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        } else if let Some(n) = v.as_i64() {
            Some(n.to_string())
        } else {
            None
        }
    })
}

fn get_i32(val: &serde_json::Value, key: &str) -> Option<i32> {
    val.get(key).and_then(|v| {
        if let Some(n) = v.as_i64() {
            Some(n as i32)
        } else if let Some(s) = v.as_str() {
            s.parse::<i32>().ok()
        } else {
            None
        }
    })
}

fn get_f64(val: &serde_json::Value, key: &str) -> Option<f64> {
    val.get(key).and_then(|v| {
        if let Some(n) = v.as_f64() {
            Some(n)
        } else if let Some(s) = v.as_str() {
            s.parse::<f64>().ok()
        } else {
            None
        }
    })
}

#[derive(Debug, Clone)]
pub struct XcCategory {
    pub category_id: String,
    pub category_name: String,
    pub parent_id: i32,
}

#[derive(Debug, Clone)]
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

fn parse_categories(text: &str) -> Vec<XcCategory> {
    let mut categories = Vec::new();
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(text) {
        for val in arr {
            categories.push(XcCategory {
                category_id: get_string(&val, "category_id").unwrap_or_default(),
                category_name: get_string(&val, "category_name")
                    .unwrap_or_else(|| "Unknown".to_string()),
                parent_id: get_i32(&val, "parent_id").unwrap_or(0),
            });
        }
    }
    categories
}

pub async fn get_live_categories(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcCategory>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));

    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_live_categories"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let text = res.text().await?;
    Ok(parse_categories(&text))
}

pub async fn get_live_streams(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcStream>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));

    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_live_streams"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let text = res.text().await?;
    let mut streams = Vec::new();
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        for val in arr {
            let stream_id = get_i32(&val, "stream_id").unwrap_or(0);
            if stream_id == 0 {
                continue;
            }
            streams.push(XcStream {
                num: val.get("num").cloned(),
                name: get_string(&val, "name").unwrap_or_else(|| "Unknown".to_string()),
                stream_type: get_string(&val, "stream_type"),
                stream_id,
                stream_icon: get_string(&val, "stream_icon"),
                epg_channel_id: get_string(&val, "epg_channel_id"),
                added: get_string(&val, "added"),
                category_id: get_string(&val, "category_id").unwrap_or_default(),
                custom_sid: get_string(&val, "custom_sid"),
                tv_archive: get_i32(&val, "tv_archive"),
                direct_source: get_string(&val, "direct_source"),
                tv_archive_duration: get_i32(&val, "tv_archive_duration"),
            });
        }
    } else {
        return Err("Failed to parse live streams JSON array".into());
    }
    Ok(streams)
}

pub async fn get_vod_categories(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcCategory>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_vod_categories"),
        ])
        .send()
        .await?
        .error_for_status()?;
    let text = res.text().await?;
    Ok(parse_categories(&text))
}

#[derive(Debug, Clone)]
pub struct XcVodStream {
    pub num: Option<serde_json::Value>,
    pub name: String,
    pub stream_type: Option<String>,
    pub stream_id: i32,
    pub stream_icon: Option<String>,
    pub added: Option<String>,
    pub category_id: String,
    pub container_extension: Option<String>,
    pub custom_sid: Option<String>,
    pub direct_source: Option<String>,
}

pub async fn get_vod_streams(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcVodStream>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_vod_streams"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let text = res.text().await?;
    let mut streams = Vec::new();
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        for val in arr {
            let stream_id = get_i32(&val, "stream_id").unwrap_or(0);
            if stream_id == 0 {
                continue;
            }
            streams.push(XcVodStream {
                num: val.get("num").cloned(),
                name: get_string(&val, "name").unwrap_or_else(|| "Unknown".to_string()),
                stream_type: get_string(&val, "stream_type"),
                stream_id,
                stream_icon: get_string(&val, "stream_icon"),
                added: get_string(&val, "added"),
                category_id: get_string(&val, "category_id").unwrap_or_default(),
                container_extension: get_string(&val, "container_extension"),
                custom_sid: get_string(&val, "custom_sid"),
                direct_source: get_string(&val, "direct_source"),
            });
        }
    } else {
        return Err("Failed to parse vod streams JSON array".into());
    }
    Ok(streams)
}

pub async fn get_series_categories(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcCategory>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_series_categories"),
        ])
        .send()
        .await?
        .error_for_status()?;
    let text = res.text().await?;
    Ok(parse_categories(&text))
}

#[derive(Debug, Clone)]
pub struct XcSeries {
    pub num: Option<serde_json::Value>,
    pub name: String,
    pub series_id: i32,
    pub cover: Option<String>,
    pub plot: Option<String>,
    pub cast: Option<String>,
    pub director: Option<String>,
    pub genre: Option<String>,
    pub releaseDate: Option<String>,
    pub last_modified: Option<String>,
    pub rating: Option<String>,
    pub rating_5based: Option<f64>,
    pub category_id: String,
}

pub async fn get_series(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<XcSeries>, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_series"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let text = res.text().await?;
    let mut streams = Vec::new();
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        for val in arr {
            let series_id = get_i32(&val, "series_id").unwrap_or(0);
            if series_id == 0 {
                continue;
            }
            streams.push(XcSeries {
                num: val.get("num").cloned(),
                name: get_string(&val, "name").unwrap_or_else(|| "Unknown".to_string()),
                series_id,
                cover: get_string(&val, "cover"),
                plot: get_string(&val, "plot"),
                cast: get_string(&val, "cast"),
                director: get_string(&val, "director"),
                genre: get_string(&val, "genre"),
                releaseDate: get_string(&val, "releaseDate"),
                last_modified: get_string(&val, "last_modified"),
                rating: get_string(&val, "rating"),
                rating_5based: get_f64(&val, "rating_5based"),
                category_id: get_string(&val, "category_id").unwrap_or_default(),
            });
        }
    } else {
        return Err("Failed to parse series JSON array".into());
    }
    Ok(streams)
}

pub async fn get_series_info(
    client: &Client,
    server_url: &str,
    username: &str,
    password: &str,
    series_id: i32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let url = format!("{}/player_api.php", server_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .query(&[
            ("username", username),
            ("password", password),
            ("action", "get_series_info"),
            ("series_id", &series_id.to_string()),
        ])
        .send()
        .await?
        .error_for_status()?;

    let text = res.text().await?;
    let info: serde_json::Value = serde_json::from_str(&text)?;
    Ok(info)
}
