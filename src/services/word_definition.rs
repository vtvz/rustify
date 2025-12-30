use anyhow::Context;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage,
    CreateChatCompletionRequestArgs,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ColumnTrait,
    ConnectionTrait,
    DeleteResult,
    EntityTrait,
    QueryFilter as _,
    QuerySelect,
};

use crate::app::AIConfig;
use crate::entity::prelude::{
    WordDefinitionActiveModel,
    WordDefinitionColumn,
    WordDefinitionEntity,
};

pub struct WordDefinitionService {}

impl WordDefinitionService {
    #[tracing::instrument(skip_all, fields(locale, profane_word))]
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

    #[tracing::instrument(skip_all, fields(locale, profane_word))]
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
            // .temperature(0.5)
            .messages([
                ChatCompletionRequestSystemMessage::from("You are a helpful assistant.").into(),
                ChatCompletionRequestUserMessage::from(prompt.as_ref()).into(),
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

    #[tracing::instrument(skip_all, fields(locale, profane_word))]
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
}
