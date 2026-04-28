use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{error, info};
use std::time::Duration;

use crate::AppState;
use crate::entities::stream;
use crate::entities::stream_sorting_rule;
use crate::entities::channel_stream;
use sea_orm::{ActiveValue, QueryOrder};
use std::process::Command as StdCommand;

/// Resolve the path to `ffprobe`. Checks the FFPROBE_PATH env var first,
/// then the ffmpeg-sidecar managed path, then common install locations,
/// then falls back to the bare name.
fn resolve_ffprobe() -> String {
    // Check system PATH first
    if let Ok(output) = StdCommand::new("ffprobe").arg("-version").output() {
        if output.status.success() {
            return "ffprobe".to_string();
        }
    }

    if let Ok(p) = std::env::var("FFPROBE_PATH") {
        if !p.is_empty() {
            info!("🔍 Using FFPROBE_PATH from env: {}", p);
            return p;
        }
    }

    // Check sidecar first
    if let Ok(dir) = ffmpeg_sidecar::paths::sidecar_dir() {
        let fname = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };
        let sidecar_path = dir.join(fname);
        if sidecar_path.is_file() {
            let p = sidecar_path.to_string_lossy().to_string();
            // Smoke test: must start AND return success
            if let Ok(output) = StdCommand::new(&p).arg("-version").output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("unknown").to_string();
                    info!("✅ Using sidecar ffprobe: {} (Version: {})", p, version);
                    return p;
                } else {
                    info!("⚠️  Sidecar ffprobe found but returned error status on -version: {}", p);
                }
            } else {
                info!("⚠️  Sidecar ffprobe found but failed to execute: {}", p);
            }
        }
    }

    let candidates = [
        "/usr/bin/ffprobe",
        "/usr/local/bin/ffprobe",
        "/data/ffmpeg-sidecar/ffprobe",
        "/data/ffprobe",
        "/usr/local/sbin/ffprobe",
        "/opt/ffmpeg/bin/ffprobe",
        "C:/ffmpeg/bin/ffprobe.exe",
        "C:/Program Files/ffmpeg/bin/ffprobe.exe",
        "C:/Program Files/DownloadHelper CoApp/ffprobe.exe",
    ];

    for c in &candidates {
        let path = std::path::Path::new(c);
        if path.is_file() {
            if let Ok(output) = StdCommand::new(c).arg("-version").output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("unknown").to_string();
                    info!("✅ Using system ffprobe: {} (Version: {})", c, version);
                    return c.to_string();
                } else {
                    info!("⚠️  Candidate {} found but returned error status on -version", c);
                }
            } else {
                info!("⚠️  Candidate {} found but failed to execute", c);
            }
        }
    }

    info!("⚠️  No ffprobe found in common locations, falling back to PATH");
    "ffprobe".to_string()
}

/// Resolve the path to `ffmpeg`. Checks the FFMPEG_PATH env var first,
/// then the ffmpeg-sidecar managed path, then common install locations,
/// then falls back to the bare name.
fn resolve_ffmpeg() -> String {
    // Check system PATH first
    if let Ok(output) = StdCommand::new("ffmpeg").arg("-version").output() {
        if output.status.success() {
            return "ffmpeg".to_string();
        }
    }

    if let Ok(p) = std::env::var("FFMPEG_PATH") {
        if !p.is_empty() {
            info!("🔍 Using FFMPEG_PATH from env: {}", p);
            return p;
        }
    }

    // Check sidecar first
    let sidecar_path = ffmpeg_sidecar::paths::ffmpeg_path();
    if sidecar_path.is_file() {
        let p = sidecar_path.to_string_lossy().to_string();
        if let Ok(output) = StdCommand::new(&p).arg("-version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("unknown").to_string();
                info!("✅ Using sidecar ffmpeg: {} (Version: {})", p, version);
                return p;
            } else {
                info!("⚠️  Sidecar ffmpeg found but returned error status on -version: {}", p);
            }
        } else {
            info!("⚠️  Sidecar ffmpeg found but failed to execute: {}", p);
        }
    }

    let candidates = [
        "/usr/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/data/ffmpeg-sidecar/ffmpeg",
        "/data/ffmpeg",
        "/usr/local/sbin/ffmpeg",
        "/opt/ffmpeg/bin/ffmpeg",
        "C:/ffmpeg/bin/ffmpeg.exe",
        "C:/Program Files/ffmpeg/bin/ffmpeg.exe",
        "C:/Program Files/DownloadHelper CoApp/ffmpeg.exe",
    ];

    for c in &candidates {
        let path = std::path::Path::new(c);
        if path.is_file() {
            if let Ok(output) = StdCommand::new(c).arg("-version").output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("unknown").to_string();
                    info!("✅ Using system ffmpeg: {} (Version: {})", c, version);
                    return c.to_string();
                } else {
                    info!("⚠️  Candidate {} found but returned error status on -version", c);
                }
            } else {
                info!("⚠️  Candidate {} found but failed to execute", c);
            }
        }
    }

    info!("⚠️  No ffmpeg found in common locations, falling back to PATH");
    "ffmpeg".to_string()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BulkCheckStatus {
    pub is_running: bool,
    pub total: usize,
    pub completed: usize,
    pub successful: usize,
    pub failed: usize,
    pub current_stream_id: Option<i64>,
    pub current_stream_name: Option<String>,
}

