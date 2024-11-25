use sea_orm::ActiveValue::Set;
use sea_orm::prelude::*;
use sea_orm::{ConnectionTrait, IntoActiveModel};

use crate::entity::prelude::*;

pub struct Manager {
    enabled: bool,
    admin: String,
}

impl Manager {
    pub fn from_env() -> Self {
        Self {
            enabled: dotenv::var("WHITELIST_ENABLED")
                .ok()
                .and_then(|val| val.parse().ok())
                .unwrap_or_default(),
            admin: dotenv::var("WHITELIST_ADMIN").unwrap_or_default(),
        }
    }

    pub fn is_admin(&self, user_id: &str) -> bool {
        self.admin == user_id
    }

    pub fn contact_admin(&self) -> &str {
        &self.admin
    }

    async fn get_by_user_id(
        &self,
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<(UserWhitelistModel, bool)> {
        let model = UserWhitelistEntity::find()
            .filter(UserWhitelistColumn::UserId.eq(user_id))
            .one(db)
            .await?;

        let res = match model {
            Some(model) => (model, false),
            None => {
                let model = UserWhitelistActiveModel {
                    user_id: Set(user_id.to_owned()),
                    ..Default::default()
                }
                .insert(db)
                .await?;

                (model, true)
            },
        };

        Ok(res)
    }

    pub async fn get_status(
        &self,
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<(UserWhitelistStatus, bool)> {
        if !self.enabled || self.admin == user_id {
            return Ok((UserWhitelistStatus::Allowed, false));
        }

        let (model, is_new) = self.get_by_user_id(db, user_id).await?;

        Ok((model.status, is_new))
    }

    pub async fn allow(
        &self,
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<UserWhitelistActiveModel> {
        self.set_status(db, user_id, UserWhitelistStatus::Allowed)
            .await
    }

    pub async fn deny(
        &self,
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<UserWhitelistActiveModel> {
        self.set_status(db, user_id, UserWhitelistStatus::Denied)
            .await
    }

    async fn set_status(
        &self,
        db: &impl ConnectionTrait,
        user_id: &str,
        status: UserWhitelistStatus,
    ) -> anyhow::Result<UserWhitelistActiveModel> {
        let mut model = self
            .get_by_user_id(db, user_id)
            .await?
            .0
            .into_active_model();

        model.status = Set(status);

        Ok(model.save(db).await?)
    }
}
