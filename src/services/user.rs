use chrono::Duration;
use redis::AsyncCommands;
use sea_orm::prelude::*;
use sea_orm::sea_query::{Alias, Expr, Func};
use sea_orm::{ConnectionTrait, FromQueryResult, QuerySelect, Set, UpdateMany, UpdateResult};

use crate::entity::prelude::*;
use crate::lyrics;
use crate::utils::Clock;

pub struct UserStatsIncreaseQueryBuilder(UpdateMany<UserEntity>);

impl UserStatsIncreaseQueryBuilder {
    fn new(user_id: &str) -> Self {
        let query: UpdateMany<_> = UserEntity::update_many();

        let query = query.filter(UserColumn::Id.eq(user_id));

        Self(query)
    }

    pub fn removed_collection(mut self, amount: u32) -> Self {
        self.0 = self.0.col_expr(
            UserColumn::RemovedCollection,
            Expr::col(UserColumn::RemovedCollection).add(amount),
        );

        self
    }

    pub fn removed_playlists(mut self, amount: u32) -> Self {
        self.0 = self.0.col_expr(
            UserColumn::RemovedPlaylists,
            Expr::col(UserColumn::RemovedPlaylists).add(amount),
        );

        self
    }

    pub fn checked_lyrics(mut self, profane: bool, provider: Option<lyrics::Provider>) -> Self {
        self.0 = self
            .0
            .col_expr(
                UserColumn::LyricsChecked,
                Expr::col(UserColumn::LyricsChecked).add(1),
            )
            .col_expr(
                UserColumn::LyricsProfane,
                Expr::col(UserColumn::LyricsProfane).add(if profane { 1 } else { 0 }),
            );

        let col = match provider {
            Some(lyrics::Provider::Genius) => UserColumn::LyricsGenius,
            Some(lyrics::Provider::Musixmatch) => UserColumn::LyricsMusixmatch,
            Some(lyrics::Provider::LrcLib) => UserColumn::LyricsLrcLib,
            Some(lyrics::Provider::AZLyrics) => UserColumn::LyricsAZLyrics,
            None => return self,
        };

        self.0 = self.0.col_expr(col, Expr::col(col).add(1));

        self
    }

    pub fn analyzed_lyrics(mut self) -> Self {
        self.0 = self.0.col_expr(
            UserColumn::LyricsAnalyzed,
            Expr::col(UserColumn::LyricsAnalyzed).add(1),
        );

        self
    }

    pub async fn exec(self, db: &impl ConnectionTrait) -> Result<UpdateResult, DbErr> {
        self.0
            .col_expr(UserColumn::UpdatedAt, Expr::value(Clock::now()))
            .exec(db)
            .await
    }
}

#[derive(FromQueryResult, Default)]
pub struct UserStats {
    pub removed_playlists: i64,
    pub removed_collection: i64,
    pub lyrics_checked: i64,
    pub lyrics_found: i64,
    pub lyrics_profane: i64,
    pub lyrics_genius: i64,
    pub lyrics_musixmatch: i64,
    pub lyrics_lrclib: i64,
    pub lyrics_azlyrics: i64,
    pub lyrics_analyzed: i64,
}

pub struct UserService;

impl UserService {
    pub fn query(id: Option<&str>, status: Option<UserStatus>) -> Select<UserEntity> {
        let mut query: Select<_> = UserEntity::find();

        if let Some(status) = status {
            query = query.filter(UserColumn::Status.eq(status));
        };

        if let Some(id) = id {
            query = query.filter(UserColumn::Id.eq(id));
        };

        query
    }

    pub async fn sync_name(
        db: &impl ConnectionTrait,
        id: &str,
        name: &str,
    ) -> anyhow::Result<UpdateResult> {
        let query: UpdateMany<_> = UserEntity::update_many();

        let update_result: UpdateResult = query
            .col_expr(UserColumn::Name, Expr::value(name))
            .col_expr(UserColumn::UpdatedAt, Expr::value(Clock::now()))
            .filter(UserColumn::Id.eq(id))
            .filter(UserColumn::Name.ne(name))
            .exec(db)
            .await?;

        Ok(update_result)
    }

