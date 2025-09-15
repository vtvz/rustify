use anyhow::Context;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use chrono::Duration;
use redis::AsyncTypedCommands;

use crate::app::{AIConfig, App};
use crate::user::UserState;

pub struct WordAnalyzer {}

impl WordAnalyzer {
    pub async fn get_definition(
        app: &App,
        state: &UserState,
        config: &AIConfig,
        profane_word: &str,
    ) -> anyhow::Result<String> {
        let word_key = format!(
            "rustify:word_profanity:{profane_word}:{locale}",
            locale = state.locale()
        );

        let mut redis = app.redis_conn().await?;

        let definition: Option<String> = redis.get(&word_key).await?;

        if let Some(definition) = definition {
            return Ok(definition);
        }

        let definition = Self::get_definition_internal(state, config, profane_word).await?;

        let ttl = Duration::days(30);

        let _: () = redis
            .set_ex(word_key, &definition, ttl.num_seconds() as _)
            .await?;

        Ok(definition)
    }

    pub async fn get_definition_internal(
        state: &UserState,
        config: &AIConfig,
        profane_word: &str,
    ) -> anyhow::Result<String> {
        let prompt = t!(
            "analysis.word-analyzer-prompt",
            profane_word = profane_word,
            locale = state.locale()
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
}
