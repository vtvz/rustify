use anyhow::Context;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ColumnTrait,
    ConnectionTrait,
    DeleteResult,
    EntityTrait,
    PaginatorTrait,
    QueryFilter,
    QueryOrder,
    QuerySelect,
};

use crate::entity::prelude::{
    WordDefinitionActiveModel,
    WordDefinitionColumn,
    WordDefinitionEntity,
    WordStatsColumn,
    WordStatsEntity,
};
use crate::app::AIConfig;

pub struct WordDefinitionService {}

#[derive(Debug)]
pub struct DefinitionWithStats {
    pub word: String,
    pub locale: String,
    pub definition: String,
    pub check_occurrences: i32,
    pub details_occurrences: i32,
    pub analyze_occurrences: i32,
}

impl WordDefinitionService {
    pub async fn get_definition(
        db: &impl ConnectionTrait,
        locale: &str,
        config: &AIConfig,
        profane_word: &str,
    ) -> anyhow::Result<String> {
        let definition: Option<String> = WordDefinitionEntity::find()
            .select_only()
            .column(WordDefinitionColumn::Definition)
            .filter(WordDefinitionColumn::Word.eq(profane_word))
            .filter(WordDefinitionColumn::Locale.eq(locale))
            .into_tuple()
            .one(db)
            .await?;

        if let Some(definition) = definition {
            return Ok(definition);
        }

        let definition = Self::get_definition_internal(config, locale, profane_word).await?;

        let model = WordDefinitionActiveModel {
            word: Set(profane_word.into()),
            definition: Set(definition.clone()),
            locale: Set(locale.into()),
            ..Default::default()
        };

        WordDefinitionEntity::insert(model).exec(db).await?;

        Ok(definition)
    }

    pub async fn get_definition_internal(
        config: &AIConfig,
        locale: &str,
        profane_word: &str,
    ) -> anyhow::Result<String> {
        let prompt = t!(
            "analysis.word-analyzer-prompt",
            profane_word = profane_word,
            locale = locale,
        );

        let req = CreateChatCompletionRequestArgs::default()
            .model(config.model())
            .temperature(0.5)
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are a helpful assistant.")
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt.as_ref())
                    .build()?
                    .into(),
            ])
            .build()?;

        let res = config
            .openai_client()
            .chat()
            .create(req)
            .await?
            .choices
            .first()
            .context("I need at least one choice")?
            .message
            .clone()
            .content
            .context("I need message content")?;

        Ok(res)
    }

    pub async fn clear_definition(
        db: &impl ConnectionTrait,
        locale: &str,
        profane_word: &str,
    ) -> anyhow::Result<DeleteResult> {
        let result = WordDefinitionEntity::delete_many()
            .filter(WordDefinitionColumn::Word.eq(profane_word))
            .filter(WordDefinitionColumn::Locale.eq(locale))
            .exec(db)
            .await?;

        Ok(result)
    }

    pub async fn list_definitions_with_stats(
        db: &impl ConnectionTrait,
        locale: &str,
        page: usize,
        page_size: usize,
    ) -> anyhow::Result<Vec<DefinitionWithStats>> {
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
                .find(|definition| definition.word == stat.word);

            let Some(definition) = definition else {
                continue;
            };

            let with_stats = DefinitionWithStats {
                word: stat.word.clone(),
                locale: locale.to_string(),
                definition: definition.definition.clone(),
                check_occurrences: stat.check_occurrences,
                details_occurrences: stat.details_occurrences,
                analyze_occurrences: stat.analyze_occurrences,
            };

            result.push(with_stats);
        }

        Ok(result)
    }

    pub async fn count_definitions_with_stats(
        db: &impl ConnectionTrait,
        locale: &str,
    ) -> anyhow::Result<usize> {
        use sea_orm::sea_query::Query;

        let subquery = Query::select()
            .column(WordStatsColumn::Word)
            .from(WordStatsEntity)
            .to_owned();

        let count = WordDefinitionEntity::find()
            .filter(WordDefinitionColumn::Locale.eq(locale))
            .filter(WordDefinitionColumn::Word.in_subquery(subquery))
            .count(db)
            .await? as usize;

        Ok(count)
    }
}
