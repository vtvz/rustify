use std::collections::HashSet;

use sea_orm::prelude::*;
use sea_orm::{ConnectionTrait, FromQueryResult, QuerySelect};

use crate::entity::prelude::*;

pub struct UserWordWhitelistService;

impl UserWordWhitelistService {
    pub async fn get_ok_words_for_user(
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<HashSet<String>> {
        #[derive(FromQueryResult, Default)]
        struct OkWords {
            word: String,
        }

        let ok_words: HashSet<String> = UserWordWhitelistEntity::find()
            .select_only()
            .filter(UserWordWhitelistColumn::UserId.eq(user_id))
            .column(UserWordWhitelistColumn::Word)
            .into_model::<OkWords>()
            .all(db)
            .await?
            .into_iter()
            .map(|s| s.word)
            .collect();

        Ok(ok_words)
    }
}
