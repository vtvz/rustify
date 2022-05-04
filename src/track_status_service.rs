use chrono::Utc;
use rspotify::model::{Id, TrackId};
use sea_orm::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::ActiveValue::Set;
use sea_orm::{ConnectionTrait, FromQueryResult, IntoActiveModel, QuerySelect, UpdateResult};

use crate::entity::prelude::*;
use crate::errors::GenericResult;

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

    pub async fn count_status(
        db: &impl ConnectionTrait,
        status: TrackStatus,
        user_id: Option<&str>,
        track_id: Option<&str>,
    ) -> GenericResult<usize> {
        let res = Self::builder()
            .status(Some(status))
            .user_id(user_id)
            .track_id(track_id)
            .build()
            .count(db)
            .await?;

        Ok(res)
    }

    pub async fn sum_skips(db: &impl ConnectionTrait, user_id: Option<&str>) -> GenericResult<u32> {
        #[derive(FromQueryResult, Default)]
        struct SkipsCount {
            count: u32,
        }

        let skips: SkipsCount = Self::builder()
            .user_id(user_id)
            .build()
            .select_only()
            .column_as(TrackStatusColumn::Skips.sum(), "count")
            .into_model::<SkipsCount>()
            .one(db)
            .await?
            .unwrap_or_default();

        Ok(skips.count)
    }

    pub async fn set_status(
        db: &impl ConnectionTrait,
        user_id: &str,
        track_id: &str,
        status: TrackStatus,
    ) -> GenericResult<TrackStatusActiveModel> {
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

    pub async fn get_ids_with_status(
        db: &impl ConnectionTrait,
        user_id: &str,
        status: TrackStatus,
    ) -> GenericResult<Vec<TrackId>> {
        let tracks: Vec<TrackStatusModel> = Self::builder()
            .status(Some(status))
            .user_id(Some(user_id))
            .build()
            .all(db)
            .await?;

        let res: Vec<_> = tracks
            .iter()
            .map(|track| TrackId::from_id(track.track_id.as_ref()))
            .collect::<Result<_, _>>()?;

        Ok(res)
    }

    pub async fn increase_skips(
        db: &impl ConnectionTrait,
        user_id: &str,
        track_id: &str,
    ) -> GenericResult<UpdateResult> {
        let update_result: UpdateResult = TrackStatusEntity::update_many()
            .col_expr(
                TrackStatusColumn::Skips,
                Expr::col(TrackStatusColumn::Skips).add(1),
            )
            .col_expr(
                TrackStatusColumn::UpdatedAt,
                Expr::value(Utc::now().naive_local()),
            )
            .filter(TrackStatusColumn::UserId.eq(user_id))
            .filter(TrackStatusColumn::TrackId.eq(track_id))
            .exec(db)
            .await?;

        Ok(update_result)
    }
}
