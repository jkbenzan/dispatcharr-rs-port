use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::{error, info};

use crate::entities::channel_stream;
use crate::entities::stream;
use crate::entities::stream_sorting_rule;
use crate::AppState;
use futures_util::stream::{self as future_stream, StreamExt};
use sea_orm::{ActiveValue, QueryOrder};
use std::collections::HashMap;
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
        let fname = if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        };
        let sidecar_path = dir.join(fname);
        if sidecar_path.is_file() {
            let p = sidecar_path.to_string_lossy().to_string();
            // Smoke test: must start AND return success
            if let Ok(output) = StdCommand::new(&p).arg("-version").output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    info!("✅ Using sidecar ffprobe: {} (Version: {})", p, version);
                    return p;
                } else {
                    info!(
                        "⚠️  Sidecar ffprobe found but returned error status on -version: {}",
                        p
                    );
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
                    let version = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    info!("✅ Using system ffprobe: {} (Version: {})", c, version);
                    return c.to_string();
                } else {
                    info!(
                        "⚠️  Candidate {} found but returned error status on -version",
                        c
                    );
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
                let version = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                info!("✅ Using sidecar ffmpeg: {} (Version: {})", p, version);
                return p;
            } else {
                info!(
                    "⚠️  Sidecar ffmpeg found but returned error status on -version: {}",
                    p
                );
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
                    let version = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    info!("✅ Using system ffmpeg: {} (Version: {})", c, version);
                    return c.to_string();
                } else {
                    info!(
                        "⚠️  Candidate {} found but returned error status on -version",
                        c
                    );
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
pub struct WorkerStatus {
    pub m3u_account_id: i64,
    pub m3u_account_name: String,
    pub current_stream_name: String,
    pub completed: usize,
    pub total: usize,
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
    pub workers: Vec<WorkerStatus>,
    pub last_results: Vec<Value>,
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
            workers: Vec::new(),
            last_results: Vec::new(),
        }
    }
}

