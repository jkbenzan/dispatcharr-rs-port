use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "epg_epgdata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub tvg_id: Option<String>,
    pub name: String,
    pub epg_source_id: Option<i64>,
    pub icon_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

