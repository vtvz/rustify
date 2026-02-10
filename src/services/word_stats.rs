use itertools::Itertools as _;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{Alias, OnConflict};
use sea_orm::{
    ColumnTrait as _,
    ConnectionTrait,
    EntityTrait as _,
    PaginatorTrait as _,
    QueryFilter as _,
    QueryOrder as _,
    QuerySelect as _,
};

use crate::entity::prelude::{
    WordDefinitionColumn,
    WordDefinitionEntity,
    WordStatsActiveModel,
    WordStatsColumn,
    WordStatsEntity,
};
use crate::utils::Clock;

pub struct WordStatsService {}

#[derive(Debug)]
pub struct StatsWithDefinition {
    pub word: String,
    pub locale: String,
    pub definition: Option<String>,
    pub check_occurrences: i32,
    pub details_occurrences: i32,
    pub analyze_occurrences: i32,
}

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
    async fn increase_occurence(
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

    #[tracing::instrument(skip_all)]
    pub async fn count_stats(db: &impl ConnectionTrait) -> anyhow::Result<usize> {
        let count = WordStatsEntity::find().count(db).await? as usize;

        Ok(count)
    }

    #[tracing::instrument(skip_all, fields(%locale, %page, %page_size))]
    pub async fn list_stats_with_definitions(
        db: &impl ConnectionTrait,
        locale: &str,
        page: usize,
        page_size: usize,
    ) -> anyhow::Result<Vec<StatsWithDefinition>> {
        let offset = page * page_size;

        let stats = WordStatsEntity::find()
            .order_by_desc(WordStatsColumn::CheckOccurrences)
            .order_by_desc(WordStatsColumn::DetailsOccurrences)
            .order_by_desc(WordStatsColumn::AnalyzeOccurrences)
            .limit(page_size as u64)
            .offset(offset as u64)
            .all(db)
            .await?;

        let definitions = WordDefinitionEntity::find()
            .filter(WordDefinitionColumn::Locale.eq(locale))
            .filter(WordDefinitionColumn::Word.is_in(stats.iter().map(|item| &item.word)))
            .all(db)
            .await?;

        let mut result = Vec::new();
        for stat in stats {
            let definition = definitions
                .iter()
                .find(|definition| definition.word == stat.word)
                .map(|model| model.definition.clone());

            let with_stats = StatsWithDefinition {
                word: stat.word.clone(),
                locale: locale.to_owned(),
                definition,
                check_occurrences: stat.check_occurrences,
                details_occurrences: stat.details_occurrences,
                analyze_occurrences: stat.analyze_occurrences,
            };

            result.push(with_stats);
        }

        Ok(result)
    }
}
