//! Channel Data Database — read-only SQLite sidecar for Gracenote station data.
//!
//! This module manages a secondary SQLite connection to `channel_data.db`, a
//! third-party database containing TV station metadata (names, call signs,
//! logos, lineups, channel numbers). The database is read-only and optional;
//! if the file is not present, all channel-db endpoints gracefully return 503.
//!
//! The database can be updated remotely: check for a newer version, download it,
//! and hot-swap the connection — all without restarting the server.

use sea_orm::{ConnectOptions, Database, DatabaseConnection, FromQueryResult, Statement};
use sea_orm::DatabaseBackend;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing;

// ---------------------------------------------------------------------------
// Data types returned by queries
// ---------------------------------------------------------------------------

/// Result of a station search query.
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct StationSearchResult {
    pub station_id: String,
    pub name: Option<String>,
    pub call_sign: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub station_type: Option<String>,
    pub logo_uri: Option<String>,
}

/// Detailed station info including lineup metadata.
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct StationDetail {
    pub station_id: String,
    pub name: Option<String>,
    pub call_sign: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub station_type: Option<String>,
    pub bcast_langs: Option<String>,
    pub logo_uri: Option<String>,
    pub logo_width: Option<i32>,
    pub logo_height: Option<i32>,
    pub lineup_count: Option<i64>,
    pub video_types: Option<String>,
    pub affiliate_id: Option<String>,
    pub affiliate_call_sign: Option<String>,
}

/// Summary of a lineup for search results.
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct LineupSummary {
    pub lineup_id: String,
    pub name: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub lineup_type: Option<String>,
    pub device: Option<String>,
    pub mso_name: Option<String>,
    pub channel_count: Option<i64>,
}

/// A channel within a lineup preview.
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct LineupChannel {
    pub channel_number: Option<String>,
    pub station_name: Option<String>,
    pub call_sign: Option<String>,
    pub station_id: String,
    pub video_type: Option<String>,
    pub logo_uri: Option<String>,
}

/// Database metadata (version, date, etc.)
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChannelDbMetadata {
    pub version: String,
    pub effective_date: String,
    pub schema_version: String,
    pub last_updated: String,
    pub station_count: i64,
}

/// Database statistics.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ChannelDbStats {
    pub total_stations: i64,
    pub total_countries: i64,
    pub total_markets: i64,
    pub total_lineups: i64,
    pub stations_with_logos: i64,
}

/// Available filter values for the search UI.
#[derive(Debug, Clone, Serialize, Default)]
pub struct FilterOptions {
    pub countries: Vec<String>,
    pub lineup_types: Vec<String>,
    pub qualities: Vec<String>,
}

/// Quality breakdown for a lineup preview.
#[derive(Debug, Clone, Serialize)]
pub struct LineupPreview {
    pub lineup: LineupInfo,
    pub total_channels: i64,
    pub quality_breakdown: std::collections::HashMap<String, i64>,
    pub channels: Vec<LineupChannel>,
}

#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct LineupInfo {
    pub lineup_id: String,
    pub name: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub lineup_type: Option<String>,
    pub device: Option<String>,
    pub mso_name: Option<String>,
}

/// Station search result enriched with countries and video types for matching.
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct StationMatchResult {
    pub station_id: String,
    pub name: Option<String>,
    pub call_sign: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub station_type: Option<String>,
    pub logo_uri: Option<String>,
    pub countries: Option<String>,
    pub video_types: Option<String>,
}

/// Metadata key-value row from the SQLite `metadata` table.
#[derive(Debug, FromQueryResult)]
struct MetadataRow {
    pub key: String,
    pub value: Option<String>,
}

/// Count result helper.
#[derive(Debug, FromQueryResult)]
struct CountResult {
    pub count: i64,
}

/// Quality breakdown row.
#[derive(Debug, FromQueryResult)]
struct QualityRow {
    pub quality: Option<String>,
    pub count: i64,
}