pub async fn check_single_stream(
    state: &Arc<AppState>,
    stream_id: i64,
    test_duration: Option<u32>,
) -> Result<serde_json::Value, (StatusCode, String)> {
    let duration = test_duration.unwrap_or(10);
    let duration_str = duration.to_string();

    let stream_obj = match stream::Entity::find_by_id(stream_id).one(&state.db).await {
        Ok(Some(s)) => s,
        _ => return Err((StatusCode::NOT_FOUND, "Stream not found".to_string())),
    };

    let stream_url = match &stream_obj.url {
        Some(url) => url.clone(),
        None => return Err((StatusCode::BAD_REQUEST, "Stream has no URL".to_string())),
    };

    info!(
        "🔍 Testing Stream: {} (URL: {}, Duration: {}s)",
        stream_obj.name, stream_url, duration
    );

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

    let ffprobe_result =
        match tokio::time::timeout(Duration::from_secs(40), ffprobe_cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                error!("ffprobe failed to start: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("ffprobe failed to start: {}", e),
                ));
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
        error!(
            "❌ ffprobe failed. Status: {:?}\nstderr: {}\nstdout: {}",
            status, stderr, stdout
        );

        let err_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            format!("Process exited with status {:?}", status)
        };

        let mut active_stream: stream::ActiveModel = stream_obj.into();
        let mut props = active_stream
            .custom_properties
            .unwrap()
            .unwrap_or_else(|| json!({}));
        props["stream_stats"] = json!({"reachable": false, "status": "offline"});
        props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        active_stream.custom_properties = Set(Some(props));
        let _ = active_stream.update(&state.db).await;

        return Err((
            StatusCode::BAD_REQUEST,
            format!("ffprobe failed: {}", err_msg),
        ));
    }

    let probe_output = String::from_utf8_lossy(&ffprobe_result.stdout);
    let probe_data: Value = match serde_json::from_str(&probe_output) {
        Ok(d) => d,
        Err(e) => {
            error!("ffprobe output JSON parsing failed: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse ffprobe output".to_string(),
            ));
        }
    };

    let streams = probe_data
        .get("streams")
        .and_then(|s| s.as_array())
        .unwrap_or(&vec![])
        .clone();

    let mut video_codec = None;
    let mut width = None;
    let mut height = None;
    let mut fps = None;
    let mut audio_codec = None;
    let mut channels = None;

    for s in &streams {
        if let Some(codec_type) = s.get("codec_type").and_then(|t| t.as_str()) {
            if codec_type == "video" && video_codec.is_none() {
                video_codec = s
                    .get("codec_name")
                    .and_then(|c| c.as_str())
                    .map(|c| c.to_uppercase());
                width = s.get("width").and_then(|w| w.as_i64());
                height = s.get("height").and_then(|h| h.as_i64());

                // Parse FPS fraction (e.g. "60/1" or "30000/1001")
                if let Some(f_str) = s.get("avg_frame_rate").and_then(|f| f.as_str()) {
                    let parts: Vec<&str> = f_str.split('/').collect();
                    if parts.len() == 2 {
                        let num: f64 = parts[0].parse().unwrap_or(0.0);
                        let den: f64 = parts[1].parse().unwrap_or(1.0);
                        if den > 0.0 {
                            fps = Some(format!("{}", num / den));
                        }
                    } else {
                        fps = Some(f_str.to_string());
                    }
                }
            } else if codec_type == "audio" && audio_codec.is_none() {
                audio_codec = s
                    .get("codec_name")
                    .and_then(|c| c.as_str())
                    .map(|c| c.to_uppercase());
                let channel_count = s.get("channels").and_then(|c| c.as_i64()).unwrap_or(0);
                channels = Some(match channel_count {
                    1 => "mono".to_string(),
                    2 => "stereo".to_string(),
                    6 => "5.1".to_string(),
                    8 => "7.1".to_string(),
                    _ => format!("{} channels", channel_count),
                });
            }
        }
    }

    if streams.is_empty() {
        let mut active_stream: stream::ActiveModel = stream_obj.into();
        let mut props = active_stream
            .custom_properties
            .unwrap()
            .unwrap_or_else(|| json!({}));
        props["stream_stats"] = json!({"reachable": false, "status": "offline"});
        props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        active_stream.custom_properties = Set(Some(props));
        let _ = active_stream.update(&state.db).await;

        return Err((
            StatusCode::BAD_REQUEST,
            "No streams found in ffprobe output".to_string(),
        ));
    }

    // 2. Run ffmpeg for bitrate
    info!("🎬 FFmpeg Bitrate Analysis for {} ({}s)", stream_obj.name, duration);
    let ffmpeg_bin = resolve_ffmpeg();
    let mut ffmpeg_cmd = Command::new(&ffmpeg_bin);
    ffmpeg_cmd.args(&[
        "-t", &duration_str,
        "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.3",
        "-i", &stream_url,
        "-c", "copy",
        "-f", "null",
        "-",
    ]);

    let timeout_secs = (duration + 30) as u64;
    let ffmpeg_result =
        match tokio::time::timeout(Duration::from_secs(timeout_secs), ffmpeg_cmd.output()).await {

            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                error!("ffmpeg failed to start: {}", e);
                // We can still save ffprobe data
                tokio::process::Command::new("echo")
                    .arg("dummy")
                    .output()
                    .await
                    .unwrap()
            }
            Err(_) => {
                error!("ffmpeg timed out");
                tokio::process::Command::new("echo")
                    .arg("dummy")
                    .output()
                    .await
                    .unwrap()
            }
        };

    let mut bitrate: Option<f64> = None;
    if ffmpeg_result.status.success() || ffmpeg_result.status.code().unwrap_or(1) != 0 {
        let stderr = String::from_utf8_lossy(&ffmpeg_result.stderr);
        for line in stderr.lines().rev() {
            if line.contains("bitrate=") {
                if let Some(idx) = line.find("bitrate=") {
                    let parts: Vec<&str> = line[idx..].split_whitespace().collect();
                    if parts.len() >= 2 {
                        let bit_str = parts[0].replace("bitrate=", "");
                        if let Ok(b) = bit_str.replace("kbits/s", "").trim().parse::<f64>() {
                            bitrate = Some(b);
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
    let mut props = active_stream
        .custom_properties
        .unwrap()
        .unwrap_or_else(|| json!({}));
    props["stream_stats"] = stats.clone();
    props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
    active_stream.custom_properties = Set(Some(props));

    if let Err(e) = active_stream.update(&state.db).await {
        error!("Failed to update stream stats in DB: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save stats to DB".to_string(),
        ));
    }

    // Prepare API response mirroring the DB stream model with updated stats
    let mut response_json = serde_json::to_value(&stream_obj).unwrap();
    if let Some(obj) = response_json.as_object_mut() {
        let mut new_props = stream_obj
            .custom_properties
            .clone()
            .unwrap_or_else(|| json!({}));
        new_props["stream_stats"] = stats.clone();
        new_props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        obj.insert("custom_properties".to_string(), new_props);

        // Flatten for frontend
        obj.insert("stream_stats".to_string(), stats);
        obj.insert(
            "stream_stats_updated_at".to_string(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
    }

    Ok(response_json)
}

pub async fn test_stream(
    State(state): State<Arc<AppState>>,
    Path(stream_id): Path<i64>,
) -> impl IntoResponse {
    let settings = crate::settings::get_maintenance_settings(&state.db).await;
    let duration = if settings.extended_test_enabled {
        Some(settings.extended_test_duration_seconds)
    } else {
        None
    };

    match check_single_stream(&state, stream_id, duration).await {
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

    let streams = stream::Entity::find()
        .filter(stream::Column::Id.is_in(payload.stream_ids))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut m3u_groups: HashMap<i64, Vec<stream::Model>> = HashMap::new();
    let mut total_streams = 0;

    for s in streams {
        if let Some(account_id) = s.m3u_account_id {
            m3u_groups
                .entry(account_id)
                .or_insert_with(Vec::new)
                .push(s);
            total_streams += 1;
        }
    }

    if total_streams == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"success": false, "message": "No valid M3U streams provided for checking"}),
            ),
        );
    }

    // Get the parallel providers setting
    let settings = crate::entities::core_settings::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();

    let mut max_concurrent = 1;
    for s in settings {
        if s.key == "stream_settings" {
            if let Some(v) = s.value.get("stream_checker_parallel_providers") {
                if let Some(num) = v.as_i64() {
                    max_concurrent = num as usize;
                }
            }
        }
    }
    if max_concurrent < 1 {
        max_concurrent = 1;
    }

    // Fetch account names
    let mut account_names = HashMap::new();
    let accounts = crate::entities::m3u_account::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    for acc in accounts {
        account_names.insert(acc.id, acc.name);
    }

    *status = BulkCheckStatus {
        is_running: true,
        total: total_streams,
        completed: 0,
        successful: 0,
        failed: 0,
        current_stream_id: None,
        current_stream_name: None,
        workers: Vec::new(),
        last_results: Vec::new(),
    };
    drop(status);

    let state_clone = state.clone();

    tokio::spawn(async move {
        // Build the stream of provider groups
        let groups_stream = future_stream::iter(m3u_groups.into_iter());

        groups_stream
            .for_each_concurrent(max_concurrent, |(account_id, streams)| {
                let state_c = state_clone.clone();
                let acc_name = account_names
                    .get(&account_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown Provider".to_string());
                let total_in_group = streams.len();

                async move {
                    // Register this worker
                    {
                        let mut st = state_c.bulk_check_status.write().await;
                        st.workers.push(WorkerStatus {
                            m3u_account_id: account_id,
                            m3u_account_name: acc_name.clone(),
                            current_stream_name: String::new(),
                            completed: 0,
                            total: total_in_group,
                        });
                    }

                    for (idx, stream_obj) in streams.into_iter().enumerate() {
                        {
                            let mut st = state_c.bulk_check_status.write().await;
                            if let Some(w) = st
                                .workers
                                .iter_mut()
                                .find(|w| w.m3u_account_id == account_id)
                            {
                                w.current_stream_name = stream_obj.name.clone();
                                w.completed = idx;
                            }
                        }

                        let res = check_single_stream(&state_c, stream_obj.id, None).await;

                        {
                            let mut st = state_c.bulk_check_status.write().await;
                            st.completed += 1;
                            match res {
                                Ok(stats) => {
                                    st.successful += 1;
                                    let mut result_obj = stats.clone();
                                    result_obj["name"] = json!(stream_obj.name);
                                    result_obj["id"] = json!(stream_obj.id);
                                    st.last_results.push(result_obj);
                                    if st.last_results.len() > 10 {
                                        st.last_results.remove(0);
                                    }
                                }
                                Err(_) => {
                                    st.failed += 1;
                                    let result_obj = json!({
                                        "name": stream_obj.name,
                                        "id": stream_obj.id,
                                        "stream_stats": { "reachable": false }
                                    });
                                    st.last_results.push(result_obj);
                                    if st.last_results.len() > 10 {
                                        st.last_results.remove(0);
                                    }
                                }
                            }
                        }
                    }

                    // Mark worker completed
                    {
                        let mut st = state_c.bulk_check_status.write().await;
                        if let Some(w) = st
                            .workers
                            .iter_mut()
                            .find(|w| w.m3u_account_id == account_id)
                        {
                            w.completed = total_in_group;
                            w.current_stream_name = "Finished".to_string();
                        }
                    }
                }
            })
            .await;

        let mut st = state_clone.bulk_check_status.write().await;
        st.is_running = false;
    });

    (
        StatusCode::OK,
        Json(json!({"success": true, "message": "Bulk check started"})),
    )
}

pub async fn get_bulk_check_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = state.bulk_check_status.read().await;
    (StatusCode::OK, Json(status.clone()))
}

// ================= SORTING RULES =================

pub async fn list_sorting_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
        Ok(inserted) => (
            StatusCode::CREATED,
            Json(json!({"success": true, "rule": inserted})),
        ),
        Err(e) => {
            error!("Failed to create rule: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": "Failed to create rule"})),
            )
        }
    }
}

