use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::{error, info, warn};

use crate::entities::channel_stream;
use crate::entities::stream;
use crate::entities::stream_sorting_rule;
use crate::AppState;
use futures_util::stream::StreamExt;
use sea_orm::ActiveValue;
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
    pub current_stream_id: Option<i64>,
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
        "-headers", "User-Agent: VLC/3.0.0\r\n",
        "-print_format", "json",
        "-show_streams",
        "-analyzeduration", "10000000",
        "-probesize", "10000000",
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
    let mut audio_channel_count = None;


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
                let mut frame_rates_to_try = vec![];
                if let Some(f) = s.get("avg_frame_rate").and_then(|f| f.as_str()) {
                    frame_rates_to_try.push(f);
                }
                if let Some(f) = s.get("r_frame_rate").and_then(|f| f.as_str()) {
                    frame_rates_to_try.push(f);
                }

                for f_str in frame_rates_to_try {
                    if f_str == "0/0" {
                        continue;
                    }
                    let parts: Vec<&str> = f_str.split('/').collect();
                    if parts.len() == 2 {
                        let num: f64 = parts[0].parse().unwrap_or(0.0);
                        let den: f64 = parts[1].parse().unwrap_or(1.0);
                        if den > 0.0 && num > 0.0 {
                            fps = Some(format!("{:.2}", num / den));
                            break;
                        }
                    } else if let Ok(val) = f_str.parse::<f64>() {
                        if val > 0.0 {
                            fps = Some(format!("{:.2}", val));
                            break;
                        }
                    }
                }
            } else if codec_type == "audio" && audio_codec.is_none() {
                audio_codec = s
                    .get("codec_name")
                    .and_then(|c| c.as_str())
                    .map(|c| c.to_uppercase());
                let count = s.get("channels").and_then(|c| c.as_i64()).unwrap_or(0);
                audio_channel_count = Some(count);

                channels = Some(match count {
                    1 => "mono".to_string(),
                    2 => "stereo".to_string(),
                    6 => "5.1".to_string(),
                    8 => "7.1".to_string(),
                    _ => format!("{} channels", count),
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
        "-headers", "User-Agent: VLC/3.0.0\r\n",
        "-i", &stream_url,
        "-vf", "freezedetect=n=0.003:d=2,blackdetect=d=2:pix_th=0.1",
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
    let mut is_frozen = false;
    let mut is_black = false;

    if ffmpeg_result.status.success() || ffmpeg_result.status.code().unwrap_or(1) != 0 {
        let stderr = String::from_utf8_lossy(&ffmpeg_result.stderr);
        for line in stderr.lines() {
            if line.contains("freeze_start:") {
                is_frozen = true;
            }
            if line.contains("black_start:") {
                is_black = true;
            }
            if line.contains("bitrate=") {
                if let Some(idx) = line.find("bitrate=") {
                    let bitrate_part = &line[idx + 8..].trim();
                    let parts: Vec<&str> = bitrate_part.split_whitespace().collect();
                    if let Some(val_str) = parts.get(0) {
                        if let Ok(b) = val_str.replace("kbits/s", "").trim().parse::<f64>() {
                            bitrate = Some(b);
                        }
                    }
                }
            }
        }
    }

    let status = if is_frozen {
        "frozen"
    } else if is_black {
        "black_screen"
    } else {
        "online"
    };

    let stats = json!({
        "reachable": true,
        "video_codec": video_codec,
        "resolution": format!("{}x{}", width.unwrap_or(0), height.unwrap_or(0)),
        "width": width,
        "height": height,
        "fps": fps,
        "audio_codec": audio_codec,
        "audio_channels": channels,
        "audio_channel_count": audio_channel_count,

        "bitrate": bitrate,
        "status": status,
        "issues": {
            "frozen": is_frozen,
            "black_screen": is_black
        }
    });

    info!("✅ Stream Test Result for {}: status={}, resolution={}, fps={}, bitrate={:?}", 
        stream_obj.name, status, stats["resolution"], stats["fps"], bitrate);

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

    // Fetch stream name for the activity event
    let stream_name = stream::Entity::find_by_id(stream_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|s| s.name.clone())
        .unwrap_or_else(|| format!("Stream #{}", stream_id));

    match check_single_stream(&state, stream_id, duration).await {
        Ok(stream_data) => {
            let _ = update_stream_health(
                &state.db,
                stream_id,
                true,
                settings.auto_prune_failed_count,
            )
            .await;

            // Log single stream check success to Activity (Option C: is_single flag)
            let _ = crate::events::record_event(
                &state.db,
                "stream_check_completed",
                Some(stream_name.clone()),
                json!({
                    "status": "success",
                    "event": "Stream Check",
                    "stream_id": stream_id,
                    "stream_name": stream_name,
                    "reachable": true,
                    "is_single": true
                }),
            ).await;

            (
                StatusCode::OK,
                Json(json!({ "success": true, "stream": stream_data })),
            )
        }
        Err((status, message)) => {
            let _ = update_stream_health(
                &state.db,
                stream_id,
                false,
                settings.auto_prune_failed_count,
            )
            .await;

            // Log single stream check failure to Activity
            let _ = crate::events::record_event(
                &state.db,
                "stream_check_completed",
                Some(stream_name.clone()),
                json!({
                    "status": "error",
                    "event": "Stream Check",
                    "stream_id": stream_id,
                    "stream_name": stream_name,
                    "reachable": false,
                    "is_single": true,
                    "message": message
                }),
            ).await;

            (
                status,
                Json(json!({ "success": false, "message": message })),
            )
        }
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

    let m_settings = crate::settings::get_maintenance_settings(&state.db).await;
    let duration = if m_settings.extended_test_enabled { Some(m_settings.extended_test_duration_seconds) } else { None };

    // Fetch account names
    let mut account_names = HashMap::new();
    let accounts = crate::entities::m3u_account::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    for acc in accounts {
        account_names.insert(acc.id, acc.name);
    }

    // Clear the cancellation flag before starting a new check
    state.bulk_check_cancelled.store(false, std::sync::atomic::Ordering::SeqCst);

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

    // Log bulk check started event for the Activity page
    let _ = crate::events::record_event(
        &state.db,
        "bulk_check_started",
        None,
        json!({
            "status": "info",
            "event": "Bulk Stream Check Started",
            "total_streams": total_streams,
            "providers": m3u_groups.len()
        }),
    ).await;

    let state_clone = state.clone();

    tokio::spawn(async move {
        // Build the stream of provider groups
        let groups_stream = futures_util::stream::iter(m3u_groups.into_iter());

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
                            current_stream_id: None,
                            current_stream_name: String::new(),
                            completed: 0,
                            total: total_in_group,
                        });
                    }

                    for (idx, stream_obj) in streams.into_iter().enumerate() {
                        // Check cancellation flag before each stream test
                        if state_c.bulk_check_cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                            info!("🛑 Bulk check cancelled — stopping worker for provider {}", acc_name);
                            break;
                        }

                        {
                            let mut st = state_c.bulk_check_status.write().await;
                            if let Some(w) = st
                                .workers
                                .iter_mut()
                                .find(|w| w.m3u_account_id == account_id)
                            {
                                w.current_stream_id = Some(stream_obj.id);
                                w.current_stream_name = stream_obj.name.clone();
                                w.completed = idx;
                            }
                        }

                        let res = check_single_stream(&state_c, stream_obj.id, duration).await;
                        let health_succeeded = res.is_ok();
                        let _ = update_stream_health(
                            &state_c.db,
                            stream_obj.id,
                            health_succeeded,
                            m_settings.auto_prune_failed_count,
                        )
                        .await;

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
                            w.current_stream_id = None;
                            w.current_stream_name = "Finished".to_string();
                        }
                    }
                }
            })
            .await;

        // Capture final stats before marking as done
        let mut st = state_clone.bulk_check_status.write().await;
        let was_cancelled = state_clone.bulk_check_cancelled.load(std::sync::atomic::Ordering::SeqCst);
        let final_total = st.total;
        let final_successful = st.successful;
        let final_failed = st.failed;
        st.is_running = false;
        drop(st);

        // Log bulk check completed event for the Activity page
        let _ = crate::events::record_event(
            &state_clone.db,
            "bulk_check_completed",
            None,
            json!({
                "status": if final_failed == 0 && !was_cancelled { "success" } else { "warning" },
                "event": "Bulk Stream Check Completed",
                "total": final_total,
                "successful": final_successful,
                "failed": final_failed,
                "cancelled": was_cancelled
            }),
        ).await;
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

