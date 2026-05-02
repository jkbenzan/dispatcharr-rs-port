use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vod_m3useriesrelation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub external_series_id: String,
    pub custom_properties: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub last_episode_refresh: Option<DateTimeWithTimeZone>,
    pub m3u_account_id: i64,
    pub series_id: i64,
    pub category_id: Option<i64>,
    pub last_seen: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

