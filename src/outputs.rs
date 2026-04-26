use crate::{entities::channel, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use sea_orm::EntityTrait;
use std::sync::Arc;

const STREAM_SECRET: &[u8] = b"dispatcharr_super_secret_temporary_key";

pub async fn generate_m3u(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // Basic implementation for M3U without auth to match original Django (or use headers)
    // In Django, /m3u/ was accessible without an explicit URL token parameter.

    let channels = channel::Entity::find()
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut m3u_body = String::from("#EXTM3U\n");

    let base_url = "http://localhost:8080"; // Ideally read from settings

    for ch in channels {
        let ch_id = ch.id;
        let name = ch.name;
        let tvg_id = ch.tvg_id.unwrap_or_default();
        let logo = ch.tvc_guide_stationid.unwrap_or_default(); // Example mapping

        m3u_body.push_str(&format!(
            "#EXTINF:-1 tvg-id=\"{}\" tvg-logo=\"{}\" group-title=\"Live TV\",{}\n",
            tvg_id, logo, name
        ));
        m3u_body.push_str(&format!("{}/stream/{}\n", base_url, ch.uuid));
    }

    Ok((
        [(axum::http::header::CONTENT_TYPE, "audio/x-mpegurl")],
        m3u_body,
    ))
}

pub async fn generate_xmltv(
    State(_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // We simply return a valid, skeletal XMLTV frame
    // In production, you would hydrate this with exact programs dynamically mapped.

    let xml_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv generator-info-name="Dispatcharr-RS">
  <channel id="CNN">
    <display-name>CNN HD</display-name>
  </channel>
  <programme start="20260416000000 +0000" stop="20260416010000 +0000" channel="CNN">
    <title>News Hour</title>
  </programme>
</tv>"#;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        xml_body.to_string(),
    ))
}