/// Cancel a running bulk check.
/// Sets the cooperative cancellation flag so workers stop before the next stream.
/// The current in-progress ffprobe/ffmpeg call will complete, but no new streams
/// will be started.
pub async fn cancel_bulk_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = state.bulk_check_status.read().await;
    if !status.is_running {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "No bulk check is currently running"})),
        );
    }
    drop(status);

    // Set the cancellation flag — workers will see this before testing the next stream
    state.bulk_check_cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    info!("🛑 Bulk check cancellation requested");

    // Log cancellation event for the Activity page
    let _ = crate::events::record_event(
        &state.db,
        "bulk_check_cancelled",
        None,
        json!({ "status": "warning", "event": "Bulk Stream Check Cancelled" }),
    ).await;

    (
        StatusCode::OK,
        Json(json!({"success": true, "message": "Bulk check cancellation requested"})),
    )
}

// ================= SORTING RULES =================

pub async fn list_sorting_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match stream_sorting_rule::Entity::find().all(&state.db).await {
        Ok(rules) => (StatusCode::OK, Json(rules)),
        Err(e) => {
            warn!(
                "Sorting rules are unavailable; returning an empty rule list so built-in stream scoring can continue: {}",
                e
            );
            (StatusCode::OK, Json(vec![]))
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

const SORTING_RULE_PROPERTIES: &[&str] = &[
    "status",
    "reachable",
    "height",
    "width",
    "resolution",
    "resolution_height",
    "resolution_width",
    "fps",
    "bitrate",
    "video_codec",
    "audio_codec",
    "audio_channels",
    "consecutive_failures",
];

const SORTING_RULE_OPERATORS: &[&str] = &["==", "!=", ">=", "<=", "contains"];

struct NormalizedRulePayload {
    name: String,
    priority: i32,
    property: String,
    operator: String,
    value: String,
    score_modifier: i32,
}

fn normalize_sorting_rule_payload(
    payload: CreateRulePayload,
) -> Result<NormalizedRulePayload, String> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err("Rule name is required".to_string());
    }

    let property = payload.property.trim().to_string();
    if !SORTING_RULE_PROPERTIES.contains(&property.as_str()) {
        return Err(format!("Unsupported sorting rule property: {}", property));
    }

    let operator = payload.operator.trim().to_string();
    if !SORTING_RULE_OPERATORS.contains(&operator.as_str()) {
        return Err(format!("Unsupported sorting rule operator: {}", operator));
    }

    let value = payload.value.trim().to_string();
    if value.is_empty() {
        return Err("Rule value is required".to_string());
    }

    // Numeric comparisons only make sense when the target value is numeric.
    // Catching this at save time keeps bad rules from silently never matching.
    if matches!(operator.as_str(), ">=" | "<=") && value.parse::<f64>().is_err() {
        return Err("Numeric operators require a numeric value".to_string());
    }

    Ok(NormalizedRulePayload {
        name,
        priority: payload.priority.max(0),
        property,
        operator,
        value,
        score_modifier: payload.score_modifier,
    })
}