impl Default for BulkCheckStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            total: 0,
            completed: 0,
            successful: 0,
            failed: 0,
            current_stream_id: None,
            current_stream_name: None,
        }
    }
}

pub async fn check_single_stream(
    state: &Arc<AppState>,
    stream_id: i64,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let stream_obj = match stream::Entity::find_by_id(stream_id).one(&state.db).await {
        Ok(Some(s)) => s,
        _ => return Err((StatusCode::NOT_FOUND, "Stream not found".to_string())),
    };

    let stream_url = match &stream_obj.url {
        Some(url) => url.clone(),
        None => return Err((StatusCode::BAD_REQUEST, "Stream has no URL".to_string())),
    };

    info!("🔍 Testing Stream: {} (URL: {})", stream_obj.name, stream_url);

    // 1. Run ffprobe
    let ffprobe_bin = resolve_ffprobe();
    let args = [
        "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.3",
        "-print_format", "json",
        "-show_streams",
        "-i", &stream_url,
    ];
    let mut ffprobe_cmd = Command::new(&ffprobe_bin);
    ffprobe_cmd.args(&args);
    
    info!("🚀 Executing ffprobe: {} {}", ffprobe_bin, args.join(" "));

    let ffprobe_result = match tokio::time::timeout(Duration::from_secs(40), ffprobe_cmd.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            error!("ffprobe failed to start: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("ffprobe failed to start: {}", e)));
        }
        Err(_) => {
            error!("ffprobe timed out after 40s");
            return Err((StatusCode::GATEWAY_TIMEOUT, "ffprobe timed out".to_string()));
        }
    };

    if !ffprobe_result.status.success() {
        let stderr = String::from_utf8_lossy(&ffprobe_result.stderr);
        let stdout = String::from_utf8_lossy(&ffprobe_result.stdout);
        let status = ffprobe_result.status;
        error!("❌ ffprobe failed. Status: {:?}\nstderr: {}\nstdout: {}", status, stderr, stdout);

        let err_msg = if !stderr.is_empty() { 
            stderr.to_string() 
        } else if !stdout.is_empty() { 
            stdout.to_string() 
        } else {
            format!("Process exited with status {:?}", status)
        };

        let mut active_stream: stream::ActiveModel = stream_obj.into();
        let mut props = active_stream.custom_properties.unwrap().unwrap_or_else(|| json!({}));
        props["stream_stats"] = json!({"reachable": false, "status": "offline"});
        props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        active_stream.custom_properties = Set(Some(props));
        let _ = active_stream.update(&state.db).await;

        return Err((StatusCode::BAD_REQUEST, format!("ffprobe failed: {}", err_msg)));
    }

    let probe_output = String::from_utf8_lossy(&ffprobe_result.stdout);
    let probe_data: Value = match serde_json::from_str(&probe_output) {
        Ok(d) => d,
        Err(e) => {
             error!("ffprobe output JSON parsing failed: {}", e);
             return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse ffprobe output".to_string()));
        }
    };

    let streams = probe_data.get("streams").and_then(|s| s.as_array()).unwrap_or(&vec![]).clone();

    let mut video_codec = None;
    let mut width = None;
    let mut height = None;
    let mut fps = None;
    let mut audio_codec = None;
    let mut channels = None;

    for s in &streams {
        if let Some(codec_type) = s.get("codec_type").and_then(|t| t.as_str()) {
            if codec_type == "video" && video_codec.is_none() {
                video_codec = s.get("codec_name").and_then(|c| c.as_str()).map(String::from);
                width = s.get("width").and_then(|w| w.as_i64());
                height = s.get("height").and_then(|h| h.as_i64());
                fps = s.get("avg_frame_rate").and_then(|f| f.as_str()).map(String::from);
            } else if codec_type == "audio" && audio_codec.is_none() {
                audio_codec = s.get("codec_name").and_then(|c| c.as_str()).map(String::from);
                channels = s.get("channels").and_then(|c| c.as_i64());
            }
        }
    }

    if streams.is_empty() {
        let mut active_stream: stream::ActiveModel = stream_obj.into();
        let mut props = active_stream.custom_properties.unwrap().unwrap_or_else(|| json!({}));
        props["stream_stats"] = json!({"reachable": false, "status": "offline"});
        props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        active_stream.custom_properties = Set(Some(props));
        let _ = active_stream.update(&state.db).await;

        return Err((StatusCode::BAD_REQUEST, "No streams found in ffprobe output".to_string()));
    }

    // 2. Run ffmpeg for bitrate
    info!("🎬 FFmpeg Bitrate Analysis for {}", stream_obj.name);
    let ffmpeg_bin = resolve_ffmpeg();
    let mut ffmpeg_cmd = Command::new(&ffmpeg_bin);
    ffmpeg_cmd.args(&[
        "-t", "10", // Test duration
        "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.3",
        "-i", &stream_url,
        "-c", "copy",
        "-f", "null",
        "-",
    ]);

    let ffmpeg_result = match tokio::time::timeout(Duration::from_secs(40), ffmpeg_cmd.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            error!("ffmpeg failed to start: {}", e);
            // We can still save ffprobe data
            tokio::process::Command::new("echo").arg("dummy").output().await.unwrap()
        }
        Err(_) => {
            error!("ffmpeg timed out");
             tokio::process::Command::new("echo").arg("dummy").output().await.unwrap()
        }
    };

    let mut bitrate = None;
    if ffmpeg_result.status.success() || ffmpeg_result.status.code().unwrap_or(1) != 0 {
        let stderr = String::from_utf8_lossy(&ffmpeg_result.stderr);
        for line in stderr.lines().rev() {
            if line.contains("bitrate=") {
                if let Some(idx) = line.find("bitrate=") {
                    let parts: Vec<&str> = line[idx..].split_whitespace().collect();
                    if parts.len() >= 2 {
                        let bit_str = parts[0].replace("bitrate=", "");
                        if let Ok(b) = bit_str.replace("kbits/s", "").parse::<f64>() {
                            bitrate = Some(b as i64);
                            break;
                        }
                    }
                }
            }
        }
    }

    let stats = json!({
        "reachable": true,
        "video_codec": video_codec,
        "resolution": format!("{}x{}", width.unwrap_or(0), height.unwrap_or(0)),
        "width": width,
        "height": height,
        "source_fps": fps,
        "audio_codec": audio_codec,
        "audio_channels": channels,
        "video_bitrate": bitrate,
        "status": "online"
    });

    let mut active_stream: stream::ActiveModel = stream_obj.clone().into();
    let mut props = active_stream.custom_properties.unwrap().unwrap_or_else(|| json!({}));
    props["stream_stats"] = stats.clone();
    props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
    active_stream.custom_properties = Set(Some(props));

    if let Err(e) = active_stream.update(&state.db).await {
         error!("Failed to update stream stats in DB: {}", e);
         return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to save stats to DB".to_string()));
    }

    // Prepare API response mirroring the DB stream model with updated stats
    let mut response_json = serde_json::to_value(&stream_obj).unwrap();
    if let Some(obj) = response_json.as_object_mut() {
        let mut new_props = stream_obj.custom_properties.clone().unwrap_or_else(|| json!({}));
        new_props["stream_stats"] = stats.clone();
        new_props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        obj.insert("custom_properties".to_string(), new_props);
        
        // Flatten for frontend
        obj.insert("stream_stats".to_string(), stats);
        obj.insert("stream_stats_updated_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
    }

    Ok(response_json)
}

