use sea_orm::prelude::*;
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{
    ConnectionTrait,
    FromQueryResult,
    IntoActiveModel,
    QuerySelect,
    Set,
    UpdateMany,
    UpdateResult,
};

use crate::entity::prelude::*;
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

    pub fn lyrics(mut self, checked: u32, profane: u32, genius: u32, musixmatch: u32) -> Self {
        self.0 = self
            .0
            .col_expr(
                UserColumn::LyricsChecked,
                Expr::col(UserColumn::LyricsChecked).add(checked),
            )
            .col_expr(
                UserColumn::LyricsProfane,
                Expr::col(UserColumn::LyricsProfane).add(profane),
            )
            .col_expr(
                UserColumn::LyricsGenius,
                Expr::col(UserColumn::LyricsGenius).add(genius),
            )
            .col_expr(
                UserColumn::LyricsMusixmatch,
                Expr::col(UserColumn::LyricsMusixmatch).add(musixmatch),
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
        db: &impl ConnectionTrait,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<bool> {
        let query: UpdateMany<_> = UserEntity::update_many();

        let update_result: UpdateResult = query
            .col_expr(UserColumn::PlayingTrack, Expr::value(track_id))
            .col_expr(UserColumn::UpdatedAt, Expr::value(Clock::now()))
            .filter(UserColumn::Id.eq(user_id))
            .filter(UserColumn::PlayingTrack.ne(track_id))
            .exec(db)
            .await?;

        Ok(update_result.rows_affected > 0)
    }

    async fn obtain_by_id(db: &impl ConnectionTrait, id: &str) -> anyhow::Result<UserActiveModel> {
        let user = Self::query(Some(id), None).one(db).await?;

        let user = match user {
            Some(spotify_auth) => spotify_auth,
            None => {
                UserActiveModel {
                    id: Set(id.to_owned()),
                    ..Default::default()
                }
                .insert(db)
                .await?
            },
        }
        .into_active_model();

        Ok(user)
    }

    pub async fn set_status(
        db: &impl ConnectionTrait,
        id: &str,
        status: UserStatus,
    ) -> anyhow::Result<UserActiveModel> {
        let mut user = Self::obtain_by_id(db, id).await?;

        user.status = Set(status);

        Ok(user.save(db).await?)
    }

    pub fn increase_stats_query(user_id: &str) -> UserStatsIncreaseQueryBuilder {
        UserStatsIncreaseQueryBuilder::new(user_id)
    }

    pub async fn get_stats(
        db: &impl ConnectionTrait,
        id: Option<&str>,
    ) -> anyhow::Result<UserStats> {
        let bigint = || Alias::new("bigint");
        let res = Self::query(id, None)
            .select_only()
            .column_as(
                UserColumn::RemovedCollection.sum().cast_as(bigint()),
                "removed_collection",
            )
            .column_as(
                UserColumn::RemovedPlaylists.sum().cast_as(bigint()),
                "removed_playlists",
            )
            .column_as(
                UserColumn::LyricsChecked.sum().cast_as(bigint()),
                "lyrics_checked",
            )
            .column_as(
                UserColumn::LyricsProfane.sum().cast_as(bigint()),
                "lyrics_profane",
            )
            .column_as(
                UserColumn::LyricsGenius.sum().cast_as(bigint()),
                "lyrics_genius",
            )
            .column_as(
                UserColumn::LyricsMusixmatch.sum().cast_as(bigint()),
                "lyrics_musixmatch",
            )
            .column_as(
                UserColumn::LyricsGenius
                    .sum()
                    .add(UserColumn::LyricsMusixmatch.sum())
                    .cast_as(bigint()),
                "lyrics_found",
            )
            .into_model::<UserStats>()
            .one(db)
            .await?
            .unwrap_or_default();

        Ok(res)
    }
}
