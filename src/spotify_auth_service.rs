use anyhow::anyhow;
use chrono::Utc;
use rspotify::Token;
use sea_orm::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::DbConn;
use sea_orm::IntoActiveModel;

use crate::entity::prelude::*;

pub struct SpotifyAuthService;

impl SpotifyAuthService {
    pub async fn set_token(
        db: &DbConn,
        user_id: &str,
        token: Token,
    ) -> anyhow::Result<SpotifyAuthActiveModel> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(SpotifyAuthColumn::UserId.eq(user_id))
            .one(db)
            .await?;

        let mut spotify_auth = match spotify_auth {
            Some(spotify_auth) => spotify_auth.into_active_model(),
            None => SpotifyAuthActiveModel {
                user_id: Set(user_id.to_owned()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .into_active_model(),
        };

        spotify_auth.access_token = Set(token.access_token);
        spotify_auth.refresh_token = Set(token
            .refresh_token
            .ok_or_else(|| anyhow!("Refresh token is required"))?);
        spotify_auth.expires_at = Set(token.expires_at);

        spotify_auth.updated_at = Set(Utc::now().naive_local());

        Ok(spotify_auth.save(db).await?)
    }

    pub async fn get_token(db: &DbConn, user_id: &str) -> anyhow::Result<Option<Token>> {
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
        db: &DbConn,
        user_id: &str,
    ) -> Result<sea_orm::DeleteResult, sea_orm::DbErr> {
        SpotifyAuthEntity::delete_by_id(user_id.to_owned())
            .exec(db)
            .await
    }

    pub async fn get_registered(db: &DbConn) -> anyhow::Result<Vec<String>> {
        let auths: Vec<SpotifyAuthModel> = match SpotifyAuthEntity::find().all(db).await {
            Ok(auths) => auths,
            Err(err) => return Err(anyhow!(err)),
        };

        let user_ids: Vec<String> = auths.iter().map(|item| item.user_id.clone()).collect();

        Ok(user_ids)
    }
}
