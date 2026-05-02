use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "m3u_m3uaccountprofile")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub max_streams: i32,
    pub is_active: bool,
    pub search_pattern: String,
    pub replace_pattern: String,
    pub current_viewers: i32,
    pub m3u_account_id: i64,
    pub custom_properties: Option<Json>,
    pub exp_date: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

