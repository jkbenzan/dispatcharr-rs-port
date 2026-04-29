use crate::entities::{epg_data, epg_program, epg_source};
use chrono::Utc;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{Cursor, Read};

const XMLTV_INSERT_BATCH_SIZE: usize = 1_000;

#[derive(Clone, Debug)]
struct ParsedChannel {
    tvg_id: Option<String>,
    name: String,
    icon_url: Option<String>,
}

#[derive(Clone, Debug)]
struct ParsedProgram {
    start_time: chrono::DateTime<chrono::FixedOffset>,
    end_time: chrono::DateTime<chrono::FixedOffset>,
    title: String,
    sub_title: Option<String>,
    description: Option<String>,
    tvg_id: Option<String>,
    epg_id: i64,
}

fn parse_xmltv_datetime(value: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_str(value, "%Y%m%d%H%M%S %z")
        .or_else(|_| chrono::DateTime::parse_from_str(value, "%Y%m%d%H%M%S%z"))
        .ok()
}

pub async fn refresh_all_guides(
    db: &DatabaseConnection,
    url: &str,
    source_id: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    update_source_status(
        db,
        source_id,
        "fetching",
        "Downloading XMLTV guide...".to_string(),
    )
    .await;

    println!("Fetching XMLTV EPG from {}", url);
    let xml_data = fetch_xmltv_payload(url).await?;

    update_source_status(
        db,
        source_id,
        "parsing",
        format!(
            "Downloaded XMLTV ({:.1} MB). Parsing channels...",
            xml_data.len() as f64 / 1_048_576.0
        ),
    )
    .await;

    let existing_channels = epg_data::Entity::find()
        .filter(epg_data::Column::EpgSourceId.eq(source_id))
        .all(db)
        .await
        .unwrap_or_default();

    let existing_tvg_ids: HashSet<String> = existing_channels
        .iter()
        .filter_map(|channel| channel.tvg_id.clone())
        .collect();

    let channels_xml = xml_data.clone();
    let parsed_channels = tokio::task::spawn_blocking(move || parse_xmltv_channels(&channels_xml))
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))??;

    insert_missing_channels(db, source_id, parsed_channels, &existing_tvg_ids).await?;

    let source_channels = epg_data::Entity::find()
        .filter(epg_data::Column::EpgSourceId.eq(source_id))
        .all(db)
        .await
        .unwrap_or_default();

    let mut epg_channel_map = HashMap::new();
    let mut epg_ids = Vec::with_capacity(source_channels.len());
    for channel in source_channels {
        epg_ids.push(channel.id);
        if let Some(tvg_id) = channel.tvg_id {
            epg_channel_map.insert(tvg_id, channel.id);
        }
    }

    update_source_status(
        db,
        source_id,
        "parsing",
        "Parsing XMLTV programmes...".to_string(),
    )
    .await;

    if !epg_ids.is_empty() {
        let _ = epg_program::Entity::delete_many()
            .filter(epg_program::Column::EpgId.is_in(epg_ids))
            .exec(db)
            .await;
    }

    let programs_xml = xml_data;
    let parsed_programs =
        tokio::task::spawn_blocking(move || parse_xmltv_programs(&programs_xml, epg_channel_map))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))??;

    insert_programs(db, parsed_programs).await?;

    if let Ok(Some(src)) = epg_source::Entity::find_by_id(source_id).one(db).await {
        let mut active: epg_source::ActiveModel = src.into();
        active.status = Set("success".to_string());
        active.last_message = Set(Some("Successfully synced XMLTV!".to_string()));
        active.updated_at = Set(Some(Utc::now().into()));
        let _ = active.update(db).await;
    }

    println!("EPG Parsing Complete for Source {}", source_id);
    Ok(())
}

async fn update_source_status(
    db: &DatabaseConnection,
    source_id: i64,
    status: &str,
    message: String,
) {
    if let Ok(Some(src)) = epg_source::Entity::find_by_id(source_id).one(db).await {
        let mut active: epg_source::ActiveModel = src.into();
        active.status = Set(status.to_string());
        active.last_message = Set(Some(message));
        let _ = active.update(db).await;
    }
}

