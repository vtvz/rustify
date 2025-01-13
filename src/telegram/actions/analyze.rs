use anyhow::Context as _;
use async_openai::types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use indoc::formatdoc;
use rspotify::model::TrackId;
use rspotify::prelude::BaseClient as _;
use teloxide::payloads::{AnswerCallbackQuerySetters as _, EditMessageTextSetters};
use teloxide::prelude::Requester as _;
use teloxide::types::{CallbackQuery, InlineKeyboardMarkup, ParseMode, UserId};

use crate::app::{AnalyzeConfig, App};
use crate::spotify::ShortTrack;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::user_service::UserService;

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
            .text("Analysis is disabled")
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
            .edit_message_text(chat_id, message_id, "Lyrics not found")
            .await?;

        return Ok(());
    };

    app.bot()
        .edit_message_text(chat_id, message_id, "⏳ Wait for analysis to finish 🔍...")
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

    UserService::increase_stats_query(state.user_id())
        .analyzed_lyrics()
        .exec(app.db())
        .await?;

    match res {
        Ok(_) => {},
        Err(_) => {
            app.bot()
                .edit_message_text(
                    chat_id,
                    message_id,
                    "Analysis failed. This happens from time to time. Try again later 🤷",
                )
                .await?;
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

    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let lang = user
        .cfg_analysis_language
        .clone()
        .unwrap_or_else(|| config.default_language().to_string());
    let model = config.model();

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([
            ChatCompletionRequestUserMessageArgs::default()
                .content(
                    formatdoc!("
                        Give me a detailed description, meaning, and storyline of the following lyrics of the song \"{song_name}\" and answer these questions:

                        Does this song relate to any religion, and what religion is?
                        Does this have profane or explicit content or phrases and what are these?
                        Does this song have any sexual amorality, actions, or even hints?
                        Is there any occultism or spiritism in this song?
                        Is there mentions of any form of violence in this song?

                        Reply in {lang} language. Do not use any formatting

                        ---

                        {lyrics}
                    ")
                )
                .build()?
                .into(),
        ])
        .build()?;

    let response = config.openai_client().chat().create(request).await?;

    let choices = response.choices.first();

    let Some(choice) = choices else { return Ok(()) };

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            formatdoc!(
                "
                    {track_name}
                    Album: {album_name}

                    {content}
                ",
                track_name = track.track_tg_link(),
                album_name = track.album_tg_link(),
                content = choice.message.content.clone().unwrap_or_default(),
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Dislike(track.id().into()).into()]
            ],
        ))
        .await?;

    Ok(())
}
