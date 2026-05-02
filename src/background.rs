use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub last_run_at: Option<DateTime<Utc>>,
    pub total_processed: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub last_error: Option<String>,
}

impl Default for TaskStats {
    fn default() -> Self {
        Self {
            last_run_at: None,
            total_processed: 0,
            success_count: 0,
            failure_count: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackgroundTelemetry {
    pub m3u_refresh: TaskStats,
    pub epg_refresh: TaskStats,
    pub stream_check: TaskStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceSettings {
    pub stream_check_frequency_days: i32,
    pub off_hours_start: u32,
    pub off_hours_end: u32,
    pub idle_threshold_minutes: u64,
    pub batch_size: usize,
}

impl Default for MaintenanceSettings {
    fn default() -> Self {
        Self {
            stream_check_frequency_days: 7,
            off_hours_start: 2,
            off_hours_end: 6,
            idle_threshold_minutes: 30,
            batch_size: 50,
        }
    }
}

pub type Telemetry = Arc<RwLock<BackgroundTelemetry>>;