pub async fn test_stream(
    State(state): State<Arc<AppState>>,
    Path(stream_id): Path<i64>,
) -> impl IntoResponse {
    match check_single_stream(&state, stream_id).await {
        Ok(stream_data) => (
            StatusCode::OK,
            Json(json!({ "success": true, "stream": stream_data })),
        ),
        Err((status, message)) => (
            status,
            Json(json!({ "success": false, "message": message })),
        ),
    }
}

#[derive(Deserialize)]
pub struct BulkCheckRequest {
    pub stream_ids: Vec<i64>,
}

pub async fn start_bulk_check(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BulkCheckRequest>,
) -> impl IntoResponse {
    let mut status = state.bulk_check_status.write().await;
    if status.is_running {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "A bulk check is already running"})),
        );
    }
    
    let stream_ids = payload.stream_ids;
    if stream_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "No streams provided"})),
        );
    }

    *status = BulkCheckStatus {
        is_running: true,
        total: stream_ids.len(),
        completed: 0,
        successful: 0,
        failed: 0,
        current_stream_id: None,
        current_stream_name: None,
    };

    let state_clone = state.clone();
    
    tokio::spawn(async move {
        for stream_id in stream_ids {
            let stream_name = {
                if let Ok(Some(s)) = stream::Entity::find_by_id(stream_id).one(&state_clone.db).await {
                    s.name
                } else {
                    "Unknown".to_string()
                }
            };
            
            {
                let mut st = state_clone.bulk_check_status.write().await;
                st.current_stream_id = Some(stream_id);
                st.current_stream_name = Some(stream_name.clone());
            }
            
            match check_single_stream(&state_clone, stream_id).await {
                Ok(_) => {
                    let mut st = state_clone.bulk_check_status.write().await;
                    st.successful += 1;
                    st.completed += 1;
                }
                Err(e) => {
                    error!("Bulk Check Failed for stream {}: {:?}", stream_name, e);
                    let mut st = state_clone.bulk_check_status.write().await;
                    st.failed += 1;
                    st.completed += 1;
                }
            }
        }
        
        let mut st = state_clone.bulk_check_status.write().await;
        st.is_running = false;
        st.current_stream_id = None;
        st.current_stream_name = None;
    });

    (StatusCode::OK, Json(json!({"success": true, "message": "Bulk check started"})))
}

