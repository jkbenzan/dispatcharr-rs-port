use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dispatcharr_channels_stream", schema_name = "public")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub logo_url: Option<String>,
    pub tvg_id: Option<String>,
    pub local_file: Option<String>,
    pub current_viewers: i32,
    pub updated_at: DateTime<FixedOffset>,
    pub m3u_account_id: Option<i64>,
    pub stream_profile_id: Option<i64>,
    pub is_custom: bool,
    pub channel_group_id: Option<i64>,
    pub last_seen: DateTime<FixedOffset>,
    pub stream_hash: Option<String>,
    pub custom_properties: Option<Json>,
    pub is_stale: bool,
    pub is_adult: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