async fn next_sorting_rule_id(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr> {
    // Assign ids explicitly so inserts do not depend on sequence permissions.
    // Some deployments granted table access after creation but missed USAGE on
    // stream_sorting_rule_id_seq, which made POST fail even though SELECT worked.
    let latest = stream_sorting_rule::Entity::find()
        .order_by_desc(stream_sorting_rule::Column::Id)
        .one(db)
        .await?;

    Ok(latest.map(|rule| rule.id + 1).unwrap_or(1))
}

pub async fn create_sorting_rule(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateRulePayload>,
) -> impl IntoResponse {
    let payload = match normalize_sorting_rule_payload(payload) {
        Ok(payload) => payload,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "message": message})),
            )
        }
    };

    let next_id = match next_sorting_rule_id(&state.db).await {
        Ok(next_id) => next_id,
        Err(e) => {
            error!("Failed to allocate sorting rule id: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": "Failed to allocate sorting rule id"
                })),
            );
        }
    };

    let rule = stream_sorting_rule::ActiveModel {
        id: ActiveValue::Set(next_id),
        name: ActiveValue::Set(payload.name),
        priority: ActiveValue::Set(payload.priority),
        property: ActiveValue::Set(payload.property),
        operator: ActiveValue::Set(payload.operator),
        value: ActiveValue::Set(payload.value),
        score_modifier: ActiveValue::Set(payload.score_modifier),
        ..Default::default()
    };

    match rule.insert(&state.db).await {
        Ok(inserted) => {
            // Log sorting rule creation for the Activity page
            let _ = crate::events::record_event(
                &state.db,
                "sorting_rule_created",
                None,
                json!({
                    "status": "success",
                    "event": "Sorting Rule Created",
                    "rule_id": inserted.id,
                    "rule_name": inserted.name,
                    "property": inserted.property,
                    "operator": inserted.operator
                }),
            ).await;
            (
                StatusCode::CREATED,
                Json(json!({"success": true, "rule": inserted})),
            )
        }
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
    let payload = match normalize_sorting_rule_payload(payload) {
        Ok(payload) => payload,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "message": message})),
            )
        }
    };

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
        Ok(updated) => {
            // Log sorting rule update for the Activity page
            let _ = crate::events::record_event(
                &state.db,
                "sorting_rule_updated",
                None,
                json!({
                    "status": "success",
                    "event": "Sorting Rule Updated",
                    "rule_id": id,
                    "rule_name": updated.name
                }),
            ).await;
            (
                StatusCode::OK,
                Json(json!({"success": true, "rule": updated})),
            )
        }
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
        Ok(_) => {
            // Log sorting rule deletion for the Activity page
            let _ = crate::events::record_event(
                &state.db,
                "sorting_rule_deleted",
                None,
                json!({
                    "status": "warning",
                    "event": "Sorting Rule Deleted",
                    "rule_id": id
                }),
            ).await;
            (StatusCode::OK, Json(json!({"success": true})))
        }
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
    let val = stream_stat_value(stream_stats, &rule.property);
    let target = &rule.value;

    match rule.operator.as_str() {
        "==" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() {
                    if let (Ok(f1), Ok(f2)) = (s.parse::<f64>(), target.parse::<f64>()) {
                        return (f1 - f2).abs() < 0.01;
                    }
                    return s == target;
                }
                if let Some(i) = v.as_i64() {
                    if let Ok(t) = target.parse::<i64>() {
                        return i == t;
                    }
                    return i.to_string() == *target;
                }
                if let Some(f) = v.as_f64() {
                    if let Ok(t) = target.parse::<f64>() {
                        return (f - t).abs() < 0.01;
                    }
                }
                if let Some(b) = v.as_bool() {
                    return b.to_string() == *target;
                }
            }
            false
        }
        "!=" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() {
                    if let (Ok(f1), Ok(f2)) = (s.parse::<f64>(), target.parse::<f64>()) {
                        return (f1 - f2).abs() >= 0.01;
                    }
                    return s != target;
                }
                if let Some(i) = v.as_i64() {
                    if let Ok(t) = target.parse::<i64>() {
                        return i != t;
                    }
                    return i.to_string() != *target;
                }
                if let Some(f) = v.as_f64() {
                    if let Ok(t) = target.parse::<f64>() {
                        return (f - t).abs() >= 0.01;
                    }
                }
                if let Some(b) = v.as_bool() {
                    return b.to_string() != *target;
                }
            }
            true
        }
        ">=" => {
            if let Some(v) = val {
                let current_val = v.as_f64()
                    .or_else(|| v.as_i64().map(|i| i as f64))
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()));
                if let (Some(f), Ok(t)) = (current_val, target.parse::<f64>()) {
                    return f >= t;
                }
            }
            false
        }
        "<=" => {
            if let Some(v) = val {
                let current_val = v.as_f64()
                    .or_else(|| v.as_i64().map(|i| i as f64))
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()));
                if let (Some(f), Ok(t)) = (current_val, target.parse::<f64>()) {
                    return f <= t;
                }
            }
            false
        }
        "contains" => {
            if let Some(v) = val {
                if let Some(s) = v.as_str() {
                    return s.to_lowercase().contains(&target.to_lowercase());
                }
            }
            false
        }
        _ => false,
    }
}