pub async fn get_bulk_check_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let status = state.bulk_check_status.read().await;
    (StatusCode::OK, Json(status.clone()))
}

// ================= SORTING RULES =================

pub async fn list_sorting_rules(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match stream_sorting_rule::Entity::find().all(&state.db).await {
        Ok(rules) => (StatusCode::OK, Json(rules)),
        Err(e) => {
            error!("Failed to fetch sorting rules: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
        }
    }
}

#[derive(Deserialize)]
pub struct CreateRulePayload {
    pub name: String,
    pub priority: i32,
    pub property: String,
    pub operator: String,
    pub value: String,
    pub score_modifier: i32,
}

pub async fn create_sorting_rule(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateRulePayload>,
) -> impl IntoResponse {
    let rule = stream_sorting_rule::ActiveModel {
        name: ActiveValue::Set(payload.name),
        priority: ActiveValue::Set(payload.priority),
        property: ActiveValue::Set(payload.property),
        operator: ActiveValue::Set(payload.operator),
        value: ActiveValue::Set(payload.value),
        score_modifier: ActiveValue::Set(payload.score_modifier),
        ..Default::default()
    };

    match rule.insert(&state.db).await {
        Ok(inserted) => (StatusCode::CREATED, Json(json!({"success": true, "rule": inserted}))),
        Err(e) => {
            error!("Failed to create rule: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"success": false, "message": "Failed to create rule"})))
        }
    }
}

pub async fn update_sorting_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<CreateRulePayload>,
) -> impl IntoResponse {
    let mut rule: stream_sorting_rule::ActiveModel = match stream_sorting_rule::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(r)) => r.into(),
        _ => return (StatusCode::NOT_FOUND, Json(json!({"success": false, "message": "Rule not found"}))),
    };

    rule.name = ActiveValue::Set(payload.name);
    rule.priority = ActiveValue::Set(payload.priority);
    rule.property = ActiveValue::Set(payload.property);
    rule.operator = ActiveValue::Set(payload.operator);
    rule.value = ActiveValue::Set(payload.value);
    rule.score_modifier = ActiveValue::Set(payload.score_modifier);

    match rule.update(&state.db).await {
        Ok(updated) => (StatusCode::OK, Json(json!({"success": true, "rule": updated}))),
        Err(e) => {
            error!("Failed to update rule: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"success": false, "message": "Failed to update rule"})))
        }
    }
}

