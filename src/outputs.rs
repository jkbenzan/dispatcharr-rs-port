use crate::{
    entities::{channel, epg_data, epg_program, epg_source, logo},
    AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Duration, Timelike, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
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

fn xml_escape(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn format_channel_number(channel_number: f64) -> String {
    if channel_number.fract() == 0.0 {
        format!("{}", channel_number as i64)
    } else {
        channel_number.to_string()
    }
}

fn xmltv_channel_id(ch: &channel::Model, tvg_id_source: &str) -> String {
    match tvg_id_source {
        "tvg_id" => ch
            .tvg_id
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format_channel_number(ch.channel_number)),
        "gracenote" => ch
            .tvc_guide_stationid
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format_channel_number(ch.channel_number)),
        _ => {
            let number = format_channel_number(ch.channel_number);
            if number.is_empty() {
                ch.id.to_string()
            } else {
                number
            }
        }
    }
}

fn xmltv_time(value: chrono::DateTime<chrono::FixedOffset>) -> String {
    value.format("%Y%m%d%H%M%S %z").to_string()
}

fn json_text(value: &Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned).or_else(|| {
        if value.is_null() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn append_custom_program_fields(xml: &mut String, custom: &Value) {
    if let Some(categories) = custom.get("categories").and_then(|v| v.as_array()) {
        for category in categories {
            if let Some(text) = json_text(category) {
                xml.push_str(&format!("    <category>{}</category>\n", xml_escape(text)));
            }
        }
    }

    if let Some(keywords) = custom.get("keywords").and_then(|v| v.as_array()) {
        for keyword in keywords {
            if let Some(text) = json_text(keyword) {
                xml.push_str(&format!("    <keyword>{}</keyword>\n", xml_escape(text)));
            }
        }
    }

    if let Some(value) = custom.get("onscreen_episode").and_then(json_text) {
        xml.push_str(&format!(
            "    <episode-num system=\"onscreen\">{}</episode-num>\n",
            xml_escape(value)
        ));
    } else if let Some(value) = custom.get("episode").and_then(json_text) {
        xml.push_str(&format!(
            "    <episode-num system=\"onscreen\">E{}</episode-num>\n",
            xml_escape(value)
        ));
    }

    if let Some(value) = custom.get("dd_progid").and_then(json_text) {
        xml.push_str(&format!(
            "    <episode-num system=\"dd_progid\">{}</episode-num>\n",
            xml_escape(value)
        ));
    }

    for system in ["thetvdb.com", "themoviedb.org", "imdb.com"] {
        let key = format!("{system}_id");
        if let Some(value) = custom.get(&key).and_then(json_text) {
            xml.push_str(&format!(
                "    <episode-num system=\"{}\">{}</episode-num>\n",
                system,
                xml_escape(value)
            ));
        }
    }

    if let (Some(season), Some(episode)) = (
        custom.get("season").and_then(|v| v.as_i64()),
        custom.get("episode").and_then(|v| v.as_i64()),
    ) {
        xml.push_str(&format!(
            "    <episode-num system=\"xmltv_ns\">{}.{}.</episode-num>\n",
            season.saturating_sub(1),
            episode.saturating_sub(1)
        ));
    }

    for (key, tag) in [
        ("language", "language"),
        ("original_language", "orig-language"),
        ("date", "date"),
        ("country", "country"),
    ] {
        if let Some(value) = custom.get(key).and_then(json_text) {
            xml.push_str(&format!("    <{}>{}</{}>\n", tag, xml_escape(value), tag));
        }
    }

    if let Some(icon) = custom.get("icon").and_then(json_text) {
        xml.push_str(&format!("    <icon src=\"{}\" />\n", xml_escape(icon)));
    }

    if custom.get("previously_shown").and_then(|v| v.as_bool()) == Some(true) {
        xml.push_str("    <previously-shown />\n");
    }
    if custom.get("premiere").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(text) = custom.get("premiere_text").and_then(json_text) {
            xml.push_str(&format!("    <premiere>{}</premiere>\n", xml_escape(text)));
        } else {
            xml.push_str("    <premiere />\n");
        }
    }
    if custom.get("new").and_then(|v| v.as_bool()) == Some(true) {
        xml.push_str("    <new />\n");
    }
    if custom.get("live").and_then(|v| v.as_bool()) == Some(true) {
        xml.push_str("    <live />\n");
    }
}

fn append_program_xml(xml: &mut String, program: &epg_program::Model, channel_id: &str) {
    xml.push_str(&format!(
        "  <programme start=\"{}\" stop=\"{}\" channel=\"{}\">\n",
        xmltv_time(program.start_time),
        xmltv_time(program.end_time),
        xml_escape(channel_id)
    ));
    xml.push_str(&format!(
        "    <title>{}</title>\n",
        xml_escape(&program.title)
    ));

    if let Some(sub_title) = &program.sub_title {
        xml.push_str(&format!(
            "    <sub-title>{}</sub-title>\n",
            xml_escape(sub_title)
        ));
    }
    if let Some(description) = &program.description {
        xml.push_str(&format!("    <desc>{}</desc>\n", xml_escape(description)));
    }
    if let Some(custom) = &program.custom_properties {
        append_custom_program_fields(xml, custom);
    }

    xml.push_str("  </programme>\n");
}

fn append_dummy_programs(xml: &mut String, channel_id: &str, channel_name: &str, days: i64) {
    let now = Utc::now();
    let total_days = days.max(1);
    let descriptions = [
        "Late Night with {channel}",
        "Dawn Patrol with {channel}",
        "Mid-Morning on {channel}",
        "Afternoon Programming on {channel}",
        "Prime Time on {channel}",
        "Overnight Programming on {channel}",
    ];

    for day in 0..total_days {
        for hour in (0..24).step_by(4) {
            let start_time = (now + Duration::days(day))
                .with_hour(hour)
                .and_then(|dt| dt.with_minute(0))
                .and_then(|dt| dt.with_second(0))
                .and_then(|dt| dt.with_nanosecond(0))
                .unwrap_or(now);
            let end_time = start_time + Duration::hours(4);
            let description = descriptions[(hour / 4) as usize].replace("{channel}", channel_name);

            xml.push_str(&format!(
                "  <programme start=\"{}\" stop=\"{}\" channel=\"{}\">\n",
                start_time.format("%Y%m%d%H%M%S %z"),
                end_time.format("%Y%m%d%H%M%S %z"),
                xml_escape(channel_id)
            ));
            xml.push_str(&format!(
                "    <title>{}</title>\n",
                xml_escape(channel_name)
            ));
            xml.push_str(&format!("    <desc>{}</desc>\n", xml_escape(description)));
            xml.push_str("  </programme>\n");
        }
    }
}

pub async fn generate_xmltv(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let tvg_id_source = params
        .get("tvg_id_source")
        .map(|value| value.to_lowercase())
        .unwrap_or_else(|| "channel_number".to_string());
    let num_days = params
        .get("days")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .clamp(0, 365);
    let prev_days = params
        .get("prev_days")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .clamp(0, 30);

    let channels = channel::Entity::find()
        .order_by_asc(channel::Column::ChannelNumber)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let epg_ids: Vec<i64> = channels
        .iter()
        .filter_map(|ch| ch.epg_data_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let logo_ids: Vec<i64> = channels
        .iter()
        .filter_map(|ch| ch.logo_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let epg_rows = if epg_ids.is_empty() {
        Vec::new()
    } else {
        epg_data::Entity::find()
            .filter(epg_data::Column::Id.is_in(epg_ids.clone()))
            .all(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let epg_by_id: HashMap<i64, epg_data::Model> =
        epg_rows.into_iter().map(|row| (row.id, row)).collect();

    let source_ids: Vec<i64> = epg_by_id
        .values()
        .filter_map(|row| row.epg_source_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let source_rows = if source_ids.is_empty() {
        Vec::new()
    } else {
        epg_source::Entity::find()
            .filter(epg_source::Column::Id.is_in(source_ids))
            .all(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let dummy_source_ids: HashSet<i64> = source_rows
        .into_iter()
        .filter(|source| source.source_type == "dummy")
        .map(|source| source.id)
        .collect();

    let logo_rows = if logo_ids.is_empty() {
        Vec::new()
    } else {
        logo::Entity::find()
            .filter(logo::Column::Id.is_in(logo_ids))
            .all(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let logo_by_id: HashMap<i64, logo::Model> =
        logo_rows.into_iter().map(|row| (row.id, row)).collect();

    let mut channel_ids_by_epg_id: HashMap<i64, Vec<String>> = HashMap::new();
    let mut dummy_channels = Vec::new();
    let mut xml = String::with_capacity(128 * 1024);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<tv generator-info-name=\"Dispatcharr-RS\" generator-info-url=\"https://github.com/Dispatcharr/Dispatcharr\">\n");

    for ch in &channels {
        let channel_id = xmltv_channel_id(ch, &tvg_id_source);
        let icon_url = ch
            .logo_id
            .and_then(|id| logo_by_id.get(&id))
            .map(|logo| logo.url.as_str())
            .unwrap_or("");

        xml.push_str(&format!("  <channel id=\"{}\">\n", xml_escape(&channel_id)));
        xml.push_str(&format!(
            "    <display-name>{}</display-name>\n",
            xml_escape(&ch.name)
        ));
        if !icon_url.is_empty() {
            xml.push_str(&format!("    <icon src=\"{}\" />\n", xml_escape(icon_url)));
        }
        xml.push_str("  </channel>\n");

        let needs_dummy = match ch.epg_data_id {
            None => true,
            Some(epg_id) => epg_by_id
                .get(&epg_id)
                .and_then(|row| row.epg_source_id)
                .map(|source_id| dummy_source_ids.contains(&source_id))
                .unwrap_or(false),
        };

        if needs_dummy {
            dummy_channels.push((channel_id, ch.name.clone()));
        } else if let Some(epg_id) = ch.epg_data_id {
            channel_ids_by_epg_id
                .entry(epg_id)
                .or_default()
                .push(channel_id);
        }
    }

    let dummy_days = if num_days > 0 { num_days } else { 3 };
    for (channel_id, channel_name) in dummy_channels {
        append_dummy_programs(&mut xml, &channel_id, &channel_name, dummy_days);
    }

    let real_epg_ids: Vec<i64> = channel_ids_by_epg_id.keys().copied().collect();
    if !real_epg_ids.is_empty() {
        let now = Utc::now();
        let mut query = epg_program::Entity::find()
            .filter(epg_program::Column::EpgId.is_in(real_epg_ids))
            .filter(epg_program::Column::EndTime.gte(now - Duration::days(prev_days)))
            .order_by_asc(epg_program::Column::EpgId)
            .order_by_asc(epg_program::Column::StartTime);

        if num_days > 0 {
            query = query.filter(epg_program::Column::StartTime.lt(now + Duration::days(num_days)));
        }

        let programs = query
            .all(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        for program in programs {
            if let Some(channel_ids) = channel_ids_by_epg_id.get(&program.epg_id) {
                for channel_id in channel_ids {
                    append_program_xml(&mut xml, &program, channel_id);
                }
            }
        }
    }

    xml.push_str("</tv>\n");

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"Dispatcharr.xml\"",
            ),
        ],
        xml,
    ))
}