fn stream_stat_value<'a>(stream_stats: &'a Value, property: &str) -> Option<&'a Value> {
    match property {
        // Backward-compatible aliases for rules created before ffprobe stats
        // stored width and height as first-class numeric fields.
        "resolution_width" => stream_stats.get("width").or_else(|| stream_stats.get(property)),
        "resolution_height" => stream_stats.get("height").or_else(|| stream_stats.get(property)),
        _ => stream_stats.get(property),
    }
}

fn stream_stat_f64(stream_stats: &Value, property: &str) -> Option<f64> {
    stream_stat_value(stream_stats, property)
        .and_then(|v| v.as_f64()
            .or_else(|| v.as_i64().map(|i| i as f64))
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
}

fn stream_stat_i64(stream_stats: &Value, property: &str) -> Option<i64> {
    stream_stat_value(stream_stats, property)
        .and_then(|v| v.as_i64()
            .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            .or_else(|| v.as_f64().map(|f| f.round() as i64))
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok())))
}

fn stream_stat_bool(stream_stats: &Value, property: &str) -> Option<bool> {
    stream_stat_value(stream_stats, property)
        .and_then(|v| v.as_bool()
            .or_else(|| v.as_str().and_then(|s| s.parse::<bool>().ok())))
}

fn stream_stat_str<'a>(stream_stats: &'a Value, property: &str) -> Option<&'a str> {
    stream_stat_value(stream_stats, property).and_then(|v| v.as_str())
}

