use anyhow::Context as _;
use async_openai::types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use backon::{ExponentialBuilder, Retryable};
use itertools::Itertools;
use rspotify::model::TrackId;
use rspotify::prelude::BaseClient as _;
use teloxide::payloads::{AnswerCallbackQuerySetters as _, EditMessageTextSetters as _};
use teloxide::prelude::Requester as _;
use teloxide::types::{CallbackQuery, InlineKeyboardMarkup, ParseMode, UserId};

use crate::app::{AIConfig, App};
use crate::profanity;
use crate::services::{TrackStatusService, UserService, WordDefinitionService, WordStatsService};
use crate::spotify::ShortTrack;
use crate::telegram::MESSAGE_MAX_LEN;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::StringUtils;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), track_id))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    track_id: &str,
) -> anyhow::Result<()> {
    let chat_id = q.from.id;

    let Some(config) = app.ai() else {
        app.bot()
            .answer_callback_query(q.id)
            .text(t!("analysis.disabled", locale = state.locale()))
            .await?;

        return Ok(());
    };

    let message_id = q.message.clone().context("Message is empty")?.id();

    let track = state
        .spotify()
        .await
        .track(TrackId::from_id(track_id)?, None)
        .await?;

    let track = ShortTrack::new(track);

    let Some(hit) = app.lyrics().search_for_track(&track).await? else {
        app.bot()
            .edit_message_text(
                chat_id,
                message_id,
                t!("analysis.lyrics-not-found", locale = state.locale()),
            )
            .await?;

        return Ok(());
    };

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!(
                "analysis.waiting",
                locale = state.locale(),
                track_name = track.track_tg_link(),
                album_name = track.album_tg_link(),
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .await?;

    let res = perform(
        app,
        state,
        chat_id,
        message_id,
        config,
        &track,
        &hit.lyrics(),
    )
    .await;

    match res {
        Ok(_) => {
            UserService::increase_stats_query(state.user_id())
                .analyzed_lyrics()
                .exec(app.db())
                .await?;
        },
        Err(err) => {
            app.bot()
                .edit_message_text(
                    chat_id,
                    message_id,
                    t!(
                        "analysis.failed",
                        locale = state.locale(),
                        track_name = track.track_tg_link(),
                        album_name = track.album_tg_link(),
                    ),
                )
                .link_preview_options(link_preview_small_top(track.url()))
                .reply_markup(InlineKeyboardMarkup::new(vec![vec![
                    InlineButtons::Analyze(track.id().into())
                        .into_inline_keyboard_button(state.locale()),
                ]]))
                .parse_mode(ParseMode::Html)
                .await?;

            tracing::warn!(err = ?err, "OpenAI request failed");
        },
    };
    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), track_id = track.id(), track_name = track.name_with_artists()))]
async fn perform(
    app: &App,
    state: &UserState,
    chat_id: UserId,
    message_id: teloxide::types::MessageId,
    config: &AIConfig,
    track: &ShortTrack,
    lyrics: &[&str],
) -> Result<(), anyhow::Error> {
    let song_name = track.name_with_artists();

    let model = config.model();

    let prompt = t!(
        "analysis.prompt",
        song_name = song_name,
        lyrics = lyrics.join("\n"),
        locale = state.locale()
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(prompt.as_ref())
            .build()?
            .into()])
        .build()?;

    let response = config.openai_client().chat().create(request).await?;

    let choices = response.choices.first();

    let Some(choice) = choices else { return Ok(()) };

    let analysis_result = choice.message.content.clone().unwrap_or_default();

    let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;
    let keyboard = InlineButtons::from_track_status(status, track.id(), state.locale());

    let checked = profanity::Manager::check(lyrics);

    WordStatsService::increase_analyze_occurence(app.db(), &checked.get_profine_words()).await?;

    let mut profane_words = vec![];

    for profane_word in checked.get_profine_words().iter().sorted() {
        let definition = (|| {
            WordDefinitionService::get_definition(app.db(), state.locale(), config, profane_word)
        })
        .retry(ExponentialBuilder::default())
        .await?;

        profane_words.push(format!(
            "<tg-spoiler><code>{profane_word}</code></tg-spoiler> — {definition}"
        ));
    }

    let (profane_words_block, profane_words_doesnt_fit_block) = if profane_words.is_empty() {
        ("".to_string(), "".to_string())
    } else {
        (
            t!(
                "analyze.profane-words",
                profane_words = profane_words.join("\n\n"),
                locale = state.locale()
            )
            .into(),
            t!("analyze.profane-words-doesnt-fit", locale = state.locale()).into(),
        )
    };

    let message_gen = |analysis_result: &str, profane_words_block: &str| {
        t!(
            "analysis.result",
            locale = state.locale(),
            track_name = track.track_tg_link(),
            album_name = track.album_tg_link(),
            analysis_result = analysis_result,
            profane_words_block = profane_words_block,
        )
    };

    let mut message = message_gen(&analysis_result, &profane_words_block);

    // Remove profane block if message is too long
    if message.chars_len() > MESSAGE_MAX_LEN {
        message = message_gen(&analysis_result, &profane_words_doesnt_fit_block);
    }

    // If this didn't help, crop analysis result as well
    if message.chars_len() > MESSAGE_MAX_LEN {
        let analysis_len = analysis_result.chars_len() - (message.chars_len() - MESSAGE_MAX_LEN);

        message = message_gen(
            &analysis_result.chars_crop(analysis_len),
            &profane_words_doesnt_fit_block,
        );
    }

    app.bot()
        .edit_message_text(chat_id, message_id, message)
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    Ok(())
}
