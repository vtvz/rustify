use anyhow::Context as _;
use rspotify::Token;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ConnectionTrait,
    IntoActiveModel as _,
    QuerySelect as _,
    QueryTrait as _,
    UpdateResult,
};

use crate::entity::prelude::*;
use crate::services::UserService;
use crate::utils::Clock;

pub struct SpotifyAuthService;

impl SpotifyAuthService {
    pub async fn set_token(
        db: &impl ConnectionTrait,
        user_id: &str,
        token: Token,
    ) -> anyhow::Result<SpotifyAuthActiveModel> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.eq(user_id))
            .one(db)
            .await?;

        let mut spotify_auth = match spotify_auth {
            Some(spotify_auth) => spotify_auth,
            None => {
                SpotifyAuthActiveModel {
                    user_id: Set(user_id.to_owned()),
                    ..Default::default()
                }
                .insert(db)
                .await?
            },
        }
        .into_active_model();

        spotify_auth.access_token = Set(token.access_token);
        spotify_auth.refresh_token =
            Set(token.refresh_token.context("Refresh token is required")?);
        spotify_auth.expires_at = Set(token.expires_at.map(|item| item.naive_utc()));

        Ok(spotify_auth.save(db).await?)
    }

    pub async fn get_token(
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<Option<Token>> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.eq(user_id))
            .one(db)
            .await?;

        let Some(spotify_auth) = spotify_auth else {
            return Ok(None);
        };

        Ok(Some(Token {
            access_token: spotify_auth.access_token,
            refresh_token: Some(spotify_auth.refresh_token),
            expires_at: spotify_auth.expires_at.map(|item| item.and_utc()),
            ..Default::default()
        }))
    }

    pub async fn suspend_until(
        db: &impl ConnectionTrait,
        user_id: &str,
        time: chrono::NaiveDateTime,
    ) -> anyhow::Result<UpdateResult> {
        let update_result: UpdateResult = SpotifyAuthEntity::update_many()
            .col_expr(SpotifyAuthColumn::UpdatedAt, Expr::value(Clock::now()))
            .col_expr(SpotifyAuthColumn::SuspendUntil, Expr::value(time))
            .filter(SpotifyAuthColumn::UserId.eq(user_id))
            .exec(db)
            .await?;

        Ok(update_result)
    }

    pub async fn suspend_for(
        db: &impl ConnectionTrait,
        user_id: &str,
        duration: chrono::Duration,
    ) -> anyhow::Result<UpdateResult> {
        let suspend_until = Clock::now() + duration;

        Self::suspend_until(db, user_id, suspend_until).await
    }

    pub async fn get_active_unsuspended_user_ids(
        db: &impl ConnectionTrait,
    ) -> anyhow::Result<Vec<String>> {
        let subquery: Select<UserEntity> = UserService::query(None, Some(UserStatus::Active))
            .select_only()
            .column(UserColumn::Id);

        let query = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.in_subquery(subquery.into_query()))
            .filter(SpotifyAuthColumn::SuspendUntil.lte(Clock::now()));

        let auths: Vec<SpotifyAuthModel> = query.all(db).await?;

        let user_ids: Vec<String> = auths.into_iter().map(|item| item.user_id).collect();

        Ok(user_ids)
    }
}