fn built_in_stream_score(stream_stats: Option<&Value>) -> i32 {
    let Some(stats) = stream_stats else {
        // Untested streams should sort behind confirmed-good streams, but
        // ahead of known-dead streams so new imports still get a fair chance.
        return -10_000;
    };

    let mut score = 0;
    let reachable = stream_stat_bool(stats, "reachable").unwrap_or(false);
    let status = stream_stat_str(stats, "status")
        .unwrap_or_default()
        .to_ascii_lowercase();

    if reachable && status == "online" {
        score += 100_000;
    } else if reachable {
        score += 60_000;
    } else {
        score -= 100_000;
    }

    let failures = stream_stat_i64(stats, "consecutive_failures").unwrap_or(0).clamp(0, 20);
    score -= (failures as i32) * 5_000;

    if stream_stat_bool(&stats["issues"], "frozen").unwrap_or(false) {
        score -= 30_000;
    }
    if stream_stat_bool(&stats["issues"], "black_screen").unwrap_or(false) {
        score -= 30_000;
    }

    // Quality: height is the most useful signal, followed by FPS and bitrate.
    // These weights intentionally stay below the online/offline boundary.
    let height = stream_stat_i64(stats, "height")
        .or_else(|| stream_stat_i64(stats, "resolution_height"))
        .unwrap_or(0)
        .clamp(0, 4320);
    score += (height as i32) * 20;

    let fps = stream_stat_f64(stats, "fps").unwrap_or(0.0).clamp(0.0, 240.0);
    score += (fps * 100.0).round() as i32;

    let bitrate = stream_stat_f64(stats, "bitrate").unwrap_or(0.0).clamp(0.0, 200_000.0);
    if bitrate > 0.0 {
        score += (bitrate / 10.0).round() as i32;
    }

    match stream_stat_str(stats, "video_codec")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "hevc" | "h265" | "h.265" => score += 750,
        "h264" | "h.264" | "avc" => score += 500,
        _ => {}
    }

    score
}

pub async fn internal_bulk_sort_streams(
    state: &Arc<AppState>,
    payload: BulkSortRequest,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let rules = match stream_sorting_rule::Entity::find()
        .order_by_asc(stream_sorting_rule::Column::Priority)
        .all(&state.db)
        .await
    {
        Ok(rules) => rules,
        Err(e) => {
            warn!(
                "Sorting rules are unavailable; using built-in reliability and quality scoring only: {}",
                e
            );
            Vec::new()
        }
    };

    let mut sorted_channels = 0;

    for channel_id in payload.channel_ids {
        // Find all channel streams
        let channel_streams = channel_stream::Entity::find()
            .filter(channel_stream::Column::ChannelId.eq(channel_id))
            .order_by_asc(channel_stream::Column::Order)
            .order_by_asc(channel_stream::Column::Id)
            .all(&state.db)
            .await?;

        let mut scored_streams: Vec<(channel_stream::Model, i32)> = Vec::new();

        for cs in channel_streams {
            let mut score = 0;
            // Load the stream and its related account
            use crate::entities::m3u_account;
            if let Ok(Some((stream, account_opt))) = stream::Entity::find_by_id(cs.stream_id)
                .find_also_related(m3u_account::Entity)
                .one(&state.db)
                .await
            {
                // Provider priority stays as a manual bias, but the built-in
                // health/quality score is deliberately much larger so
                // reliability and stream quality win by default.
                if let Some(account) = account_opt {
                    score += account.priority;
                }

                let stream_stats = stream
                    .custom_properties
                    .as_ref()
                    .and_then(|props| props.get("stream_stats"));
                score += built_in_stream_score(stream_stats);

                if let Some(stats) = stream_stats {
                    for rule in &rules {
                        if evaluate_rule(rule, stats) {
                            score += rule.score_modifier;
                        }
                    }
                }
            }
            scored_streams.push((cs, score));
        }

        // Sort descending by score, then preserve existing order for exact
        // ties so repeated sort runs are stable and easy to reason about.
        scored_streams.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.order.cmp(&b.0.order))
                .then_with(|| a.0.id.cmp(&b.0.id))
        });

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

