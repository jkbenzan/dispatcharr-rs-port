use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "m3u_m3ufilter", schema_name = "public")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub filter_type: String,
    pub regex_pattern: String,
    pub exclude: bool,
    pub m3u_account_id: i64,
    pub order: i32,
    pub custom_properties: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
