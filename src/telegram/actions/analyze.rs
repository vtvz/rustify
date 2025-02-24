use std::time::Duration;

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

    let prompt = formatdoc!("
        Provide a detailed description, meaning, and storyline of the following song lyrics: \"{song_name}\" and answer these questions:

	    1.	Does this song relate to any religion, and if so, which religion? Provide details.
	    2.	Does this song contain profane or explicit content or phrases? If yes, list them.
	    3.	Does this song include any sexual amorality, actions, or even hints? If yes, specify.
	    4.	Does this song reference any form of occultism or spiritism? If yes, explain.
	    5.	Are there any mentions of violence in this song? If yes, describe them.

        Reply in {lang} language and {lang} only. Respond with no formatting. There are lyrics:

        {lyrics}
    ");

    /*
    // temp
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let resp: serde_json::Value = http_client
        .get(dotenv::var("DIRECTUS_API_URL")?)
        .bearer_auth(dotenv::var("DIRECTUS_API_KEY")?)
        .send()
        .await?
        .json()
        .await?;

    let test = &resp["data"]["prompt"];

    if let serde_json::Value::String(directus_prompt) = test {
        prompt = directus_prompt
            .replace("{song_name}", &song_name)
            .replace("{lang}", &lang)
            .replace("{lyrics}", &lyrics);
    }
    // temp
    */

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into()])
        .build()?;

    let response = config.openai_client().chat().create(request).await?;

    let details = if let Some(usage) = response.usage {
        formatdoc!(
            "
                Reasoning model: <code>{model}</code>
                Tokens: prompt — <code>{}</code>, completion — <code>{}</code>
            ",
            usage.prompt_tokens,
            usage.completion_tokens,
        )
        .trim()
        .to_string()
    } else {
        format!("Reasoning model: <code>{model}</code>")
    };

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

                    {details}

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