/// Simple text row for filter option queries.
#[derive(Debug, FromQueryResult)]
struct TextRow {
    pub val: Option<String>,
}

// ---------------------------------------------------------------------------
// ChannelDb — the main handle
// ---------------------------------------------------------------------------

/// Manages the optional read-only SQLite connection to the channel data database.
pub struct ChannelDb {
    /// The active database connection, or `None` if the file was not found.
    pub conn: Option<DatabaseConnection>,
    /// Absolute path to the database file.
    pub db_path: PathBuf,
}

impl ChannelDb {
    /// Initialize the channel database.
    ///
    /// If the file at `db_path` exists and contains the required tables, returns
    /// a `ChannelDb` with an active connection. Otherwise returns a `ChannelDb`
    /// with `conn = None` (graceful degradation).
    pub async fn init(db_path: &str) -> Self {
        let path = PathBuf::from(db_path);

        if !path.exists() {
            tracing::warn!(
                "⚠️  Channel data database not found at '{}' — channel identification features disabled",
                db_path
            );
            return Self { conn: None, db_path: path };
        }

        match Self::open_connection(&path).await {
            Ok(conn) => {
                // Validate schema
                if let Err(e) = Self::validate_schema(&conn).await {
                    tracing::error!("❌ Channel data database schema validation failed: {}", e);
                    return Self { conn: None, db_path: path };
                }
                tracing::info!("📺 Channel data database loaded from '{}'", db_path);
                Self { conn: Some(conn), db_path: path }
            }
            Err(e) => {
                tracing::error!("❌ Failed to open channel data database: {}", e);
                Self { conn: None, db_path: path }
            }
        }
    }

    /// Open a read-only SQLite connection.
    async fn open_connection(path: &Path) -> Result<DatabaseConnection, sea_orm::DbErr> {
        let url = format!("sqlite:{}?mode=ro", path.display());
        let mut opt = ConnectOptions::new(url);
        opt.sqlx_logging(false);
        Database::connect(opt).await
    }

    /// Verify that the required tables exist in the database.
    async fn validate_schema(conn: &DatabaseConnection) -> Result<(), String> {
        let required_tables = ["stations", "lineups", "station_lineups", "lineup_markets", "metadata"];
        for table in required_tables {
            let sql = format!(
                "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='{}'",
                table
            );
            let result = CountResult::find_by_statement(Statement::from_string(
                DatabaseBackend::Sqlite, sql,
            ))
            .one(conn)
            .await
            .map_err(|e| format!("Failed to check table '{}': {}", table, e))?;

            match result {
                Some(r) if r.count > 0 => {}
                _ => return Err(format!("Required table '{}' not found", table)),
            }
        }
        Ok(())
    }

