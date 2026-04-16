use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "m3u_m3uaccount", schema_name = "public")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub server_url: Option<String>,
    pub max_streams: i32,
    pub is_active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
    pub user_agent_id: Option<i64>,
    pub server_group_id: Option<i64>,
    pub locked: bool,
    pub stream_profile_id: Option<i64>,
    pub custom_properties: Option<Json>,
    pub refresh_interval: i32,
    pub refresh_task_id: Option<i32>,
    pub file_path: Option<String>,
    pub stale_stream_days: i32,
    pub account_type: String,
    pub password: Option<String>,
    pub username: Option<String>,
    pub last_message: Option<String>,
    pub status: String,
    pub priority: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
