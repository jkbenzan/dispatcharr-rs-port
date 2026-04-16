use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "epg_epgsource", schema_name = "public")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub is_active: bool,
    pub file_path: Option<String>,
    pub refresh_interval: i32,
    pub refresh_task_id: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
    pub status: String,
    pub last_message: Option<String>,
    pub extracted_file_path: Option<String>,
    pub custom_properties: Option<Json>,
    pub priority: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
