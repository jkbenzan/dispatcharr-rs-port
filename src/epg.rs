use crate::entities::{epg_data, epg_program, epg_source};
use chrono::Utc;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::error::Error;
use std::io::{Cursor, Read};

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
    if let Ok(Some(src)) = epg_source::Entity::find_by_id(source_id).one(db).await {
        let mut active: epg_source::ActiveModel = src.into();
        active.status = Set("fetching".to_string());
        active.last_message = Set(Some("Downloading & parsing XMLTV...".to_string()));
        let _ = active.update(db).await;
    }

    println!("Fetching XMLTV EPG from {}", url);
    let xml_data = if std::path::Path::new(url).exists() {
        let bytes = tokio::fs::read(url).await?;
        decode_xmltv_payload(bytes, url)?
    } else {
        let client = reqwest::Client::builder()
            .user_agent("Dispatcharr/1.0")
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(20))
            .build()?;

        let response = client.get(url).send().await?.error_for_status()?;
        let bytes = response.bytes().await?.to_vec();
        decode_xmltv_payload(bytes, url)?
    };

    if let Ok(Some(src)) = epg_source::Entity::find_by_id(source_id).one(db).await {
        let mut active: epg_source::ActiveModel = src.into();
        active.status = Set("parsing".to_string());
        active.last_message = Set(Some(format!(
            "Downloaded XMLTV ({:.1} MB). Parsing channels...",
            xml_data.len() as f64 / 1_048_576.0
        )));
        let _ = active.update(db).await;
    }

    let existing_channels = epg_data::Entity::find()
        .filter(epg_data::Column::EpgSourceId.eq(source_id))
        .all(db)
        .await
        .unwrap_or_default();

    let mut epg_channel_map: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    for ch in existing_channels {
        if let Some(tvg) = ch.tvg_id.clone() {
            epg_channel_map.insert(tvg, ch.id);
        }
    }

    let mut reader = Reader::from_str(&xml_data);
    let mut buf = Vec::new();
    let mut current_channel: Option<epg_data::ActiveModel> = None;
    let mut channels_batch = vec![];
    let mut in_channel = false;
    let mut current_tag = String::new();

    // First pass: make EPGData rows exist before programmes reference them.
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                current_tag = name.to_string();

                if name == "channel" {
                    in_channel = true;
                    let mut tvg_id = None;
                    for attr in e.attributes() {
                        if let Ok(a) = attr {
                            if a.key.as_ref() == b"id" {
                                tvg_id = Some(
                                    String::from_utf8(a.value.into_owned()).unwrap_or_default(),
                                );
                            }
                        }
                    }
                    current_channel = Some(epg_data::ActiveModel {
                        tvg_id: Set(tvg_id),
                        name: Set("Unknown".to_string()),
                        epg_source_id: Set(Some(source_id)),
                        ..Default::default()
                    });
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                if name == "icon" && in_channel {
                    for attr in e.attributes() {
                        if let Ok(a) = attr {
                            if a.key.as_ref() == b"src" {
                                if let Some(mut ch) = current_channel.take() {
                                    ch.icon_url = Set(Some(
                                        String::from_utf8(a.value.into_owned()).unwrap_or_default(),
                                    ));
                                    current_channel = Some(ch);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let txt = e.unescape().unwrap_or_default().into_owned();
                let txt = txt.trim();
                if txt.is_empty() {
                    continue;
                }

                if in_channel {
                    if let Some(mut ch) = current_channel.take() {
                        if current_tag == "display-name" {
                            ch.name = Set(txt.to_string());
                        }
                        current_channel = Some(ch);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                if name == "channel" {
                    in_channel = false;
                    if let Some(ch) = current_channel.take() {
                        if let sea_orm::ActiveValue::Set(Some(tvg)) = ch.tvg_id.clone() {
                            if !epg_channel_map.contains_key(&tvg) {
                                epg_channel_map.insert(tvg, 0);
                                channels_batch.push(ch);
                            }
                        } else {
                            channels_batch.push(ch);
                        }
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

    if !channels_batch.is_empty() {
        let _ = epg_data::Entity::insert_many(channels_batch).exec(db).await;
        tokio::task::yield_now().await;
    }

    let source_channels = epg_data::Entity::find()
        .filter(epg_data::Column::EpgSourceId.eq(source_id))
        .all(db)
        .await
        .unwrap_or_default();

    epg_channel_map.clear();
    let mut epg_ids = Vec::with_capacity(source_channels.len());
    for ch in source_channels {
        epg_ids.push(ch.id);
        if let Some(tvg) = ch.tvg_id.clone() {
            epg_channel_map.insert(tvg, ch.id);
        }
    }

    if let Ok(Some(src)) = epg_source::Entity::find_by_id(source_id).one(db).await {
        let mut active: epg_source::ActiveModel = src.into();
        active.status = Set("parsing".to_string());
        active.last_message = Set(Some("Parsing XMLTV programmes...".to_string()));
        let _ = active.update(db).await;
    }

    if !epg_ids.is_empty() {
        let _ = epg_program::Entity::delete_many()
            .filter(epg_program::Column::EpgId.is_in(epg_ids))
            .exec(db)
            .await;
    }

    let mut reader = Reader::from_str(&xml_data);
    let mut buf = Vec::new();
    let mut current_program: Option<epg_program::ActiveModel> = None;
    let mut programs_batch = vec![];
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
                    let mut prog = epg_program::ActiveModel {
                        title: Set("Unknown".to_string()),
                        start_time: Set(Utc::now().into()),
                        end_time: Set(Utc::now().into()),
                        ..Default::default()
                    };
                    let mut matched_epg_id = None;

                    for attr in e.attributes() {
                        if let Ok(a) = attr {
                            let key = a.key.as_ref();
                            let val = String::from_utf8(a.value.into_owned()).unwrap_or_default();
                            if key == b"start" {
                                if let Some(dt) = parse_xmltv_datetime(&val) {
                                    prog.start_time = Set(dt);
                                }
                            } else if key == b"stop" {
                                if let Some(dt) = parse_xmltv_datetime(&val) {
                                    prog.end_time = Set(dt);
                                }
                            } else if key == b"channel" {
                                prog.tvg_id = Set(Some(val.clone()));
                                matched_epg_id = epg_channel_map.get(&val).copied();
                            }
                        }
                    }

                    if let Some(epg_id) = matched_epg_id {
                        prog.epg_id = Set(epg_id);
                        current_program = Some(prog);
                    } else {
                        current_program = None;
                    }
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

                if let Some(mut prog) = current_program.take() {
                    if current_tag == "title" {
                        prog.title = Set(txt.to_string());
                    } else if current_tag == "desc" {
                        prog.description = Set(Some(txt.to_string()));
                    } else if current_tag == "sub-title" {
                        prog.sub_title = Set(Some(txt.to_string()));
                    }
                    current_program = Some(prog);
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = std::str::from_utf8(qname.into_inner()).unwrap_or("");
                if name == "programme" {
                    in_programme = false;
                    if let Some(prog) = current_program.take() {
                        programs_batch.push(prog);
                        if programs_batch.len() >= 1000 {
                            let chunk = std::mem::take(&mut programs_batch);
                            let _ = epg_program::Entity::insert_many(chunk).exec(db).await;
                            tokio::task::yield_now().await;
                        }
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

    if !programs_batch.is_empty() {
        let _ = epg_program::Entity::insert_many(programs_batch)
            .exec(db)
            .await;
        tokio::task::yield_now().await;
    }

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
