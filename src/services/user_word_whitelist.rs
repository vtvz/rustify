use std::collections::HashSet;

use sea_orm::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ConnectionTrait, FromQueryResult, QuerySelect, Set};

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

    pub async fn count_ok_words_for_user(
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<u64> {
        let count = UserWordWhitelistEntity::find()
            .select_only()
            .filter(UserWordWhitelistColumn::UserId.eq(user_id))
            .count(db)
            .await?;

        Ok(count)
    }

    pub async fn add_ok_word_for_user(
        db: &impl ConnectionTrait,
        user_id: String,
        word: String,
    ) -> anyhow::Result<bool> {
        let rec = UserWordWhitelistActiveModel {
            user_id: Set(user_id),
            word: Set(word.to_lowercase()),
            ..Default::default()
        };

        let res = UserWordWhitelistEntity::insert(rec)
            .on_conflict(
                OnConflict::columns(vec![
                    UserWordWhitelistColumn::UserId,
                    UserWordWhitelistColumn::Word,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(db)
            .await;

        match res {
            Err(DbErr::RecordNotInserted) => Ok(false),
            Err(err) => Err(err.into()),
            Ok(_) => Ok(true),
        }
    }

    pub async fn remove_ok_word_for_user(
        db: &impl ConnectionTrait,
        user_id: &str,
        word: &str,
    ) -> anyhow::Result<bool> {
        let res = UserWordWhitelistEntity::delete_many()
            .filter(UserWordWhitelistColumn::UserId.eq(user_id))
            .filter(UserWordWhitelistColumn::Word.eq(word.to_lowercase()))
            .exec(db)
            .await?;

        Ok(res.rows_affected > 0)
    }
}
