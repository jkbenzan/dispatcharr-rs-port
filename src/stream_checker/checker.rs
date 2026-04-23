use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{error, info};
use std::time::Duration;

use crate::AppState;
use crate::entities::stream;

pub async fn test_stream(
    State(state): State<Arc<AppState>>,
    Path(stream_id): Path<i64>,
) -> impl IntoResponse {
    let stream_obj = match stream::Entity::find_by_id(stream_id).one(&state.db).await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"success": false, "message": "Stream not found"})),
            );
        }
    };

    let stream_url = match &stream_obj.url {
        Some(url) => url.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "message": "Stream has no URL"})),
            );
        }
    };

    info!("🔍 Testing Stream: {} (ID: {})", stream_obj.name, stream_id);

    // 1. Run ffprobe
    let mut ffprobe_cmd = Command::new("ffprobe");
    ffprobe_cmd.args(&[
        "-v", "error",
        "-skip_frame", "nokey",
        "-print_format", "json",
        "-show_streams",
        &stream_url,
    ]);

    // Set user agent
    ffprobe_cmd.arg("-user_agent");
    ffprobe_cmd.arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.3");

    let ffprobe_result = match tokio::time::timeout(Duration::from_secs(40), ffprobe_cmd.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            error!("ffprobe failed to start: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": format!("ffprobe failed: {}", e)})),
            );
        }
        Err(_) => {
            error!("ffprobe timed out");
            return (
                StatusCode::GATEWAY_TIMEOUT,
                Json(json!({"success": false, "message": "ffprobe timed out"})),
            );
        }
    };

    if !ffprobe_result.status.success() {
        let stderr = String::from_utf8_lossy(&ffprobe_result.stderr);
        error!("ffprobe error: {}", stderr);

        let mut active_stream: stream::ActiveModel = stream_obj.into();
        let mut props = active_stream.custom_properties.unwrap().unwrap_or_else(|| json!({}));
        props["stream_stats"] = json!({});
        props["stream_stats_updated_at"] = Value::Null;
        active_stream.custom_properties = Set(Some(props));
        let _ = active_stream.update(&state.db).await;

        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "ffprobe failed", "stderr": stderr.to_string()})),
        );
    }

    let probe_output = String::from_utf8_lossy(&ffprobe_result.stdout);
    let probe_data: Value = match serde_json::from_str(&probe_output) {
        Ok(d) => d,
        Err(e) => {
             error!("ffprobe output JSON parsing failed: {}", e);
             return (
                 StatusCode::INTERNAL_SERVER_ERROR,
                 Json(json!({"success": false, "message": "Failed to parse ffprobe output"})),
             );
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
        props["stream_stats"] = json!({});
        props["stream_stats_updated_at"] = Value::Null;
        active_stream.custom_properties = Set(Some(props));
        let _ = active_stream.update(&state.db).await;

        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"success": false, "message": "No streams found in ffprobe output"})),
        );
    }

    // 2. Run ffmpeg for bitrate
    info!("🎬 FFmpeg Bitrate Analysis for {}", stream_obj.name);
    let mut ffmpeg_cmd = Command::new("ffmpeg");
    ffmpeg_cmd.args(&[
        "-t", "10", // Test duration
        "-i", &stream_url,
        "-c", "copy",
        "-f", "null",
        "-",
    ]);

    ffmpeg_cmd.arg("-user_agent");
    ffmpeg_cmd.arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.3");

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
        "video_codec": video_codec,
        "width": width,
        "height": height,
        "fps": fps,
        "audio_codec": audio_codec,
        "channels": channels,
        "bitrate": bitrate,
        "status": "online"
    });

    let mut active_stream: stream::ActiveModel = stream_obj.clone().into();
    let mut props = active_stream.custom_properties.unwrap().unwrap_or_else(|| json!({}));
    props["stream_stats"] = stats.clone();
    props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
    active_stream.custom_properties = Set(Some(props));

    if let Err(e) = active_stream.update(&state.db).await {
         error!("Failed to update stream stats in DB: {}", e);
         return (
             StatusCode::INTERNAL_SERVER_ERROR,
             Json(json!({"success": false, "message": "Failed to save stats to DB"})),
         );
    }

    // Prepare API response mirroring the DB stream model with updated stats
    let mut response_json = serde_json::to_value(&stream_obj).unwrap();
    if let Some(obj) = response_json.as_object_mut() {
        let mut new_props = stream_obj.custom_properties.clone().unwrap_or_else(|| json!({}));
        new_props["stream_stats"] = stats;
        new_props["stream_stats_updated_at"] = json!(chrono::Utc::now().to_rfc3339());
        obj.insert("custom_properties".to_string(), new_props);
    }

    (StatusCode::OK, Json(json!({
        "success": true,
        "stream": response_json
    })))
}