    pub async fn sync_current_playing(
        mut redis: redis::aio::MultiplexedConnection,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<bool> {
        let key = format!("rustify:track_check:{user_id}:{track_id}");
        // TODO: move somewhere else
        let default_ttl = Duration::hours(24).num_seconds() as u64;
        let ttl: u64 = dotenv::var("LAST_PLAYED_TTL")
            .unwrap_or(default_ttl.to_string())
            .parse()
            .unwrap_or(default_ttl);

        let played: bool = redis.exists(&key).await?;

        let _: () = redis.set_ex(key, true, ttl as u64).await?;

        // returns true when track new
        Ok(!played)
    }

    pub async fn obtain_by_id(db: &impl ConnectionTrait, id: &str) -> anyhow::Result<UserModel> {
        let user = Self::query(Some(id), None).one(db).await?;

        let user = match user {
            Some(user) => user,
            None => {
                UserActiveModel {
                    id: Set(id.to_owned()),
                    ..Default::default()
                }
                .insert(db)
                .await?
            },
        };

        Ok(user)
    }

    pub async fn set_status(
        db: &impl ConnectionTrait,
        id: &str,
        status: UserStatus,
    ) -> anyhow::Result<UpdateResult> {
        let res = UserEntity::update_many()
            .filter(UserColumn::Id.eq(id))
            .col_expr(UserColumn::Status, Expr::value(status))
            .exec(db)
            .await?;

        Ok(res)
    }

    pub async fn set_cfg_skippage_secs(
        db: &impl ConnectionTrait,
        id: &str,
        duration: chrono::Duration,
    ) -> anyhow::Result<UpdateResult> {
        let res = UserEntity::update_many()
            .filter(UserColumn::Id.eq(id))
            .col_expr(
                UserColumn::CfgSkippageSecs,
                Expr::value(duration.num_seconds()),
            )
            .exec(db)
            .await?;

        Ok(res)
    }

    pub async fn set_cfg_skippage_enabled(
        db: &impl ConnectionTrait,
        id: &str,
        enabled: bool,
    ) -> anyhow::Result<UpdateResult> {
        let res = UserEntity::update_many()
            .filter(UserColumn::Id.eq(id))
            .col_expr(UserColumn::CfgSkippageEnabled, Expr::value(enabled))
            .exec(db)
            .await?;

        Ok(res)
    }

    pub async fn set_locale(
        db: &impl ConnectionTrait,
        id: &str,
        locale: UserLocale,
    ) -> anyhow::Result<UpdateResult> {
        let res = UserEntity::update_many()
            .filter(UserColumn::Id.eq(id))
            .col_expr(UserColumn::Locale, Expr::value(locale))
            .exec(db)
            .await?;

        Ok(res)
    }

    pub async fn get_users_for_locale(
        db: &impl ConnectionTrait,
        locale: UserLocale,
    ) -> anyhow::Result<Vec<UserModel>> {
        let res = Self::query(None, None)
            .filter(UserColumn::Locale.eq(locale))
            .all(db)
            .await?;

        Ok(res)
    }

    pub async fn count_users_locales(
        db: &impl ConnectionTrait,
    ) -> anyhow::Result<Vec<(UserLocale, i64)>> {
        let res: Vec<(UserLocale, i64)> = Self::query(None, None)
            .select_only()
            .column(UserColumn::Locale)
            .column_as(UserColumn::Id.count(), "count")
            .group_by(UserColumn::Locale)
            .into_tuple()
            .all(db)
            .await?;

        Ok(res)
    }

    pub async fn set_magic_playlist(
        db: &impl ConnectionTrait,
        id: &str,
        playlist_id: &str,
    ) -> anyhow::Result<UpdateResult> {
        let res = UserEntity::update_many()
            .filter(UserColumn::Id.eq(id))
            .col_expr(UserColumn::MagicPlaylist, Expr::value(playlist_id))
            .exec(db)
            .await?;

        Ok(res)
    }

    pub fn increase_stats_query(user_id: &str) -> UserStatsIncreaseQueryBuilder {
        UserStatsIncreaseQueryBuilder::new(user_id)
    }

    pub async fn count_users(
        db: &impl ConnectionTrait,
        status: Option<UserStatus>,
    ) -> anyhow::Result<i64> {
        #[derive(FromQueryResult, Default)]
        struct UsersCount {
            count: Option<i64>,
        }

        let mut q = UserEntity::find().select_only().expr_as(
            UserColumn::Id.count().cast_as(Alias::new("bigint")),
            "count",
        );

        if let Some(status) = status {
            q = q.filter(UserColumn::Status.eq(status));
        }

        let skips: UsersCount = q.into_model().one(db).await?.unwrap_or_default();

        Ok(skips.count.unwrap_or_default())
    }

    pub async fn get_stats(
        db: &impl ConnectionTrait,
        id: Option<&str>,
    ) -> anyhow::Result<UserStats> {
        let bigint = || Alias::new("bigint");
        let res = Self::query(id, None)
            .select_only()
            .expr_as(
                Func::coalesce([
                    UserColumn::RemovedCollection.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "removed_collection",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::RemovedPlaylists.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "removed_playlists",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsChecked.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_checked",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsProfane.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_profane",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsGenius.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_genius",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsMusixmatch.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_musixmatch",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsLrcLib.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_lrclib",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsAZLyrics.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_azlyrics",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsGenius
                        .sum()
                        .add(UserColumn::LyricsMusixmatch.sum())
                        .add(UserColumn::LyricsLrcLib.sum())
                        .add(UserColumn::LyricsAZLyrics.sum())
                        .cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_found",
            )
            .expr_as(
                Func::coalesce([
                    UserColumn::LyricsAnalyzed.sum().cast_as(bigint()),
                    Expr::val(0).into(),
                ]),
                "lyrics_analyzed",
            )
            .into_model::<UserStats>()
            .one(db)
            .await?
            .unwrap_or_default();

        Ok(res)
    }
}
