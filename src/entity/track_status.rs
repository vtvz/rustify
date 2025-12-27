use core::str::FromStr;

use sea_orm::Set;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::async_trait::async_trait;

use crate::utils::Clock;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "track_status"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveModel, DeriveActiveModel)]
pub struct Model {
    pub id: i32,
    pub user_id: String,
    pub track_id: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub skips: i64,
    pub status: Status,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.updated_at = Set(Clock::now());

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
    type ValueType = i64;

    fn auto_increment() -> bool {
        true
    }
}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Integer.def(),
            Self::UserId => ColumnType::Text.def(),
            Self::TrackId => ColumnType::Text.def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
            Self::Skips => ColumnType::BigInteger.def(),
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

#[derive(Debug, Clone, EnumIter, DeriveActiveEnum, PartialEq, Eq, Default, Copy)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum Status {
    #[sea_orm(string_value = "disliked")]
    Disliked,
    #[sea_orm(string_value = "ignore")]
    Ignore,
    #[sea_orm(string_value = "none")]
    #[default]
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
