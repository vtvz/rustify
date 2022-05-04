use core::str::FromStr;

use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "track_status"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel)]
pub struct Model {
    pub id: u32,
    pub user_id: String,
    pub track_id: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub skips: u32,
    pub status: Status,
}

impl ActiveModelBehavior for ActiveModel {
    fn before_save(mut self, insert: bool) -> Result<Self, DbErr> {
        self.updated_at = Set(Utc::now().naive_local());
        if insert {
            self.created_at = Set(Utc::now().naive_local());
        }

        Ok(self)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    TrackId,
    CreatedAt,
    UpdatedAt,
    Skips,
    Status,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = u32;

    fn auto_increment() -> bool {
        true
    }
}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Unsigned.def(),
            Self::UserId => ColumnType::String(None).def(),
            Self::TrackId => ColumnType::String(None).def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
            Self::Skips => ColumnType::Unsigned.def(),
            Self::Status => Status::db_type(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

#[derive(Debug, Clone, EnumIter, DeriveActiveEnum, PartialEq)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum Status {
    #[sea_orm(string_value = "disliked")]
    Disliked,
    #[sea_orm(string_value = "ignore")]
    Ignore,
    #[sea_orm(string_value = "none")]
    None,
}

impl FromStr for Status {
    type Err = sea_orm::DbErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Status::try_from(s)
    }
}

impl TryFrom<&str> for Status {
    type Error = sea_orm::DbErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Status::try_from_value(&value.to_owned())
    }
}

impl ToString for Status {
    fn to_string(&self) -> String {
        self.to_value()
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::None
    }
}
