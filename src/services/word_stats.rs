use itertools::Itertools;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{Alias, OnConflict};
use sea_orm::{ConnectionTrait, EntityTrait as _};

use crate::entity::prelude::{WordStatsActiveModel, WordStatsColumn, WordStatsEntity};
use crate::utils::Clock;

pub struct WordStatsService {}

impl WordStatsService {
    #[tracing::instrument(skip_all)]
    pub async fn increase_check_occurence(
        db: &impl ConnectionTrait,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> anyhow::Result<()> {
        Self::increase_occurence(db, words, 0, 1, 0).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn increase_details_occurence(
        db: &impl ConnectionTrait,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> anyhow::Result<()> {
        Self::increase_occurence(db, words, 1, 0, 0).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn increase_analyze_occurence(
        db: &impl ConnectionTrait,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> anyhow::Result<()> {
        Self::increase_occurence(db, words, 0, 0, 1).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn increase_occurence(
        db: &impl ConnectionTrait,
        words: impl IntoIterator<Item = impl Into<String>>,
        details: i32,
        check: i32,
        analyze: i32,
    ) -> anyhow::Result<()> {
        let models = words
            .into_iter()
            .map(|word| WordStatsActiveModel {
                word: Set(word.into()),
                details_occurrences: Set(details),
                check_occurrences: Set(check),
                analyze_occurrences: Set(analyze),
                updated_at: Set(Clock::now()),
                ..Default::default()
            })
            .collect_vec();

        if models.is_empty() {
            return Ok(());
        }

        WordStatsEntity::insert_many(models)
            .on_conflict(
                OnConflict::column(WordStatsColumn::Word)
                    .value(
                        WordStatsColumn::CheckOccurrences,
                        Expr::col((WordStatsEntity, WordStatsColumn::CheckOccurrences)).add(
                            Expr::col((Alias::new("excluded"), WordStatsColumn::CheckOccurrences)),
                        ),
                    )
                    .value(
                        WordStatsColumn::DetailsOccurrences,
                        Expr::col((WordStatsEntity, WordStatsColumn::DetailsOccurrences)).add(
                            Expr::col((
                                Alias::new("excluded"),
                                WordStatsColumn::DetailsOccurrences,
                            )),
                        ),
                    )
                    .value(
                        WordStatsColumn::AnalyzeOccurrences,
                        Expr::col((WordStatsEntity, WordStatsColumn::AnalyzeOccurrences)).add(
                            Expr::col((
                                Alias::new("excluded"),
                                WordStatsColumn::AnalyzeOccurrences,
                            )),
                        ),
                    )
                    .to_owned(),
            )
            .exec(db)
            .await?;

        Ok(())
    }
}
