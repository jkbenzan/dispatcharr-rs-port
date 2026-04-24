use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "core_systemnotification")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub notification_key: String,
    pub notification_type: String,
    pub priority: String,
    pub source: String,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    pub action_data: String, // Assuming JSON stored as string or we can use Json
    pub is_active: bool,
    pub admin_only: bool,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