pub async fn update_sorting_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<CreateRulePayload>,
) -> impl IntoResponse {
    let mut rule: stream_sorting_rule::ActiveModel =
        match stream_sorting_rule::Entity::find_by_id(id)
            .one(&state.db)
            .await
        {
            Ok(Some(r)) => r.into(),
            _ => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"success": false, "message": "Rule not found"})),
                )
            }
        };

    rule.name = ActiveValue::Set(payload.name);
    rule.priority = ActiveValue::Set(payload.priority);
    rule.property = ActiveValue::Set(payload.property);
    rule.operator = ActiveValue::Set(payload.operator);
    rule.value = ActiveValue::Set(payload.value);
    rule.score_modifier = ActiveValue::Set(payload.score_modifier);

    match rule.update(&state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(json!({"success": true, "rule": updated})),
        ),
        Err(e) => {
            error!("Failed to update rule: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": "Failed to update rule"})),
            )
        }
    }
}

pub async fn delete_sorting_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match stream_sorting_rule::Entity::delete_by_id(id)
        .exec(&state.db)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({"success": true}))),
        Err(e) => {
            error!("Failed to delete rule: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": "Failed to delete rule"})),
            )
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
                if let Some(s) = v.as_str() {
                    return s == target;
                }
                if let Some(i) = v.as_i64() {
                    return i.to_string() == *target;
                }
            }
            false
        }
        "!=" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() {
                    return s != target;
                }
                if let Some(i) = v.as_i64() {
                    return i.to_string() != *target;
                }
            }
            true
        }
        ">=" => {
            if let Some(v) = val {
                if let (Some(i), Ok(t)) = (v.as_i64(), target.parse::<i64>()) {
                    return i >= t;
                }
            }
            false
        }
        "<=" => {
            if let Some(v) = val {
                if let (Some(i), Ok(t)) = (v.as_i64(), target.parse::<i64>()) {
                    return i <= t;
                }
            }
            false
        }
        "contains" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() {
                    return s.contains(target);
                }
            }
            false
        }
        _ => false,
    }
}

