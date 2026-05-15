use serde::{Deserialize, Serialize};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use crate::entities::core_settings;
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSettings {
    pub enabled: bool,
    pub api_key: String,
}

impl Default for TmdbSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: String::new(),
        }
    }
}

pub async fn get_tmdb_settings(db: &sea_orm::DatabaseConnection) -> TmdbSettings {
    let s = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("tmdb_settings"))
        .one(db)
        .await;

    if let Ok(Some(s)) = s {
        if let Ok(settings) = serde_json::from_value(s.value) {
            return settings;
        }
    }

    TmdbSettings::default()
}

#[derive(Debug, Deserialize)]
pub struct TmdbSearchResponse {
    pub results: Vec<TmdbSearchResult>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmdbSearchResult {
    pub id: i64,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    pub vote_average: Option<f64>,
}

pub async fn search_movie(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<i32>,
) -> Result<Option<TmdbSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = format!(
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}",
        api_key,
        urlencoding::encode(title)
    );
    if let Some(y) = year {
        url.push_str(&format!("&primary_release_year={}", y));
    }

    let resp = client.get(&url).send().await?.json::<TmdbSearchResponse>().await?;
    Ok(resp.results.into_iter().next())
}

pub async fn search_tv(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<i32>,
) -> Result<Option<TmdbSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = format!(
        "https://api.themoviedb.org/3/search/tv?api_key={}&query={}",
        api_key,
        urlencoding::encode(title)
    );
    if let Some(y) = year {
        url.push_str(&format!("&first_air_date_year={}", y));
    }

    let resp = client.get(&url).send().await?.json::<TmdbSearchResponse>().await?;
    Ok(resp.results.into_iter().next())
}
