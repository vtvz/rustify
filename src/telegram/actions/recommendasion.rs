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

use crate::entity::prelude::TrackStatus;
use crate::app::{AIConfig, App};
use crate::services::{RecommendasionService, TrackStatusService};
use crate::spotify::ShortTrack;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::user::UserState;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecommendationsRaw {
    pub recommendations: Vec<TrackRaw>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackRaw {
    pub artist_name: String,
    pub track_title: String,
}

#[derive(Default, Debug)]
pub struct Recommendations {
    pub recommended: Vec<ShortTrack>,
    pub disliked: Vec<ShortTrack>,
    pub slop: Vec<TrackRaw>,
}

impl Recommendations {
    pub fn disliked_formatted(&self) -> Vec<String> {
        self.disliked
            .iter()
            .map(|item| format!("👎 {}", item.track_tg_link()))
            .collect()
    }

    pub fn slop_formatted(&self) -> Vec<String> {
        let search_url = url::Url::parse("https://open.spotify.com/search").expect("It's parsable");

        let mut slop_links = vec![];
        for slop in &self.slop {
            let mut url = search_url.clone();
            let track_name = format!("{} — {}", slop.artist_name, slop.track_title);

            url.path_segments_mut()
                .expect("Infallible")
                .push(&track_name);

            slop_links.push(format!(r#"🔍 <a href="{url}">{track_name}</a>"#,));
        }

        slop_links
    }
}

#[derive(Default, Debug)]
pub struct UserData {
    pub liked: Vec<ShortTrack>,
    pub recommended: Vec<ShortTrack>,
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = state.user_id(),
    )
)]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, chat_id, state.locale()).await?;

        return Ok(HandleStatus::Handled);
    }

    let Some(_) = app.ai() else {
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

#[tracing::instrument(
    skip_all,
    fields(
        user_id = state.user_id(),
    )
)]
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

    let Some(config) = app.ai() else {
        app.bot()
            .edit_message_text(
                chat_id,
                message_id,
                t!("recommendasion.disabled", locale = state.locale()),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    tracing::info!("User called Recommendasion");

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

    let mut user_data = UserData {
        recommended: RecommendasionService::get_already_recommended(
            &mut redis_conn,
            state.user_id(),
        )
        .await?,

        liked: get_liked_tracks(&spotify).await?,
    };

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!("recommendasion.ask-ai", locale = state.locale()),
        )
        .await?;

    let recommendations = get_recommendations(app, state, config, &mut user_data).await?;

    let slop_rate = recommendations.slop.len() * 100
        / (recommendations.slop.len() + recommendations.recommended.len() + 1);

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            t!("recommendasion.queue", locale = state.locale()),
        )
        .await?;

    let mut recommendation_links = vec![];
    for recommendation in &recommendations.recommended {
        spotify
            .add_item_to_queue(recommendation.raw_id().clone().into(), None)
            .await?;

        recommendation_links.push(recommendation.track_tg_link());
    }

    let mut disliked_links = recommendations.disliked_formatted();

    let slop_links = recommendations.slop_formatted();

    RecommendasionService::save_already_recommended(
        &mut redis_conn,
        state.user_id(),
        &user_data.recommended,
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

async fn get_recommendations(
    app: &'static App,
    state: &UserState,
    config: &AIConfig,
    user_data: &mut UserData,
) -> Result<Recommendations, anyhow::Error> {
    let attempts = 10;
    let min_tracks = 10;

    let mut recommendations = Recommendations::default();
    for attempt in 0..attempts {
        let recommendations_result =
            (|| get_recommendations_attempt(app, state, config, &*user_data))
                .retry(ExponentialBuilder::default())
                .notify(|err: &anyhow::Error, dur: Duration| {
                    tracing::warn!(
                        err = ?err,
                        "Recommendasion is failed. Retry in {dur} sec",
                        dur = dur.as_secs()
                    );
                })
                .await?;

        for recommendation in recommendations_result.recommended {
            recommendations.recommended.push(recommendation.clone());

            user_data.recommended.insert(0, recommendation);
        }

        recommendations
            .disliked
            .extend(recommendations_result.disliked);
        recommendations.slop.extend(recommendations_result.slop);

        if recommendations.recommended.len() > min_tracks {
            break;
        }

        tracing::debug!(
            attempt = attempt + 1,
            "AI recommended less than {attempts} tracks. Continue"
        );
    }
    Ok(recommendations)
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

async fn get_recommendations_attempt(
    app: &App,
    state: &UserState,
    config: &AIConfig,
    user_data: &UserData,
) -> anyhow::Result<Recommendations> {
    let recommendations_raw = get_raw_recommendations(config, user_data).await?;

    let spotify = state.spotify().await;

    let mut recommendations = Recommendations::default();

    for track_recommendation in recommendations_raw.recommendations {
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
            recommendations.slop.push(track_recommendation);

            continue;
        };

        let track = ShortTrack::new(track);

        let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

        if matches!(status, TrackStatus::Disliked) {
            recommendations.disliked.push(track);

            continue;
        }

        if user_data
            .recommended
            .iter()
            .any(|item: &ShortTrack| item.id() == track.id())
        {
            continue;
        };

        if user_data
            .liked
            .iter()
            .any(|item: &ShortTrack| item.id() == track.id())
        {
            continue;
        };

        recommendations.recommended.insert(0, track);
    }

    Ok(recommendations)
}

async fn get_raw_recommendations(
    config: &AIConfig,
    user_data: &UserData,
) -> Result<RecommendationsRaw, anyhow::Error> {
    let liked_tracks = user_data
        .liked
        .iter()
        .map(|track| track.name_with_artists())
        .join("\n");

    let amount = 30;
    let system_prompt = formatdoc!(
        "
            You are a music recommendation engine. Your primary goal is to suggest music tracks that the user will enjoy.

            CRITICAL RULES - FAILURE TO FOLLOW THESE WILL RESULT IN POOR USER EXPERIENCE:
            1. NEVER recommend tracks that appear in the user's favorite list (they already have them)
            2. NEVER recommend the same artist twice in one recommendation set
            3. Only recommend real, existing songs available on streaming platforms
            4. Avoid explicit or profane lyrics
        "
    );

    let liked_prompt = formatdoc!(
        "
            FAVORITE TRACKS (recommend similar styles/genres to these):
            {liked_tracks}
        "
    );

    let task_prompt = formatdoc!(
        "
            TASK: Generate exactly {amount} NEW music recommendations that:
            1. Are similar in style/genre to my FAVORITE tracks
            2. Are from different artists (no duplicates)
            3. Are real, existing songs available on streaming platforms
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
                .content(liked_prompt.as_str())
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

    let tracks: RecommendationsRaw = serde_json::from_str(&response_message.function.arguments)?;

    Ok(tracks)
}
