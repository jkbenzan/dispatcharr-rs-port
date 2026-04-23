use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vod_vodcategory", schema_name = "public")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub category_type: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::vod_m3uvodcategoryrelation::Entity")]
    VodM3uvodcategoryrelation,
}

impl Related<super::vod_m3uvodcategoryrelation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VodM3uvodcategoryrelation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
