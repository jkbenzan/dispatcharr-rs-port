use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dispatcharr_channels_channel")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub channel_number: f64,
    pub name: String,
    pub tvg_id: Option<String>,
    pub stream_profile_id: Option<i64>,
    pub channel_group_id: Option<i64>,
    pub uuid: Uuid,
    pub epg_data_id: Option<i64>,
    pub logo_id: Option<i64>,
    pub tvc_guide_stationid: Option<String>,
    pub user_level: i32,
    pub auto_created: bool,
    pub auto_created_by_id: Option<i64>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub is_adult: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