pub async fn update_stream_health(
    db: &DatabaseConnection,
    stream_id: i64,
    is_success: bool,
    threshold: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(current_stream) = stream::Entity::find_by_id(stream_id).one(db).await? else {
        return Ok(());
    };

    let mut active: stream::ActiveModel = current_stream.clone().into();
    let mut props = current_stream
        .custom_properties
        .clone()
        .unwrap_or_else(|| json!({}));
    
    let mut stats = props.get("stream_stats").cloned().unwrap_or_else(|| json!({}));
    let mut failure_count = stats.get("consecutive_failures").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

    if is_success {
        failure_count = 0;
    } else {
        failure_count += 1;
        if threshold > 0 && failure_count >= threshold {
            info!(
                "[Maintenance] Pruning stream {}: reached {} consecutive failures.",
                current_stream.id, failure_count
            );
            active.is_stale = sea_orm::Set(true);
        }
    }

    if let Some(obj) = stats.as_object_mut() {
        obj.insert("consecutive_failures".to_string(), json!(failure_count));
    }
    
    if let Some(obj) = props.as_object_mut() {
        obj.insert("stream_stats".to_string(), stats);
    }

    active.custom_properties = sea_orm::Set(Some(props));
    active.update(db).await?;
    Ok(())
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
        let duration = if settings.extended_test_enabled { Some(settings.extended_test_duration_seconds) } else { None };

        match check_single_stream(&state, stream_id, duration).await {
            Ok(_) => {
                success_count += 1;
                let _ = update_stream_health(&state.db, s.id, true, settings.auto_prune_failed_count).await;
            }
            Err(e) => {
                failure_count += 1;
                error!("[Maintenance] Check failed for stream {}: {:?}", stream_id, e);
                let _ = update_stream_health(&state.db, s.id, false, settings.auto_prune_failed_count).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_evaluate_rule_numeric() {
        let rule = stream_sorting_rule::Model {
            id: 1,
            name: "Test rule".to_string(),
            priority: 1,
            property: "resolution_height".to_string(),
            operator: "==".to_string(),
            value: "1080".to_string(),
            score_modifier: 10,
        };

        let stats_match = json!({"resolution_height": 1080});
        let stats_mismatch = json!({"resolution_height": 720});

        assert!(evaluate_rule(&rule, &stats_match));
        assert!(!evaluate_rule(&rule, &stats_mismatch));
    }

    #[test]
    fn test_evaluate_rule_uses_resolution_height_alias() {
        let rule = stream_sorting_rule::Model {
            id: 1,
            name: "Height rule".to_string(),
            priority: 1,
            property: "resolution_height".to_string(),
            operator: ">=".to_string(),
            value: "1080".to_string(),
            score_modifier: 10,
        };

        let stats_match = json!({"height": 1080, "resolution": "1920x1080"});
        let stats_mismatch = json!({"height": 720, "resolution": "1280x720"});

        assert!(evaluate_rule(&rule, &stats_match));
        assert!(!evaluate_rule(&rule, &stats_mismatch));
    }

    #[test]
    fn test_built_in_score_prioritizes_reliable_stream_over_quality() {
        let online_720 = json!({
            "reachable": true,
            "status": "online",
            "height": 720,
            "fps": "59.94",
            "bitrate": 0.0,
            "consecutive_failures": 0
        });
        let offline_4k = json!({
            "reachable": false,
            "status": "offline",
            "height": 2160,
            "fps": "60.00",
            "bitrate": 25000.0,
            "consecutive_failures": 1
        });

        assert!(built_in_stream_score(Some(&online_720)) > built_in_stream_score(Some(&offline_4k)));
    }

    #[test]
    fn test_built_in_score_prefers_higher_resolution_and_fps_when_reliable() {
        let online_720 = json!({
            "reachable": true,
            "status": "online",
            "height": 720,
            "fps": "29.97",
            "bitrate": 3500.0,
            "consecutive_failures": 0
        });
        let online_1080 = json!({
            "reachable": true,
            "status": "online",
            "height": 1080,
            "fps": "59.94",
            "bitrate": 8000.0,
            "consecutive_failures": 0
        });

        assert!(built_in_stream_score(Some(&online_1080)) > built_in_stream_score(Some(&online_720)));
    }

    #[test]
    fn test_built_in_score_penalizes_failures_and_bad_video_issues() {
        let clean = json!({
            "reachable": true,
            "status": "online",
            "height": 1080,
            "fps": "59.94",
            "bitrate": 8000.0,
            "consecutive_failures": 0,
            "issues": { "frozen": false, "black_screen": false }
        });
        let bad = json!({
            "reachable": true,
            "status": "online",
            "height": 1080,
            "fps": "59.94",
            "bitrate": 8000.0,
            "consecutive_failures": 3,
            "issues": { "frozen": true, "black_screen": false }
        });

        assert!(built_in_stream_score(Some(&clean)) > built_in_stream_score(Some(&bad)));
    }

    #[test]
    fn test_evaluate_rule_operator_greater_than() {
        let rule = stream_sorting_rule::Model {
            id: 1,
            name: "Test rule".to_string(),
            priority: 1,
            property: "bitrate".to_string(),
            operator: ">=".to_string(),
            value: "5000".to_string(),
            score_modifier: 10,
        };

        let stats_match = json!({"bitrate": 5500});
        let stats_mismatch = json!({"bitrate": 4000});

        assert!(evaluate_rule(&rule, &stats_match));
        assert!(!evaluate_rule(&rule, &stats_mismatch));
    }

    #[test]
    fn test_evaluate_rule_boolean() {
        let rule = stream_sorting_rule::Model {
            id: 1,
            name: "Test rule".to_string(),
            priority: 1,
            property: "is_live".to_string(),
            operator: "==".to_string(),
            value: "true".to_string(),
            score_modifier: 10,
        };

        let stats_match = json!({"is_live": true});
        let stats_mismatch = json!({"is_live": false});

        assert!(evaluate_rule(&rule, &stats_match));
        assert!(!evaluate_rule(&rule, &stats_mismatch));
    }

    #[test]
    fn test_evaluate_rule_string_contains() {
        let rule = stream_sorting_rule::Model {
            id: 1,
            name: "Test rule".to_string(),
            priority: 1,
            property: "codec".to_string(),
            operator: "contains".to_string(),
            value: "h264".to_string(),
            score_modifier: 10,
        };

        let stats_match = json!({"codec": "video/h264"});
        let stats_mismatch = json!({"codec": "hevc"});

        assert!(evaluate_rule(&rule, &stats_match));
        assert!(!evaluate_rule(&rule, &stats_mismatch));
    }

    #[test]
    fn test_normalize_sorting_rule_payload_trims_and_clamps_priority() {
        let normalized = normalize_sorting_rule_payload(CreateRulePayload {
            name: "  Prefer 1080p  ".to_string(),
            priority: -5,
            property: "height".to_string(),
            operator: ">=".to_string(),
            value: " 1080 ".to_string(),
            score_modifier: 25,
        })
        .expect("valid payload should normalize");

        assert_eq!(normalized.name, "Prefer 1080p");
        assert_eq!(normalized.priority, 0);
        assert_eq!(normalized.value, "1080");
    }

    #[test]
    fn test_normalize_sorting_rule_payload_rejects_invalid_values() {
        let invalid_property = normalize_sorting_rule_payload(CreateRulePayload {
            name: "Bad property".to_string(),
            priority: 0,
            property: "unknown".to_string(),
            operator: "==".to_string(),
            value: "online".to_string(),
            score_modifier: 10,
        });
        assert!(invalid_property.is_err());

        let invalid_numeric_target = normalize_sorting_rule_payload(CreateRulePayload {
            name: "Bad numeric".to_string(),
            priority: 0,
            property: "height".to_string(),
            operator: ">=".to_string(),
            value: "HD".to_string(),
            score_modifier: 10,
        });
        assert!(invalid_numeric_target.is_err());
    }
}
