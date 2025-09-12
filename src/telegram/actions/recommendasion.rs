use std::time::Duration;

use anyhow::Context;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolArgs,
    ChatCompletionToolType,
    CreateChatCompletionRequestArgs,
    FunctionObjectArgs,
};
use backon::{ExponentialBuilder, Retryable};
use futures::StreamExt;
use indoc::formatdoc;
use itertools::Itertools;
use rspotify::model::SearchType;
use rspotify::prelude::{BaseClient, OAuthClient as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::Requester;
use teloxide::sugar::request::RequestLinkPreviewExt;
use teloxide::types::{CallbackQuery, ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::app::{AnalyzeConfig, App};
use crate::entity::prelude::TrackStatus;
use crate::recommendasion_service::RecommendasionService;
use crate::spotify::ShortTrack;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::TrackStatusService;
use crate::user::UserState;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendations {
    pub recommendations: Vec<Track>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub artist_name: String,
    pub track_title: String,
}

pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, chat_id, state.locale()).await?;

        return Ok(HandleStatus::Handled);
    }

    let Some(_) = app.analyze() else {
        app.bot()
            .send_message(
                chat_id,
                t!("recommendasion.disabled", locale = state.locale()),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    app.bot()
        .send_message(
            chat_id,
            t!("recommendasion.welcome", locale = state.locale()),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            vec![vec![
                InlineButtons::Recommendasion.into_inline_keyboard_button(state.locale()),
            ]],
        )))
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
) -> anyhow::Result<HandleStatus> {
    let chat_id = q.from.id;

    let message_id = q.message.clone().context("Message is empty")?.id();

    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, chat_id.into(), state.locale()).await?;

        return Ok(HandleStatus::Handled);
    }

    let mut redis_conn = app.redis_conn().await?;

    let Some(config) = app.analyze() else {
        app.bot()
            .edit_message_text(
                chat_id,
                message_id,
                t!("recommendasion.disabled", locale = state.locale()),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let spotify = state.spotify().await;

    let Some(_) = spotify
        .current_playback(None, None::<&[rspotify::model::AdditionalType]>)
        .await?
    else {
        app.bot()
            .edit_message_text(
                chat_id,
                message_id,
                t!("recommendasion.device-not-found", locale = state.locale()),
            )
            .reply_markup(InlineKeyboardMarkup::new(vec![vec![
                InlineButtons::Recommendasion.into_inline_keyboard_button(state.locale()),
            ]]))
            .await?;

        return Ok(HandleStatus::Handled);
    };

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!(
                "recommendasion.collecting-favorites",
                locale = state.locale()
            ),
        )
        .await?;

    let mut already_recommended =
        RecommendasionService::get_already_recommended(&mut redis_conn, state.user_id()).await?;

    let liked_tracks = get_liked_tracks(&spotify).await?;

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!(
                "recommendasion.collecting-dislikes",
                locale = state.locale()
            ),
        )
        .await?;

    let disliked_tracks = get_disliked_tracks(app, state, &spotify).await?;

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!("recommendasion.ask-ai", locale = state.locale()),
        )
        .await?;

    let mut recommendations = vec![];
    let mut recommended_disliked = vec![];
    let mut slop = vec![];

    for _ in 0..5 {
        let (recommendations_iter, disliked_iter, slop_iter) = (|| {
            get_real_recommendations(
                app,
                state,
                config,
                &liked_tracks,
                &disliked_tracks,
                &already_recommended,
            )
        })
        .retry(ExponentialBuilder::default())
        .notify(|err: &anyhow::Error, dur: Duration| {
            tracing::warn!(
                err = ?err,
                "Recommendasion is failed. Retry in {dur} sec",
                dur = dur.as_secs()
            );
        })
        .await?;

        for recommendation in recommendations_iter {
            recommendations.push(recommendation.clone());

            already_recommended.insert(0, recommendation);
        }

        recommended_disliked.extend(disliked_iter);
        slop.extend(slop_iter);

        if recommendations.len() > 10 {
            break;
        }
    }

    let slop_rate = slop.len() * 100 / (slop.len() + recommendations.len() + 1);

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!("recommendasion.queue", locale = state.locale()),
        )
        .await?;

    let mut recommendation_links = vec![];
    for recommendation in recommendations {
        spotify
            .add_item_to_queue(recommendation.raw_id().clone().into(), None)
            .await?;

        recommendation_links.push(recommendation.track_tg_link());
    }

    let mut disliked_links = vec![];
    for disliked in recommended_disliked {
        disliked_links.push(format!("👎 {}", disliked.track_tg_link()));
    }

    let search_url = url::Url::parse("https://open.spotify.com/search")?;

    let mut slop_links = vec![];
    for slop in slop {
        let mut url = search_url.clone();
        let track_name = format!("{} — {}", slop.artist_name, slop.track_title);

        url.path_segments_mut()
            .expect("Infallible")
            .push(&track_name);

        slop_links.push(format!(r#"🔍 <a href="{url}">{track_name}</a>"#,));
    }

    RecommendasionService::save_already_recommended(
        &mut redis_conn,
        state.user_id(),
        &already_recommended,
    )
    .await?;

    disliked_links.extend(slop_links);

    let msg = t!(
        "recommendasion.result",
        recommendation_links = recommendation_links.join("\n"),
        slop_rate = slop_rate,
        slop_links = disliked_links.join("\n"),
        locale = state.locale(),
    );

    app.bot()
        .edit_message_text(chat_id, message_id, msg)
        .parse_mode(ParseMode::Html)
        .disable_link_preview(true)
        .reply_markup(InlineKeyboardMarkup::new(vec![vec![
            InlineButtons::Recommendasion.into_inline_keyboard_button(state.locale()),
        ]]))
        .await?;

    Ok(HandleStatus::Handled)
}

async fn get_liked_tracks(
    spotify: &tokio::sync::RwLockReadGuard<'_, rspotify::AuthCodeSpotify>,
) -> Result<Vec<ShortTrack>, anyhow::Error> {
    let mut saved_tracks = spotify.current_user_saved_tracks(None);
    let mut liked_tracks: Vec<ShortTrack> = vec![];
    while let Some(track) = saved_tracks.next().await {
        let track = track?;
        liked_tracks.push(track.track.into());

        if liked_tracks.len() >= 100 {
            break;
        }
    }
    Ok(liked_tracks)
}

async fn get_disliked_tracks(
    app: &'static App,
    state: &UserState,
    spotify: &tokio::sync::RwLockReadGuard<'_, rspotify::AuthCodeSpotify>,
) -> Result<Vec<ShortTrack>, anyhow::Error> {
    let disliked_track_ids =
        TrackStatusService::get_ids_with_status(app.db(), state.user_id(), TrackStatus::Disliked)
            .await?;
    let disliked_short_tracks = if !disliked_track_ids.is_empty() {
        spotify
            .tracks(disliked_track_ids, None)
            .await?
            .into_iter()
            .map(ShortTrack::from)
            .collect_vec()
    } else {
        vec![]
    };
    Ok(disliked_short_tracks)
}

async fn get_real_recommendations(
    app: &App,
    state: &UserState,
    config: &AnalyzeConfig,
    liked_tracks: &[ShortTrack],
    disliked_tracks: &[ShortTrack],
    already_recommended_tracks: &[ShortTrack],
) -> anyhow::Result<(Vec<ShortTrack>, Vec<ShortTrack>, Vec<Track>)> {
    let track_recommendations = get_recommendations(
        config,
        liked_tracks,
        disliked_tracks,
        already_recommended_tracks,
    )
    .await?;

    let spotify = state.spotify().await;

    let mut recommended = vec![];
    let mut disliked = vec![];
    let mut slop = vec![];

    for track_recommendation in track_recommendations.recommendations {
        let rspotify::model::SearchResult::Tracks(res) = spotify
            .search(
                &format!(
                    "track:{track} artist:{artist}",
                    track = track_recommendation.track_title,
                    artist = track_recommendation.artist_name
                ),
                SearchType::Track,
                None,
                None,
                Some(1),
                None,
            )
            .await?
        else {
            anyhow::bail!("Searching for tracks must return tracks")
        };

        let Some(track) = res.items.first().cloned() else {
            slop.push(track_recommendation);

            continue;
        };

        let track = ShortTrack::new(track);

        let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

        if matches!(status, TrackStatus::Disliked) {
            disliked.push(track);

            continue;
        }

        if already_recommended_tracks
            .iter()
            .any(|item: &ShortTrack| item.id() == track.id())
        {
            continue;
        };

        if liked_tracks
            .iter()
            .any(|item: &ShortTrack| item.id() == track.id())
        {
            continue;
        };

        recommended.insert(0, track);
    }

    Ok((recommended, disliked, slop))
}

async fn get_recommendations(
    config: &AnalyzeConfig,
    liked_tracks: &[ShortTrack],
    disliked_tracks: &[ShortTrack],
    already_recommended_tracks: &[ShortTrack],
) -> Result<Recommendations, anyhow::Error> {
    let liked_tracks = liked_tracks
        .iter()
        .map(|track| track.name_with_artists())
        .join("\n");

    let disliked_tracks = disliked_tracks
        .iter()
        .map(|track| track.name_with_artists())
        .join("\n");

    let recommended_tracks = already_recommended_tracks
        .iter()
        .map(|track| track.name_with_artists())
        .join("\n");

    let amount = 30;
    let system_prompt = formatdoc!(
        "
            You are a music recommendation engine. Your primary goal is to suggest music tracks that the user will enjoy while STRICTLY AVOIDING any tracks they have disliked.

            CRITICAL RULES - FAILURE TO FOLLOW THESE WILL RESULT IN POOR USER EXPERIENCE:
            1. NEVER recommend tracks that appear in the user's disliked list - this is the most important rule
            2. NEVER recommend tracks that appear in the user's favorite list (they already have them)
            3. NEVER recommend tracks that have been previously suggested
            4. NEVER recommend the same artist twice in one recommendation set
            5. Only recommend real, existing songs available on streaming platforms
            6. Avoid explicit or profane lyrics

            The user has explicitly disliked certain tracks - recommending similar tracks will frustrate them. Pay extra attention to the disliked tracks list and avoid anything similar in style, artist, or genre to those tracks.
        "
    );

    let disliked_prompt = formatdoc!(
        "
            🚫 FORBIDDEN TRACKS - DO NOT RECOMMEND ANYTHING SIMILAR TO THESE:
            {disliked_tracks}

            ⚠️ CRITICAL: The above tracks are DISLIKED by the user. Do NOT recommend:
            - Any of these exact tracks
            - Tracks by the same artists
            - Tracks in similar genres/styles
            - Tracks with similar energy/mood
            - Tracks from the same era if they share similar characteristics
        "
    );

    let liked_prompt = formatdoc!(
        "
            ✅ FAVORITE TRACKS (recommend similar styles/genres to these):
            {liked_tracks}
        "
    );

    let recommended_prompt = formatdoc!(
        "
            📝 PREVIOUSLY RECOMMENDED (do not repeat):
            {recommended_tracks}
        "
    );

    let task_prompt = formatdoc!(
        "
            TASK: Generate exactly {amount} NEW music recommendations that:
            1. Are similar in style/genre to my FAVORITE tracks
            2. Are COMPLETELY DIFFERENT from my DISLIKED tracks
            3. Have NOT been previously recommended
            4. Are from different artists (no duplicates)
            5. Are real, existing songs available on streaming platforms

            Remember: Recommending anything similar to the FORBIDDEN tracks will result in a poor user experience.
        "
    );

    let req = CreateChatCompletionRequestArgs::default()
        .model(config.model())
        .temperature(2.0)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt.as_str())
                .build()?.
                into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(disliked_prompt.as_str())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(liked_prompt.as_str())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(recommended_prompt.as_str())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(task_prompt.as_str())
                .build()?
                .into(),
        ])
        .tools(vec![
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(
                    FunctionObjectArgs::default()
                        .name("recommend_tracks")
                        .description(format!("Generate {amount} music track recommendations based on user's listening history and preferences"))
                        .strict(true)
                        .parameters(json!({
                            "type": "object",
                            "properties": {
                                "recommendations": {
                                    "type": "array",
                                    "description": format!("List of {amount} recommended music tracks"),
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "artist_name": {
                                                "type": "string",
                                                "description": "Name of the artist"
                                            },
                                            "track_title": {
                                                "type": "string",
                                                "description": "Title of the track"
                                            }
                                        },
                                        "additionalProperties": false,
                                        "required": ["artist_name", "track_title"]
                                    },
                                },
                            },
                            "additionalProperties": false,
                            "required": ["recommendations"]
                        }))
                        .build()?,
                )
                .build()?,
        ])
        .tool_choice("recommend_tracks")
        .build()?;

    let response_message = config
        .openai_client()
        .chat()
        .create(req)
        .await?
        .choices
        .first()
        .context("No choices returned from OpenAI API")?
        .message
        .clone()
        .tool_calls
        .context("No tool calls found in response message")?
        .first()
        .cloned()
        .context("No tool call found in response")?;

    if response_message.function.name != "recommend_tracks" {
        anyhow::bail!("Wrong function is called");
    }

    let tracks: Recommendations = serde_json::from_str(&response_message.function.arguments)?;

    Ok(tracks)
}