async fn fetch_xmltv_payload(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    if std::path::Path::new(url).exists() {
        let bytes = tokio::fs::read(url).await?;
        return decode_xmltv_payload(bytes, url);
    }

    let client = reqwest::Client::builder()
        .user_agent("Dispatcharr/1.0")
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()?;

    let response = client.get(url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?.to_vec();
    decode_xmltv_payload(bytes, url)
}

async fn insert_missing_channels(
    db: &DatabaseConnection,
    source_id: i64,
    channels: Vec<ParsedChannel>,
    existing_tvg_ids: &HashSet<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut seen = existing_tvg_ids.clone();
    let mut batch = Vec::new();

    for channel in channels {
        if let Some(tvg_id) = &channel.tvg_id {
            if !seen.insert(tvg_id.clone()) {
                continue;
            }
        }

        batch.push(epg_data::ActiveModel {
            tvg_id: Set(channel.tvg_id),
            name: Set(channel.name),
            epg_source_id: Set(Some(source_id)),
            icon_url: Set(channel.icon_url),
            ..Default::default()
        });

        if batch.len() >= XMLTV_INSERT_BATCH_SIZE {
            let chunk = std::mem::take(&mut batch);
            let _ = epg_data::Entity::insert_many(chunk).exec(db).await;
            tokio::task::yield_now().await;
        }
    }

    if !batch.is_empty() {
        let _ = epg_data::Entity::insert_many(batch).exec(db).await;
        tokio::task::yield_now().await;
    }

    Ok(())
}

async fn insert_programs(
    db: &DatabaseConnection,
    programs: Vec<ParsedProgram>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut batch = Vec::new();

    for program in programs {
        batch.push(epg_program::ActiveModel {
            start_time: Set(program.start_time),
            end_time: Set(program.end_time),
            title: Set(program.title),
            sub_title: Set(program.sub_title),
            description: Set(program.description),
            tvg_id: Set(program.tvg_id),
            epg_id: Set(program.epg_id),
            ..Default::default()
        });

        if batch.len() >= XMLTV_INSERT_BATCH_SIZE {
            let chunk = std::mem::take(&mut batch);
            let _ = epg_program::Entity::insert_many(chunk).exec(db).await;
            tokio::task::yield_now().await;
        }
    }

    if !batch.is_empty() {
        let _ = epg_program::Entity::insert_many(batch).exec(db).await;
        tokio::task::yield_now().await;
    }

    Ok(())
}

fn parse_xmltv_channels(
    xml_data: &str,
) -> Result<Vec<ParsedChannel>, Box<dyn Error + Send + Sync>> {
    let mut reader = Reader::from_str(xml_data);
    let mut buf = Vec::new();
    let mut current_channel: Option<ParsedChannel> = None;
    let mut channels = Vec::new();
    let mut in_channel = false;
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                current_tag = name.to_string();

                if name == "channel" {
                    in_channel = true;
                    let mut tvg_id = None;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"id" {
                            tvg_id = Some(
                                String::from_utf8(attr.value.into_owned()).unwrap_or_default(),
                            );
                        }
                    }
                    current_channel = Some(ParsedChannel {
                        tvg_id,
                        name: "Unknown".to_string(),
                        icon_url: None,
                    });
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                if name == "icon" && in_channel {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"src" {
                            if let Some(channel) = current_channel.as_mut() {
                                channel.icon_url = Some(
                                    String::from_utf8(attr.value.into_owned()).unwrap_or_default(),
                                );
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_channel && current_tag == "display-name" {
                    let txt = e.unescape().unwrap_or_default().into_owned();
                    let txt = txt.trim();
                    if !txt.is_empty() {
                        if let Some(channel) = current_channel.as_mut() {
                            channel.name = txt.to_string();
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                if name == "channel" {
                    in_channel = false;
                    if let Some(channel) = current_channel.take() {
                        channels.push(channel);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid XMLTV while parsing channels: {}", e),
                )
                .into())
            }
            _ => (),
        }
        buf.clear();
    }

    Ok(channels)
}

fn parse_xmltv_programs(
    xml_data: &str,
    epg_channel_map: HashMap<String, i64>,
) -> Result<Vec<ParsedProgram>, Box<dyn Error + Send + Sync>> {
    let mut reader = Reader::from_str(xml_data);
    let mut buf = Vec::new();
    let mut current_program: Option<ParsedProgram> = None;
    let mut programs = Vec::new();
    let mut in_programme = false;
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                current_tag = name.to_string();

                if name == "programme" {
                    in_programme = true;
                    let now = Utc::now().into();
                    let mut start_time = now;
                    let mut end_time = now;
                    let mut tvg_id = None;
                    let mut matched_epg_id = None;

                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        let val = String::from_utf8(attr.value.into_owned()).unwrap_or_default();
                        if key == b"start" {
                            if let Some(dt) = parse_xmltv_datetime(&val) {
                                start_time = dt;
                            }
                        } else if key == b"stop" {
                            if let Some(dt) = parse_xmltv_datetime(&val) {
                                end_time = dt;
                            }
                        } else if key == b"channel" {
                            matched_epg_id = epg_channel_map.get(&val).copied();
                            tvg_id = Some(val);
                        }
                    }

                    current_program = matched_epg_id.map(|epg_id| ParsedProgram {
                        start_time,
                        end_time,
                        title: "Unknown".to_string(),
                        sub_title: None,
                        description: None,
                        tvg_id,
                        epg_id,
                    });
                }
            }
            Ok(Event::Text(e)) => {
                if !in_programme {
                    buf.clear();
                    continue;
                }

                let txt = e.unescape().unwrap_or_default().into_owned();
                let txt = txt.trim();
                if txt.is_empty() {
                    buf.clear();
                    continue;
                }

                if let Some(program) = current_program.as_mut() {
                    if current_tag == "title" {
                        program.title = txt.to_string();
                    } else if current_tag == "desc" {
                        program.description = Some(txt.to_string());
                    } else if current_tag == "sub-title" {
                        program.sub_title = Some(txt.to_string());
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                if name == "programme" {
                    in_programme = false;
                    if let Some(program) = current_program.take() {
                        programs.push(program);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid XMLTV while parsing programmes: {}", e),
                )
                .into())
            }
            _ => (),
        }
        buf.clear();
    }

    Ok(programs)
}

fn decode_xmltv_payload(
    bytes: Vec<u8>,
    source: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    if bytes.starts_with(&[0x1f, 0x8b]) || source.ends_with(".gz") {
        let mut decoder = GzDecoder::new(Cursor::new(bytes));
        let mut decoded = String::new();
        decoder.read_to_string(&mut decoded)?;
        return Ok(decoded);
    }

    if bytes.starts_with(b"PK") || source.ends_with(".zip") {
        let reader = Cursor::new(bytes.clone());
        let mut archive = zip::ZipArchive::new(reader)?;
        for index in 0..archive.len() {
            let mut file = archive.by_index(index)?;
            if file.is_file() {
                let mut decoded = String::new();
                file.read_to_string(&mut decoded)?;
                return Ok(decoded);
            }
        }
    }

    Ok(String::from_utf8(bytes)?)
}
