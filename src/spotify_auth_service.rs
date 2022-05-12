use rspotify::Token;
use sea_orm::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ConnectionTrait, IntoActiveModel, QuerySelect, QueryTrait};

use crate::entity::prelude::*;
use crate::errors::{Context, GenericResult};
use crate::user_service::UserService;
use crate::utils::Clock;

pub struct SpotifyAuthService;

impl SpotifyAuthService {
    pub async fn set_token(
        db: &impl ConnectionTrait,
        user_id: &str,
        token: Token,
    ) -> GenericResult<SpotifyAuthActiveModel> {
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
        spotify_auth.expires_at = Set(token.expires_at);

        Ok(spotify_auth.save(db).await?)
    }

    pub async fn get_token(
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> GenericResult<Option<Token>> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.eq(user_id))
            .one(db)
            .await?;

        let spotify_auth = match spotify_auth {
            Some(spotify_auth) => spotify_auth,
            None => return Ok(None),
        };

        Ok(Some(Token {
            access_token: spotify_auth.access_token,
            refresh_token: Some(spotify_auth.refresh_token),
            expires_at: spotify_auth.expires_at,
            ..Default::default()
        }))
    }

    pub async fn remove_token(
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> Result<sea_orm::DeleteResult, sea_orm::DbErr> {
        SpotifyAuthEntity::delete_by_id(user_id.to_owned())
            .exec(db)
            .await
    }

    pub async fn suspend_util(
        db: &impl ConnectionTrait,
        user_id: &str,
        time: chrono::NaiveDateTime,
    ) -> GenericResult<bool> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.eq(user_id))
            .one(db)
            .await?;

        let mut spotify_auth = match spotify_auth {
            Some(spotify_auth) => spotify_auth.into_active_model(),
            None => return Ok(false),
        };

        spotify_auth.suspend_until = Set(time);

        spotify_auth.save(db).await?;

        Ok(true)
    }

    pub async fn suspend_for(
        db: &impl ConnectionTrait,
        user_id: &str,
        duration: chrono::Duration,
    ) -> GenericResult<bool> {
        let suspend_until = Clock::now() + duration;

        SpotifyAuthService::suspend_util(db, user_id, suspend_until).await
    }

    pub async fn get_registered(db: &impl ConnectionTrait) -> GenericResult<Vec<String>> {
        let subquery: Select<UserEntity> = UserService::query(None, Some(UserStatus::Active))
            .select_only()
            .column(UserColumn::Id);

        let query = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.in_subquery(subquery.into_query()))
            .filter(SpotifyAuthColumn::SuspendUntil.lte(Clock::now()));

        let auths: Vec<SpotifyAuthModel> = match query.all(db).await {
            Ok(auths) => auths,
            Err(err) => return Err(err.into()),
        };

        let user_ids: Vec<String> = auths.iter().map(|item| item.user_id.clone()).collect();

        Ok(user_ids)
    }
}