pub async fn internal_bulk_sort_streams(
    state: &Arc<AppState>,
    payload: BulkSortRequest,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let rules = stream_sorting_rule::Entity::find()
        .order_by_asc(stream_sorting_rule::Column::Priority)
        .all(&state.db)
        .await?;

    let mut sorted_channels = 0;

    for channel_id in payload.channel_ids {
        // Find all channel streams
        let channel_streams = channel_stream::Entity::find()
            .filter(channel_stream::Column::ChannelId.eq(channel_id))
            .all(&state.db)
            .await?;

        let mut scored_streams: Vec<(channel_stream::Model, i32)> = Vec::new();

        for cs in channel_streams {
            let mut score = 0;
            // Load the stream
            if let Ok(Some(stream)) = stream::Entity::find_by_id(cs.stream_id)
                .one(&state.db)
                .await
            {
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

    Ok(sorted_channels)
}

pub async fn bulk_sort_streams(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BulkSortRequest>,
) -> impl IntoResponse {
    match internal_bulk_sort_streams(&state, payload).await {
        Ok(count) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": format!("Successfully sorted {} channels.", count)
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": "Failed to sort channels"})),
        ),
    }
}

pub async fn run_automated_maintenance(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use chrono::{DateTime, Utc, Duration};


    let settings = crate::settings::get_maintenance_settings(&state.db).await;
    
    // Find streams that haven't been checked in N days
    let streams = stream::Entity::find().all(&state.db).await?;
    let mut stale_streams = Vec::new();
    let threshold = Utc::now() - Duration::days(settings.stream_check_frequency_days as i64);

    for s in streams {
        let is_stale = if let Some(props) = &s.custom_properties {
            if let Some(updated_at_str) = props.get("stream_stats_updated_at").and_then(|v| v.as_str()) {
                if let Ok(updated_at) = DateTime::parse_from_rfc3339(updated_at_str) {
                    updated_at.with_timezone(&Utc) < threshold
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if is_stale {
            stale_streams.push(s);
        }
        if stale_streams.len() >= settings.batch_size {
            break;
        }
    }

    if stale_streams.is_empty() {
        return Ok(());
    }

    info!("[Maintenance] Found {} stale streams to check.", stale_streams.len());

    
    let mut affected_channels = std::collections::HashSet::new();
    let mut success_count = 0;
    let mut failure_count = 0;

    for s in stale_streams {

        let stream_id = s.id;
        // Run check (always use default short test for automated background checks)
        match check_single_stream(&state, stream_id, None).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                failure_count += 1;
                error!("[Maintenance] Check failed for stream {}: {:?}", stream_id, e);
            }
        }


        // Find affected channels
        let cs_links = channel_stream::Entity::find()
            .filter(channel_stream::Column::StreamId.eq(stream_id))
            .all(&state.db)
            .await?;
        for link in cs_links {
            affected_channels.insert(link.channel_id);
        }
    }

    // Update Telemetry
    {
        let mut telemetry = state.background_telemetry.write().await;
        telemetry.stream_check.last_run_at = Some(Utc::now());
        telemetry.stream_check.total_processed += success_count + failure_count;
        telemetry.stream_check.success_count += success_count;
        telemetry.stream_check.failure_count += failure_count;
    }

    // Trigger Bulk Sort for affected channels
    if !affected_channels.is_empty() {
        info!("[Maintenance] Re-sorting {} affected channels.", affected_channels.len());
        let payload = BulkSortRequest {
            channel_ids: affected_channels.into_iter().collect(),
        };
        let _ = internal_bulk_sort_streams(&state, payload).await;
    }

    Ok(())
}
