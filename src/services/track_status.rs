use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{ConnectionTrait, FromQueryResult, IntoActiveModel, QuerySelect, UpdateResult};

use crate::entity::prelude::*;
use crate::utils::Clock;

pub struct TrackStatusQueryBuilder(Select<TrackStatusEntity>);

impl TrackStatusQueryBuilder {
    fn new() -> Self {
        Self(TrackStatusEntity::find())
    }

    pub fn user_id(mut self, user_id: Option<&str>) -> Self {
        if let Some(user_id) = user_id {
            self.0 = self.0.filter(TrackStatusColumn::UserId.eq(user_id));
        }

        self
    }

    pub fn track_id(mut self, track_id: Option<&str>) -> Self {
        if let Some(track_id) = track_id {
            self.0 = self.0.filter(TrackStatusColumn::TrackId.eq(track_id));
        }

        self
    }

    pub fn status(mut self, status: Option<TrackStatus>) -> Self {
        if let Some(status) = status {
            self.0 = self.0.filter(TrackStatusColumn::Status.eq(status));
        }

        self
    }

    pub fn build(self) -> Select<TrackStatusEntity> {
        self.0
    }
}

pub struct TrackStatusService;

impl TrackStatusService {
    fn builder() -> TrackStatusQueryBuilder {
        TrackStatusQueryBuilder::new()
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn count_status(
        db: &impl ConnectionTrait,
        status: TrackStatus,
        user_id: Option<&str>,
        track_id: Option<&str>,
    ) -> anyhow::Result<u64> {
        let res = Self::builder()
            .status(Some(status))
            .user_id(user_id)
            .track_id(track_id)
            .build()
            .count(db)
            .await?;

        Ok(res)
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn sum_skips(
        db: &impl ConnectionTrait,
        user_id: Option<&str>,
    ) -> anyhow::Result<i64> {
        #[derive(FromQueryResult, Default)]
        struct SkipsCount {
            count: Option<i64>,
        }

        let skips: SkipsCount = Self::builder()
            .user_id(user_id)
            .build()
            .select_only()
            .column_as(
                TrackStatusColumn::Skips.sum().cast_as(Alias::new("bigint")),
                "count",
            )
            .into_model::<SkipsCount>()
            .one(db)
            .await?
            .unwrap_or_default();

        Ok(skips.count.unwrap_or_default())
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn set_status(
        db: &impl ConnectionTrait,
        user_id: &str,
        track_id: &str,
        status: TrackStatus,
    ) -> anyhow::Result<TrackStatusActiveModel> {
        let track_status = Self::builder()
            .track_id(Some(track_id))
            .user_id(Some(user_id))
            .build()
            .one(db)
            .await?;

        let mut track_status = match track_status {
            Some(track_status) => track_status.into_active_model(),
            None => TrackStatusActiveModel {
                track_id: Set(track_id.to_owned()),
                user_id: Set(user_id.to_owned()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .into_active_model(),
        };

        track_status.status = Set(status);

        Ok(track_status.save(db).await?)
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn get_status(
        db: &impl ConnectionTrait,
        user_id: &str,
        track_id: &str,
    ) -> TrackStatus {
        let track_status = Self::builder()
            .track_id(Some(track_id))
            .user_id(Some(user_id))
            .build()
            .one(db)
            .await;

        match track_status {
            Ok(Some(track_status)) => track_status.status,
            _ => TrackStatus::None,
        }
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn increase_skips(
        db: &impl ConnectionTrait,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<UpdateResult> {
        let update_result: UpdateResult = TrackStatusEntity::update_many()
            .col_expr(
                TrackStatusColumn::Skips,
                Expr::col(TrackStatusColumn::Skips).add(1),
            )
            .col_expr(TrackStatusColumn::UpdatedAt, Expr::value(Clock::now()))
            .filter(TrackStatusColumn::UserId.eq(user_id))
            .filter(TrackStatusColumn::TrackId.eq(track_id))
            .exec(db)
            .await?;

        Ok(update_result)
    }
}
