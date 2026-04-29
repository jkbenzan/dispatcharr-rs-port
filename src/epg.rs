use crate::entities::{epg_data, epg_program, epg_source};
use chrono::Utc;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::path::PathBuf;

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
    let temp_file = download_to_temp_file(url).await?;
    let file_path = temp_file.clone();

    let file_size = std::fs::metadata(&file_path)?.len();
    update_source_status(
        db,
        source_id,
        "parsing",
        format!(
            "Downloaded XMLTV ({:.1} MB). Parsing channels...",
            file_size as f64 / 1_048_576.0
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

    let path_for_channels = file_path.clone();
    let parsed_channels = tokio::task::spawn_blocking(move || {
        let reader = create_xml_reader(&path_for_channels)?;
        parse_xmltv_channels(reader)
    })
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

    let path_for_programs = file_path.clone();
    let parsed_programs = tokio::task::spawn_blocking(move || {
        let reader = create_xml_reader(&path_for_programs)?;
        parse_xmltv_programs(reader, epg_channel_map)
    })
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))??;

    insert_programs(db, parsed_programs).await?;

    // Cleanup temp file
    let _ = std::fs::remove_file(&file_path);

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

async fn download_to_temp_file(url: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let temp_dir = std::env::temp_dir();
    let file_name = format!("epg_{}.xml", rand::random::<u64>());
    let file_path = temp_dir.join(file_name);

    if std::path::Path::new(url).exists() {
        std::fs::copy(url, &file_path)?;
        return Ok(file_path);
    }

    let client = reqwest::Client::builder()
        .user_agent("Dispatcharr/1.0")
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let mut response = client.get(url).send().await?.error_for_status()?;
    let mut file = std::fs::File::create(&file_path)?;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
    }

    Ok(file_path)
}

fn create_xml_reader(
    path: &std::path::Path,
) -> Result<Reader<BufReader<Box<dyn Read + Send>>>, Box<dyn Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let reader: Box<dyn Read + Send> = if path.to_string_lossy().ends_with(".gz")
        || is_gzipped(path)?
    {
        Box::new(GzDecoder::new(file))
    } else if path.to_string_lossy().ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(file)?;
        let mut found = false;
        let mut inner_reader: Option<Box<dyn Read + Send>> = None;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.is_file() {
                inner_reader = Some(Box::new(Cursor::new(file.bytes().collect::<Result<Vec<u8>, _>>()?)));
                found = true;
                break;
            }
        }
        if !found {
            return Err("No file found in ZIP".into());
        }
        inner_reader.unwrap()
    } else {
        Box::new(file)
    };

    let mut xml_reader = Reader::from_reader(BufReader::new(reader));
    xml_reader.trim_text(true);
    Ok(xml_reader)
}

fn is_gzipped(path: &std::path::Path) -> Result<bool, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 2];
    let _ = file.read(&mut buf);
    Ok(buf == [0x1f, 0x8b])
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

fn parse_xmltv_channels<R: BufRead>(
    mut reader: Reader<R>,
) -> Result<Vec<ParsedChannel>, Box<dyn Error + Send + Sync>> {
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

fn parse_xmltv_programs<R: BufRead>(
    mut reader: Reader<R>,
    epg_channel_map: HashMap<String, i64>,
) -> Result<Vec<ParsedProgram>, Box<dyn Error + Send + Sync>> {
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
