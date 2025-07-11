use std::str::FromStr;

use sea_orm::Set;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::async_trait::async_trait;

use crate::utils::Clock;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "user"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveModel, DeriveActiveModel)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub locale: Locale,
    pub removed_playlists: i64,
    pub removed_collection: i64,
    pub lyrics_checked: i64,
    pub lyrics_analyzed: i64,
    pub lyrics_genius: i64,
    pub lyrics_musixmatch: i64,
    pub lyrics_profane: i64,
    pub status: Status,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub cfg_check_profanity: bool,
    pub cfg_skip_tracks: bool,
    pub cfg_skippage_secs: i64,
    pub cfg_skippage_enabled: bool,
    pub magic_playlist: Option<String>,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.updated_at = Set(Clock::now());
        if insert {
            self.created_at = Set(Clock::now());
        }

        Ok(self)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Name,
    Locale,
    RemovedPlaylists,
    RemovedCollection,
    LyricsChecked,
    LyricsAnalyzed,
    LyricsGenius,
    LyricsMusixmatch,
    #[sea_orm(column_name = "lyrics_lrclib")]
    LyricsLrcLib,
    #[sea_orm(column_name = "lyrics_azlyrics")]
    LyricsAZLyrics,
    LyricsProfane,
    Status,
    CreatedAt,
    UpdatedAt,
    CfgCheckProfanity,
    CfgSkipTracks,
    CfgSkippageSecs,
    CfgSkippageEnabled,
    MagicPlaylist,
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
            Self::Id => ColumnType::Text.def(),
            Self::Name => ColumnType::Text.def(),
            Self::Locale => Locale::db_type(),
            Self::RemovedPlaylists => ColumnType::BigInteger.def(),
            Self::RemovedCollection => ColumnType::BigInteger.def(),
            Self::LyricsChecked => ColumnType::BigInteger.def(),
            Self::LyricsAnalyzed => ColumnType::BigInteger.def(),
            Self::LyricsGenius => ColumnType::BigInteger.def(),
            Self::LyricsMusixmatch => ColumnType::BigInteger.def(),
            Self::LyricsLrcLib => ColumnType::BigInteger.def(),
            Self::LyricsAZLyrics => ColumnType::BigInteger.def(),
            Self::LyricsProfane => ColumnType::BigInteger.def(),
            Self::Status => Status::db_type(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
            Self::CfgCheckProfanity => ColumnType::Boolean.def(),
            Self::CfgSkipTracks => ColumnType::Boolean.def(),
            Self::CfgSkippageSecs => ColumnType::BigInteger.def(),
            Self::CfgSkippageEnabled => ColumnType::Boolean.def(),
            Self::MagicPlaylist => ColumnType::Text.def().null(),
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

#[derive(Debug, Clone, EnumIter, DeriveActiveEnum, PartialEq, Eq)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum Status {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "blocked")]
    Blocked,
    #[sea_orm(string_value = "forbidden")]
    Forbidden,
    #[sea_orm(string_value = "token_invalid")]
    TokenInvalid,
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

impl Default for Status {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, EnumIter, DeriveActiveEnum, PartialEq, Eq)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum Locale {
    #[sea_orm(string_value = "ru")]
    Russian,
    #[sea_orm(string_value = "en")]
    English,
}

impl Locale {
    pub fn language(&self) -> &str {
        match self {
            Self::Russian => "Russian",
            Self::English => "English",
        }
    }
}

impl AsRef<str> for Locale {
    fn as_ref(&self) -> &str {
        match self {
            Self::Russian => "ru",
            Self::English => "en",
        }
    }
}

impl FromStr for Locale {
    type Err = sea_orm::DbErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&str> for Locale {
    type Error = sea_orm::DbErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from_value(&value.to_owned())
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::English
    }
}