    /// Get a reference to the connection, or return an error if unavailable.
    pub fn conn(&self) -> Result<&DatabaseConnection, &'static str> {
        self.conn.as_ref().ok_or("Channel data database not available")
    }

    /// Check if the database is loaded.
    pub fn is_available(&self) -> bool {
        self.conn.is_some()
    }

    /// Reload the connection (e.g., after downloading a new database file).
    pub async fn reload(&mut self) -> Result<(), String> {
        if !self.db_path.exists() {
            self.conn = None;
            return Err("Database file not found after download".to_string());
        }
        match Self::open_connection(&self.db_path).await {
            Ok(conn) => {
                Self::validate_schema(&conn).await?;
                self.conn = Some(conn);
                tracing::info!("📺 Channel data database reloaded successfully");
                Ok(())
            }
            Err(e) => {
                self.conn = None;
                Err(format!("Failed to reload database: {}", e))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Query functions
    // -----------------------------------------------------------------------

    /// Get database metadata (version, date, station count).
    pub async fn get_metadata(&self) -> Result<ChannelDbMetadata, String> {
        let conn = self.conn()?;
        let rows = MetadataRow::find_by_statement(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT key, value FROM metadata WHERE key IN ('data_version','effective_date','schema_version','last_updated')".to_string(),
        ))
        .all(conn).await.map_err(|e| e.to_string())?;

        let mut meta = ChannelDbMetadata::default();
        for row in rows {
            let val = row.value.unwrap_or_default();
            match row.key.as_str() {
                "data_version" => meta.version = val,
                "effective_date" => meta.effective_date = val,
                "schema_version" => meta.schema_version = val,
                "last_updated" => meta.last_updated = val,
                _ => {}
            }
        }

        let count = CountResult::find_by_statement(Statement::from_string(
            DatabaseBackend::Sqlite, "SELECT COUNT(*) as count FROM stations".to_string(),
        ))
        .one(conn).await.map_err(|e| e.to_string())?;
        meta.station_count = count.map(|c| c.count).unwrap_or(0);

        Ok(meta)
    }

    /// Get database statistics.
    pub async fn get_stats(&self) -> Result<ChannelDbStats, String> {
        let conn = self.conn()?;
        let mut stats = ChannelDbStats::default();

        let queries = [
            ("SELECT COUNT(*) as count FROM stations", &mut stats.total_stations),
            ("SELECT COUNT(DISTINCT country) as count FROM lineup_markets", &mut stats.total_countries),
            ("SELECT COUNT(DISTINCT postal_code) as count FROM lineup_markets", &mut stats.total_markets),
            ("SELECT COUNT(*) as count FROM lineups", &mut stats.total_lineups),
            ("SELECT COUNT(*) as count FROM stations WHERE logo_uri IS NOT NULL AND logo_uri != ''", &mut stats.stations_with_logos),
        ];

        for (sql, target) in queries {
            let result = CountResult::find_by_statement(Statement::from_string(
                DatabaseBackend::Sqlite, sql.to_string(),
            ))
            .one(conn).await.map_err(|e| e.to_string())?;
            *target = result.map(|c| c.count).unwrap_or(0);
        }

        Ok(stats)
    }

    /// Get available filter options (countries, lineup types, qualities).
    pub async fn get_filter_options(&self) -> Result<FilterOptions, String> {
        let conn = self.conn()?;
        let mut opts = FilterOptions::default();

        // Countries
        let rows = TextRow::find_by_statement(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT DISTINCT country as val FROM lineup_markets WHERE country IS NOT NULL ORDER BY country".to_string(),
        )).all(conn).await.map_err(|e| e.to_string())?;
        opts.countries = rows.into_iter().filter_map(|r| r.val).collect();

        // Lineup types
        let rows = TextRow::find_by_statement(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT DISTINCT type as val FROM lineups WHERE type IS NOT NULL ORDER BY type".to_string(),
        )).all(conn).await.map_err(|e| e.to_string())?;
        opts.lineup_types = rows.into_iter().filter_map(|r| r.val).collect();

        // Qualities (combine UHDTV and 4k)
        let rows = TextRow::find_by_statement(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT DISTINCT video_type as val FROM station_lineups WHERE video_type IS NOT NULL AND LENGTH(video_type) > 0 ORDER BY video_type".to_string(),
        )).all(conn).await.map_err(|e| e.to_string())?;

        let mut has_uhd = false;
        for row in rows {
            if let Some(val) = row.val {
                let upper = val.to_uppercase();
                if upper == "UHDTV" || upper == "4K" {
                    if !has_uhd {
                        opts.qualities.push("UHDTV/4K".to_string());
                        has_uhd = true;
                    }
                } else {
                    opts.qualities.push(val);
                }
            }
        }

        Ok(opts)
    }

    /// Search stations by name, call sign, or station ID.
    pub async fn search_stations(
        &self,
        query: &str,
        country: Option<&str>,
        quality: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StationSearchResult>, String> {
        let conn = self.conn()?;
        let like_query = format!("%{}%", query);
        let limit_val = if limit == 0 { 500 } else { limit };

        // Build SQL dynamically based on filters
        let mut sql = String::from(
            "SELECT DISTINCT s.station_id, s.name, s.call_sign, s.type as station_type, s.logo_uri \
             FROM stations s \
             LEFT JOIN station_lineups sl ON s.station_id = sl.station_id \
             LEFT JOIN lineups l ON sl.lineup_id = l.lineup_id \
             LEFT JOIN lineup_markets lm ON l.lineup_id = lm.lineup_id \
             WHERE (LOWER(s.name) LIKE LOWER($1) OR LOWER(s.call_sign) LIKE LOWER($2) OR s.station_id = $3)"
        );

        let mut param_idx = 4u32;
        let mut params: Vec<sea_orm::Value> = vec![
            like_query.clone().into(),
            like_query.into(),
            query.to_string().into(),
        ];

        if let Some(c) = country {
            if !c.is_empty() && c.to_uppercase() != "ALL" {
                sql.push_str(&format!(" AND lm.country = ${}", param_idx));
                params.push(c.to_string().into());
                param_idx += 1;
            }
        }

        if let Some(q) = quality {
            if !q.is_empty() {
                // Handle comma-separated qualities
                let qualities: Vec<&str> = q.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                if !qualities.is_empty() {
                    let placeholders: Vec<String> = qualities.iter().enumerate()
                        .map(|(i, _)| format!("${}", param_idx + i as u32))
                        .collect();
                    sql.push_str(&format!(" AND sl.video_type IN ({})", placeholders.join(",")));
                    for q_val in &qualities {
                        params.push(q_val.to_string().into());
                    }
                    param_idx += qualities.len() as u32;
                }
            }
        }

        sql.push_str(&format!(
            " ORDER BY CASE WHEN LOWER(s.name) = LOWER(${}) THEN 1 \
             WHEN LOWER(s.call_sign) = LOWER(${}) THEN 2 \
             WHEN LOWER(s.name) LIKE LOWER(${}) THEN 3 \
             ELSE 4 END, s.name LIMIT {}",
            param_idx, param_idx + 1, param_idx + 2, limit_val
        ));
        params.push(query.to_string().into());
        params.push(query.to_string().into());
        params.push(format!("{}%", query).into());

        // Execute raw SQL with positional params
        // SeaORM raw queries with SQLite use `?` not `$N`, so we need to replace
        let sql_with_qmarks = replace_positional_params(&sql);

        let results = StationSearchResult::find_by_statement(
            Statement::from_sql_and_values(DatabaseBackend::Sqlite, &sql_with_qmarks, params),
        )
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(results)
    }

    /// Get detailed station information.
    pub async fn get_station(&self, station_id: &str) -> Result<Option<StationDetail>, String> {
        let conn = self.conn()?;
        let result = StationDetail::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT s.station_id, s.name, s.call_sign, s.type as station_type, \
             s.bcast_langs, s.logo_uri, s.logo_width, s.logo_height, \
             COUNT(DISTINCT sl.lineup_id) as lineup_count, \
             GROUP_CONCAT(DISTINCT sl.video_type) as video_types, \
             MAX(sl.affiliate_id) as affiliate_id, \
             MAX(sl.affiliate_call_sign) as affiliate_call_sign \
             FROM stations s \
             LEFT JOIN station_lineups sl ON s.station_id = sl.station_id \
             WHERE s.station_id = ? \
             GROUP BY s.station_id, s.name, s.call_sign, s.type, s.bcast_langs, \
             s.logo_uri, s.logo_width, s.logo_height",
            vec![station_id.into()],
        ))
        .one(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(result)
    }

    /// Search lineups by postal code and country.
    pub async fn search_lineups_by_zip(
        &self,
        country: &str,
        postal_code: &str,
    ) -> Result<Vec<LineupSummary>, String> {
        let conn = self.conn()?;
        let results = LineupSummary::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT DISTINCT l.lineup_id, l.name, l.type as lineup_type, l.device, l.mso_name, \
             COUNT(DISTINCT sl.station_id) as channel_count \
             FROM lineups l \
             JOIN lineup_markets lm ON l.lineup_id = lm.lineup_id \
             LEFT JOIN station_lineups sl ON l.lineup_id = sl.lineup_id \
             WHERE lm.country = ? AND lm.postal_code = ? \
             GROUP BY l.lineup_id, l.name, l.type, l.device, l.mso_name \
             ORDER BY l.type, l.name",
            vec![country.into(), postal_code.into()],
        ))
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(results)
    }

    /// Get a full lineup preview with channels and quality breakdown.
    pub async fn preview_lineup(&self, lineup_id: &str) -> Result<Option<LineupPreview>, String> {
        let conn = self.conn()?;

        // Get lineup info
        let lineup = LineupInfo::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT lineup_id, name, type as lineup_type, device, mso_name FROM lineups WHERE lineup_id = ?",
            vec![lineup_id.into()],
        ))
        .one(conn)
        .await
        .map_err(|e| e.to_string())?;

        let lineup = match lineup {
            Some(l) => l,
            None => return Ok(None),
        };

        // Quality breakdown
        let quality_rows = QualityRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT COALESCE(video_type, 'UNKNOWN') as quality, COUNT(*) as count \
             FROM station_lineups WHERE lineup_id = ? GROUP BY video_type",
            vec![lineup_id.into()],
        ))
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

        let mut quality_breakdown = std::collections::HashMap::new();
        for row in quality_rows {
            quality_breakdown.insert(row.quality.unwrap_or_else(|| "UNKNOWN".to_string()), row.count);
        }

        // Total channel count
        let total = CountResult::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM station_lineups WHERE lineup_id = ?",
            vec![lineup_id.into()],
        ))
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
        .map(|c| c.count)
        .unwrap_or(0);

        // All channels in this lineup
        let channels = LineupChannel::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT sl.channel_number, s.name as station_name, s.call_sign, \
             sl.station_id, sl.video_type, s.logo_uri \
             FROM station_lineups sl \
             JOIN stations s ON sl.station_id = s.station_id \
             WHERE sl.lineup_id = ? \
             ORDER BY CAST(sl.channel_number AS REAL)",
            vec![lineup_id.into()],
        ))
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(Some(LineupPreview {
            lineup,
            total_channels: total,
            quality_breakdown,
            channels,
        }))
    }

    /// Search stations for match suggestions, returning enriched results with
    /// countries and video types.
    pub async fn search_for_matching(
        &self,
        clean_name: &str,
        country_filter: Option<&str>,
        resolution_filter: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<StationMatchResult>, String> {
        let conn = self.conn()?;
        let like_query = format!("%{}%", clean_name);
        let limit_val = if limit == 0 { 20 } else { limit };

        let mut sql = String::from(
            "SELECT DISTINCT s.station_id, s.name, s.call_sign, s.type as station_type, s.logo_uri, \
             GROUP_CONCAT(DISTINCT lm.country) as countries, \
             GROUP_CONCAT(DISTINCT sl.video_type) as video_types \
             FROM stations s \
             LEFT JOIN station_lineups sl ON s.station_id = sl.station_id \
             LEFT JOIN lineups l ON sl.lineup_id = l.lineup_id \
             LEFT JOIN lineup_markets lm ON l.lineup_id = lm.lineup_id \
             WHERE (LOWER(s.name) LIKE LOWER(?) OR LOWER(s.call_sign) LIKE LOWER(?) \
             OR LOWER(s.name) = LOWER(?) OR LOWER(s.call_sign) = LOWER(?))"
        );

        let mut params: Vec<sea_orm::Value> = vec![
            like_query.clone().into(),
            like_query.into(),
            clean_name.to_string().into(),
            clean_name.to_string().into(),
        ];

        if let Some(resolutions) = resolution_filter {
            if !resolutions.is_empty() {
                let placeholders: Vec<&str> = resolutions.iter().map(|_| "?").collect();
                sql.push_str(&format!(" AND sl.video_type IN ({})", placeholders.join(",")));
                for r in resolutions {
                    params.push(r.clone().into());
                }
            }
        }

        if let Some(country) = country_filter {
            if !country.is_empty() {
                sql.push_str(" AND lm.country = ?");
                params.push(country.to_string().into());
            }
        }

        sql.push_str(
            " GROUP BY s.station_id, s.name, s.call_sign, s.type, s.logo_uri \
             ORDER BY CASE WHEN LOWER(s.name) = LOWER(?) THEN 1 \
             WHEN LOWER(s.call_sign) = LOWER(?) THEN 2 \
             WHEN LOWER(s.name) LIKE LOWER(?) THEN 3 ELSE 4 END"
        );
        params.push(clean_name.to_string().into());
        params.push(clean_name.to_string().into());
        params.push(format!("{}%", clean_name).into());

        sql.push_str(&format!(" LIMIT {}", limit_val));

        let results = StationMatchResult::find_by_statement(
            Statement::from_sql_and_values(DatabaseBackend::Sqlite, &sql, params),
        )
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(results)
    }

    /// Get a single station by ID for matching (with countries/video_types).
    pub async fn get_station_for_matching(&self, station_id: &str) -> Result<Option<StationMatchResult>, String> {
        let conn = self.conn()?;
        let result = StationMatchResult::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT DISTINCT s.station_id, s.name, s.call_sign, s.type as station_type, s.logo_uri, \
             GROUP_CONCAT(DISTINCT lm.country) as countries, \
             GROUP_CONCAT(DISTINCT sl.video_type) as video_types \
             FROM stations s \
             LEFT JOIN station_lineups sl ON s.station_id = sl.station_id \
             LEFT JOIN lineups l ON sl.lineup_id = l.lineup_id \
             LEFT JOIN lineup_markets lm ON l.lineup_id = lm.lineup_id \
             WHERE s.station_id = ? \
             GROUP BY s.station_id, s.name, s.call_sign, s.type, s.logo_uri",
            vec![station_id.into()],
        ))
        .one(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(result)
    }

    /// Get channels from a lineup for import, with quality filtering.
    pub async fn get_lineup_channels_for_import(
        &self,
        lineup_id: &str,
        include_sd: bool,
        include_hd: bool,
        include_uhd: bool,
        include_unknown: bool,
    ) -> Result<Vec<LineupChannel>, String> {
        let conn = self.conn()?;

        let mut conditions = Vec::new();
        if include_sd { conditions.push("sl.video_type = 'SDTV'".to_string()); }
        if include_hd { conditions.push("sl.video_type = 'HDTV'".to_string()); }
        if include_uhd { conditions.push("sl.video_type = 'UHDTV'".to_string()); }
        if include_unknown { conditions.push("(sl.video_type IS NULL OR sl.video_type = '')".to_string()); }

        if conditions.is_empty() {
            return Err("At least one quality type must be selected".to_string());
        }

        let quality_filter = conditions.join(" OR ");
        let sql = format!(
            "SELECT sl.channel_number, s.name as station_name, s.call_sign, \
             sl.station_id, sl.video_type, s.logo_uri \
             FROM station_lineups sl \
             JOIN stations s ON sl.station_id = s.station_id \
             WHERE sl.lineup_id = ? AND ({}) \
             ORDER BY CAST(sl.channel_number AS REAL)",
            quality_filter
        );

        let results = LineupChannel::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            &sql,
            vec![lineup_id.into()],
        ))
        .all(conn)
        .await
        .map_err(|e| e.to_string())?;

        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Replace `$1`, `$2`, etc. positional parameters with `?` for SQLite.
fn replace_positional_params(sql: &str) -> String {
    let re = regex::Regex::new(r"\$\d+").unwrap();
    re.replace_all(sql, "?").to_string()
}
