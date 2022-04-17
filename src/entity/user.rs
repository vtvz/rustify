use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::Set;
use std::str::FromStr;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "user"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub removed_playlists: u32,
    pub removed_collection: u32,
    pub status: Status,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
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
    Name,
    RemovedPlaylists,
    RemovedCollection,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = String;

    fn auto_increment() -> bool {
        false
    }
}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::String(None).def(),
            Self::Name => ColumnType::String(None).def(),
            Self::RemovedPlaylists => ColumnType::Unsigned.def(),
            Self::RemovedCollection => ColumnType::Unsigned.def(),
            Self::Status => Status::db_type(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::spotify_auth::Entity")]
    SpotifyAuth,
}

impl Related<super::spotify_auth::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SpotifyAuth.def()
    }
}

#[derive(Debug, Clone, EnumIter, DeriveActiveEnum, PartialEq)]
#[sea_orm(rs_type = "String", db_type = "String(None)")]
pub enum Status {
    #[sea_orm(string_value = "active")]
    Active,
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
        Self::Active
    }
}
