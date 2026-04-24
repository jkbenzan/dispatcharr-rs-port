use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(
    table_name = "dispatcharr_channels_channelgroupm3uaccount",
    schema_name = "public"
)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub enabled: bool,
    pub channel_group_id: i64,
    pub m3u_account_id: i64,
    pub custom_properties: Option<Json>,
    pub auto_channel_sync: bool,
    pub auto_sync_channel_start: Option<f64>,
    pub is_stale: bool,
    pub last_seen: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
