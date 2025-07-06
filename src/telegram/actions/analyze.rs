use anyhow::Context as _;
use async_openai::types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use rspotify::model::TrackId;
use rspotify::prelude::BaseClient as _;
use teloxide::payloads::{AnswerCallbackQuerySetters as _, EditMessageTextSetters};
use teloxide::prelude::Requester as _;
use teloxide::types::{CallbackQuery, InlineKeyboardMarkup, ParseMode, UserId};

use crate::app::{AnalyzeConfig, App};
use crate::spotify::ShortTrack;
use crate::telegram::MESSAGE_MAX_LEN;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::track_status_service::TrackStatusService;
use crate::user::UserState;
use crate::user_service::UserService;
use crate::utils::StringUtils;

pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    track_id: &str,
) -> anyhow::Result<()> {
    let chat_id = q.from.id;

    let Some(config) = app.analyze() else {
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
            t!("analysis.waiting", locale = state.locale()),
        )
        .await?;

    let res = perform(
        app,
        state,
        chat_id,
        message_id,
        config,
        track,
        hit.lyrics().join("\n"),
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
                    t!("analysis.failed", locale = state.locale()),
                )
                .await?;

            tracing::warn!(err = ?err, "OpenAI request failed");
        },
    };
    Ok(())
}

async fn perform(
    app: &App,
    state: &UserState,
    chat_id: UserId,
    message_id: teloxide::types::MessageId,
    config: &AnalyzeConfig,
    track: ShortTrack,
    lyrics: String,
) -> Result<(), anyhow::Error> {
    let song_name = track.name_with_artists();

    let model = config.model();

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(
                config
                    .prompt()
                    .replace("{song_name}", &song_name)
                    .replace("{lang}", state.language())
                    .replace("{lyrics}", &lyrics),
            )
            .build()?
            .into()])
        .build()?;

    let response = config.openai_client().chat().create(request).await?;

    let choices = response.choices.first();

    let Some(choice) = choices else { return Ok(()) };

    let analysis_result = choice.message.content.clone().unwrap_or_default();

    let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;
    let keyboard = InlineButtons::from_track_status(status, track.id(), state.locale());

    let mut message = t!(
        "analysis.result",
        locale = state.locale(),
        track_name = track.track_tg_link(),
        album_name = track.album_tg_link(),
        analysis_result = analysis_result,
    );

    if message.chars_len() > MESSAGE_MAX_LEN {
        let analysis_len = analysis_result.chars_len() - (message.chars_len() - MESSAGE_MAX_LEN);
        message = t!(
            "analysis.result",
            locale = state.locale(),
            track_name = track.track_tg_link(),
            album_name = track.album_tg_link(),
            analysis_result = &analysis_result.chars_crop(analysis_len)
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
