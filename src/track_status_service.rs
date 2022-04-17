use chrono::Utc;

use rspotify::model::{Id, TrackId};
use sea_orm::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::ActiveValue::Set;
use sea_orm::FromQueryResult;
use sea_orm::IntoActiveModel;
use sea_orm::QuerySelect;
use sea_orm::{DbConn, UpdateResult};

use crate::entity::prelude::*;

pub use crate::entity::track_status::Status;

pub struct TrackStatusService;

impl TrackStatusService {
    fn query(
        status: Option<Status>,
        user_id: Option<&str>,
        track_id: Option<&str>,
    ) -> Select<TrackStatusEntity> {
        let mut query: Select<_> = TrackStatusEntity::find();

        if let Some(user_id) = user_id {
            query = query.filter(TrackStatusColumn::UserId.eq(user_id));
        };

        if let Some(track_id) = track_id {
            query = query.filter(TrackStatusColumn::TrackId.eq(track_id));
        };

        if let Some(status) = status {
            query = query.filter(TrackStatusColumn::Status.eq(status));
        };

        query
    }

    pub async fn count_status(
        db: &DbConn,
        status: Status,
        user_id: Option<&str>,
        track_id: Option<&str>,
    ) -> anyhow::Result<usize> {
        let res = Self::query(Some(status), user_id, track_id)
            .count(db)
            .await?;

        Ok(res)
    }

    pub async fn sum_skips(db: &DbConn, user_id: Option<&str>) -> anyhow::Result<u32> {
        #[derive(FromQueryResult, Default)]
        struct SkipsCount {
            count: u32,
        }

        let skips: SkipsCount = Self::query(None, user_id, None)
            .select_only()
            .column_as(TrackStatusColumn::Skips.sum(), "count")
            .into_model::<SkipsCount>()
            .one(db)
            .await?
            .unwrap_or_default();

        Ok(skips.count)
    }

    pub async fn set_status(
        db: &DbConn,
        user_id: &str,
        track_id: &str,
        status: Status,
    ) -> anyhow::Result<TrackStatusActiveModel> {
        let track_status = Self::query(None, Some(user_id), Some(track_id))
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

    pub async fn get_status(db: &DbConn, user_id: &str, track_id: &str) -> Status {
        let track_status = Self::query(None, Some(user_id), Some(track_id))
            .one(db)
            .await;

        match track_status {
            Ok(Some(track_status)) => track_status.status,
            _ => Status::None,
        }
    }

    pub async fn get_ids_with_status(
        db: &DbConn,
        user_id: &str,
        status: Status,
    ) -> anyhow::Result<Vec<TrackId>> {
        let tracks: Vec<TrackStatusModel> = Self::query(Some(status), Some(user_id), None)
            .all(db)
            .await?;

        let res: Vec<_> = tracks
            .iter()
            .map(|track| TrackId::from_id(track.track_id.as_ref()))
            .collect::<Result<_, _>>()?;

        Ok(res)
    }

    pub async fn increase_skips(
        db: &DbConn,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<UpdateResult> {
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
