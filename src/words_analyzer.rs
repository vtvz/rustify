use anyhow::Context;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use chrono::Duration;
use indoc::formatdoc;
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
        let word_key = format!("rustify:word_profanity:{profane_word}");

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
        let prompt = formatdoc!(
            "
                You are given a word from a song: {profane_word}.
                1. Assume by default that the word is likely to be profane, unless clearly normal.
                2. Always explain what it means or how it translates into {language}, considering it appears in song lyrics.
                3. If the word is profane, also classify its profaneness as one of:
                - normal word
                - mildly profane
                - highly profane
                4. If the word is not profane, mark it as normal word without mentioning offensiveness.
                5. The answer must be strictly in {language}.
                6. The answer must be between 50 and 150 characters.
                7. The answer must be a single line, no line breaks.
                8. Do not include the given word itself or any other offensive words.
                9. Keep it clean and suitable for all audiences.
            ",
            language = state.language(),
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
                    .content(prompt)
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
