use async_openai::types::chat::{
    ChatCompletionRequestUserMessage,
    CreateChatCompletionRequestArgs,
};
use backon::{ExponentialBuilder, Retryable};
use itertools::Itertools;
use rspotify::model::TrackId;
use teloxide::payloads::{AnswerCallbackQuerySetters as _, EditMessageTextSetters as _};
use teloxide::prelude::Requester as _;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{CallbackQuery, InlineKeyboardMarkup, Message, ParseMode};

use crate::app::{AIConfig, App};
use crate::profanity;
use crate::services::{
    RateLimitAction,
    RateLimitOutput,
    RateLimitService,
    TrackStatusService,
    UserService,
    WordDefinitionService,
    WordStatsService,
};
use crate::spotify::ShortTrack;
use crate::telegram::MESSAGE_MAX_LEN;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::teloxide::CallbackQueryExt as _;
use crate::utils::{DurationPrettyFormat, StringUtils};

#[tracing::instrument(skip_all, fields(user_id = state.user_id(), %track_id))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    track_id: &str,
) -> anyhow::Result<()> {
    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    let Some(config) = app.ai() else {
        app.bot()
            .answer_callback_query(q.id)
            .text(t!("analysis.disabled", locale = state.locale()))
            .await?;

        return Ok(());
    };

    let mut redis_conn = app.redis_conn().await?;

    if let RateLimitOutput::NeedToWait(duration) =
        RateLimitService::enforce_limit(&mut redis_conn, state.user_id(), RateLimitAction::Analyze)
            .await?
    {
        app.bot()
            .answer_callback_query(q.id)
            .text(t!(
                "rate-limit.analysis",
                duration = duration.pretty_format(),
                locale = state.locale()
            ))
            .await?;

        return Ok(());
    }

    let track = state
        .spotify()
        .await
        .short_track_cached(&mut redis_conn, TrackId::from_id(track_id)?)
        .await?;

    let Some(hit) = app.lyrics().search_for_track(&track).await? else {
        app.bot()
            .edit_text(
                &message,
                t!("analysis.lyrics-not-found", locale = state.locale()),
            )
            .await?;

        return Ok(());
    };

    app.bot()
        .edit_text(
            &message,
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

    let res = perform(app, state, &message, config, &track, &hit.lyrics()).await;

    match res {
        Ok(()) => {
            UserService::increase_stats_query(state.user_id())
                .analyzed_lyrics()
                .exec(app.db())
                .await?;
        },
        Err(err) => {
            app.bot()
                .edit_text(
                    &message,
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
    }
    Ok(())
}

#[tracing::instrument(skip_all, fields(
    user_id = state.user_id(),
    track_id = track.id(),
    track_name = track.name_with_artists()
))]
async fn perform(
    app: &App,
    state: &UserState,
    message: &Message,
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
        .messages([ChatCompletionRequestUserMessage::from(prompt.as_ref()).into()])
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
        (String::new(), String::new())
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

    let message_text_gen = |analysis_result: &str, profane_words_block: &str| {
        t!(
            "analysis.result",
            locale = state.locale(),
            track_name = track.track_tg_link(),
            album_name = track.album_tg_link(),
            analysis_result = analysis_result,
            profane_words_block = profane_words_block,
        )
    };

    let mut text = message_text_gen(&analysis_result, &profane_words_block);

    // Remove profane block if message is too long
    if text.chars_len() > MESSAGE_MAX_LEN {
        text = message_text_gen(&analysis_result, &profane_words_doesnt_fit_block);
    }

    // If this didn't help, crop analysis result as well
    if text.chars_len() > MESSAGE_MAX_LEN {
        let analysis_len = analysis_result.chars_len() - (text.chars_len() - MESSAGE_MAX_LEN);

        text = message_text_gen(
            &analysis_result.chars_crop(analysis_len),
            &profane_words_doesnt_fit_block,
        );
    }

    app.bot()
        .edit_text(message, text)
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    Ok(())
}
