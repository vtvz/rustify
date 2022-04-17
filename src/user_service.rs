use chrono::Utc;
use sea_orm::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{ConnectionTrait, IntoActiveModel};
use sea_orm::{FromQueryResult, QuerySelect};
use sea_orm::{Set, UpdateMany, UpdateResult};

use crate::entity::prelude::*;

#[derive(FromQueryResult, Default)]
pub struct UserStats {
    pub removed_playlists: u32,
    pub removed_collection: u32,
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
            .col_expr(UserColumn::UpdatedAt, Expr::value(Utc::now().naive_local()))
            .filter(UserColumn::Id.eq(id))
            .filter(UserColumn::Name.ne(name))
            .exec(db)
            .await?;

        Ok(update_result)
    }

    pub async fn set_status(
        db: &impl ConnectionTrait,
        id: &str,
        status: UserStatus,
    ) -> anyhow::Result<UserActiveModel> {
        let user = Self::query(Some(id), None).one(db).await?;

        let mut user = match user {
            Some(spotify_auth) => spotify_auth,
            None => {
                UserActiveModel {
                    id: Set(id.to_owned()),
                    ..Default::default()
                }
                .insert(db)
                .await?
            }
        }
        .into_active_model();

        user.status = Set(status);

        Ok(user.save(db).await?)
    }

    pub async fn increase_stats(
        db: &impl ConnectionTrait,
        user_id: &str,
        removed_playlists: u32,
        removed_collection: u32,
    ) -> anyhow::Result<UpdateResult> {
        let query: UpdateMany<_> = UserEntity::update_many();

        let update_result: UpdateResult = query
            .col_expr(
                UserColumn::RemovedCollection,
                Expr::col(UserColumn::RemovedCollection).add(removed_collection),
            )
            .col_expr(
                UserColumn::RemovedPlaylists,
                Expr::col(UserColumn::RemovedPlaylists).add(removed_playlists),
            )
            .col_expr(UserColumn::UpdatedAt, Expr::value(Utc::now().naive_local()))
            .filter(UserColumn::Id.eq(user_id))
            .exec(db)
            .await?;

        Ok(update_result)
    }

    pub async fn get_stats(
        db: &impl ConnectionTrait,
        id: Option<&str>,
    ) -> anyhow::Result<UserStats> {
        let res = Self::query(id, None)
            .select_only()
            .column_as(UserColumn::RemovedCollection.sum(), "removed_collection")
            .column_as(UserColumn::RemovedPlaylists.sum(), "removed_playlists")
            .into_model::<UserStats>()
            .one(db)
            .await?
            .unwrap_or_default();

        Ok(res)
    }
}
