use core::str::FromStr;

use rspotify::model::{Id, TrackId};
use sea_orm::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::IntoActiveModel;
use sea_orm::{DbConn, NotSet};
use strum_macros::{AsRefStr, EnumString};

use crate::entity;
use crate::entity::prelude::TrackStatus as TrackStatusEntity;
use crate::entity::track_status;

#[derive(Clone, EnumString, AsRefStr)]
pub enum Status {
    #[strum(serialize = "disliked")]
    Disliked,
    #[strum(serialize = "ignore")]
    Ignore,
    #[strum(serialize = "none")]
    None,
}

impl Default for Status {
    fn default() -> Self {
        Self::None
    }
}

pub struct TrackStatusService;

impl TrackStatusService {
    pub async fn count_status(
        db: &DbConn,
        user_id: String,
        status: Status,
    ) -> anyhow::Result<usize> {
        let res = TrackStatusEntity::find()
            .filter(entity::track_status::Column::UserId.eq(user_id))
            .filter(entity::track_status::Column::Status.eq(status.as_ref()))
            .count(db)
            .await?;

        Ok(res)
    }

    pub async fn set_status(
        db: &DbConn,
        user_id: String,
        track_id: String,
        status: Status,
    ) -> anyhow::Result<entity::track_status::ActiveModel> {
        let track_status = TrackStatusEntity::find()
            .filter(entity::track_status::Column::TrackId.eq(track_id.clone()))
            .filter(entity::track_status::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        let mut track_status = match track_status {
            Some(track_status) => track_status.into_active_model(),
            None => entity::track_status::ActiveModel {
                track_id: Set(track_id),
                user_id: Set(user_id),
                ..Default::default()
            }
            .insert(db)
            .await?
            .into_active_model(),
        };

        track_status.status = Set(status.as_ref().to_owned());
        track_status.updated_at = NotSet;

        Ok(track_status.save(db).await?)
    }

    pub async fn get_status(db: &DbConn, user_id: String, track_id: String) -> Status {
        let track_status = TrackStatusEntity::find()
            .filter(entity::track_status::Column::TrackId.eq(track_id.clone()))
            .filter(entity::track_status::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await;

        let track_status = match track_status {
            Ok(track_status) => track_status,
            Err(_) => return Status::None,
        };

        match track_status {
            Some(track_status) => {
                let res: Result<Status, _> = Status::from_str(track_status.status.as_ref());

                res.unwrap_or_default()
            }
            None => Status::None,
        }
    }

    pub async fn get_ids_with_status(
        db: &DbConn,
        user_id: String,
        status: Status,
    ) -> anyhow::Result<Vec<TrackId>> {
        let tracks: Vec<track_status::Model> = TrackStatusEntity::find()
            .filter(entity::track_status::Column::UserId.eq(user_id.clone()))
            .filter(entity::track_status::Column::Status.eq(status.as_ref()))
            .all(db)
            .await?;

        let res: Vec<_> = tracks
            .iter()
            .map(|track| TrackId::from_id(track.track_id.as_ref()))
            .collect::<Result<_, _>>()?;

        Ok(res)
    }
}
