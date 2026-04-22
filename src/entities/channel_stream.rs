use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(
    table_name = "dispatcharr_channels_channelstream",
    schema_name = "public"
)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub channel_id: i64,
    pub stream_id: i64,
    pub order: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
