use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "epg_programdata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub start_time: DateTimeWithTimeZone,
    pub end_time: DateTimeWithTimeZone,
    pub title: String,
    pub sub_title: Option<String>,
    pub description: Option<String>,
    pub tvg_id: Option<String>,
    pub epg_id: i64,
    pub custom_properties: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