pub async fn delete_sorting_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match stream_sorting_rule::Entity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => (StatusCode::OK, Json(json!({"success": true}))),
        Err(e) => {
            error!("Failed to delete rule: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"success": false, "message": "Failed to delete rule"})))
        }
    }
}

// ================= SORTING LOGIC =================

#[derive(Deserialize)]
pub struct BulkSortRequest {
    pub channel_ids: Vec<i64>,
}

fn evaluate_rule(rule: &stream_sorting_rule::Model, stream_stats: &Value) -> bool {
    let val = stream_stats.get(&rule.property);
    let target = &rule.value;
    
    match rule.operator.as_str() {
        "==" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() { return s == target; }
                if let Some(i) = v.as_i64() { return i.to_string() == *target; }
            }
            false
        },
        "!=" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() { return s != target; }
                if let Some(i) = v.as_i64() { return i.to_string() != *target; }
            }
            true
        },
        ">=" => {
            if let Some(v) = val {
                if let (Some(i), Ok(t)) = (v.as_i64(), target.parse::<i64>()) { return i >= t; }
            }
            false
        },
        "<=" => {
            if let Some(v) = val {
                if let (Some(i), Ok(t)) = (v.as_i64(), target.parse::<i64>()) { return i <= t; }
            }
            false
        },
        "contains" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() { return s.contains(target); }
            }
            false
        },
        _ => false,
    }
}

pub async fn bulk_sort_streams(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BulkSortRequest>,
) -> impl IntoResponse {
    let rules = match stream_sorting_rule::Entity::find().order_by_asc(stream_sorting_rule::Column::Priority).all(&state.db).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"success": false, "message": "Failed to load rules"}))),
    };

    let mut sorted_channels = 0;

    for channel_id in payload.channel_ids {
        // Find all channel streams
        let channel_streams = match channel_stream::Entity::find()
            .filter(channel_stream::Column::ChannelId.eq(channel_id))
            .all(&state.db).await {
                Ok(cs) => cs,
                Err(_) => continue,
            };

        let mut scored_streams: Vec<(channel_stream::Model, i32)> = Vec::new();

        for cs in channel_streams {
            let mut score = 0;
            // Load the stream
            if let Ok(Some(stream)) = stream::Entity::find_by_id(cs.stream_id).one(&state.db).await {
                if let Some(props) = stream.custom_properties {
                    if let Some(stats) = props.get("stream_stats") {
                        for rule in &rules {
                            if evaluate_rule(rule, stats) {
                                score += rule.score_modifier;
                            }
                        }
                    }
                }
            }
            scored_streams.push((cs, score));
        }

        // Sort descending by score
        scored_streams.sort_by(|a, b| b.1.cmp(&a.1));

        // Update the database with new ordering
        for (index, (cs, _score)) in scored_streams.into_iter().enumerate() {
            let mut active_cs: channel_stream::ActiveModel = cs.into();
            active_cs.order = ActiveValue::Set(index as i32);
            let _ = active_cs.update(&state.db).await;
        }

        sorted_channels += 1;
    }

    (StatusCode::OK, Json(json!({
        "success": true, 
        "message": format!("Successfully sorted {} channels.", sorted_channels)
    })))
}
