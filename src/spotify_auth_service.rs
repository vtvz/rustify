use anyhow::anyhow;
use rspotify::Token;
use sea_orm::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::DbConn;
use sea_orm::IntoActiveModel;

use crate::entity;
use crate::entity::prelude::SpotifyAuth as SpotifyAuthEntity;
use crate::entity::spotify_auth::Model;

pub struct SpotifyAuthService;

impl SpotifyAuthService {
    pub async fn set_token(
        db: &DbConn,
        user_id: String,
        token: Token,
    ) -> anyhow::Result<entity::spotify_auth::ActiveModel> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(entity::spotify_auth::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        let mut spotify_auth = match spotify_auth {
            Some(spotify_auth) => spotify_auth.into_active_model(),
            None => entity::spotify_auth::ActiveModel {
                user_id: Set(user_id),
                ..Default::default()
            }
            .insert(db)
            .await?
            .into_active_model(),
        };

        spotify_auth.access_token = Set(token.access_token);
        spotify_auth.refresh_token = Set(token
            .refresh_token
            .ok_or(anyhow!("Refresh token is required"))?);

        Ok(spotify_auth.save(db).await?)
    }

    pub async fn get_token(db: &DbConn, user_id: String) -> anyhow::Result<Option<Token>> {
        let spotify_auth = SpotifyAuthEntity::find()
            .filter(entity::spotify_auth::Column::UserId.eq(user_id.clone()))
            .one(db)
            .await?;

        let spotify_auth = match spotify_auth {
            Some(spotify_auth) => spotify_auth,
            None => return Ok(None),
        };

        Ok(Some(Token {
            access_token: spotify_auth.access_token,
            refresh_token: Some(spotify_auth.refresh_token),
            ..Default::default()
        }))
    }

    pub async fn get_registered(db: &DbConn) -> anyhow::Result<Vec<String>> {
        let auths: Vec<Model> = match SpotifyAuthEntity::find().all(db).await {
            Ok(auths) => auths,
            Err(err) => return Err(anyhow!(err)),
        };

        let user_ids: Vec<String> = auths.iter().map(|item| item.user_id.clone()).collect();

        Ok(user_ids)
    }
}
