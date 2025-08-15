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
use serde_json::json;
use teloxide::payloads::EditMessageTextSetters;
use teloxide::prelude::Requester;
use teloxide::sugar::request::RequestLinkPreviewExt;
use teloxide::types::{ChatId, ParseMode};

use crate::app::{AnalyzeConfig, App};
use crate::entity::prelude::TrackStatus;
use crate::recommendasion_service::RecommendasionService;
use crate::spotify::ShortTrack;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
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

    let mut redis_conn = app.redis_conn().await?;

    let mut recommended =
        RecommendasionService::get_already_recommended(&mut redis_conn, state.user_id()).await?;
    let recommended_str = recommended.join("\n");

    let Some(config) = app.analyze() else {
        app.bot()
            .send_message(chat_id, "Recommendasion is disabled")
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let m = app
        .bot()
        .send_message(chat_id, "Collecting your favorite songs")
        .await?;

    let spotify = state.spotify().await;
    let mut saved_tracks = spotify.current_user_saved_tracks(None);
    let mut tracks: Vec<ShortTrack> = vec![];

    while let Some(track) = saved_tracks.next().await {
        let track = track?;
        tracks.push(track.track.into());

        if tracks.len() >= 100 {
            break;
        }
    }

    app.bot()
        .edit_message_text(chat_id, m.id, "Collecting your disliked songs")
        .await?;

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

    let disliked_tracks = disliked_short_tracks
        .into_iter()
        .map(|track| track.name_with_artists())
        .join("\n");

    app.bot()
        .edit_message_text(chat_id, m.id, "Asking AI about recommended track for you")
        .await?;

    let track_names = tracks
        .iter()
        .map(|item| item.name_with_artists())
        .join("\n");

    let track_recommendations =
        (|| get_recommendations(config, &track_names, &disliked_tracks, &recommended_str))
            .retry(ExponentialBuilder::default())
            .notify(|err: &anyhow::Error, dur: Duration| {
                tracing::warn!(
                    err = ?err,
                    "Recommendasion is failed. Retry in {dur} sec",
                    dur = dur.as_secs()
                );
            })
            .await?;

    let search_url = url::Url::parse("https://open.spotify.com/search")?;

    app.bot()
        .edit_message_text(
            chat_id,
            m.id,
            "Finding songs in spotify and adding to queue",
        )
        .await?;

    let mut track_links = vec![];
    for track_recommendation in &track_recommendations.recommendations {
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
            let mut url = search_url.clone();
            let track_name = format!(
                "{} — {}",
                track_recommendation.artist_name, track_recommendation.track_title
            );

            url.path_segments_mut()
                .expect("Infallible")
                .push(&track_name);

            track_links.push(format!(r#"🔍 <a href="{url}">{track_name}</a>"#,));

            continue;
        };

        let track = ShortTrack::new(track);

        recommended.insert(0, track.name_with_artists());

        let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

        if matches!(status, TrackStatus::Disliked) {
            track_links.push(format!("👎 {}", track.track_tg_link()));

            continue;
        }

        spotify
            .add_item_to_queue(track.raw_id().clone().into(), None)
            .await?;

        track_links.push(track.track_tg_link());
    }

    RecommendasionService::save_already_recommended(&mut redis_conn, state.user_id(), &recommended)
        .await?;

    app.bot()
        .edit_message_text(chat_id, m.id, track_links.join("\n"))
        .parse_mode(ParseMode::Html)
        .disable_link_preview(true)
        .await?;

    Ok(HandleStatus::Handled)
}

async fn get_recommendations(
    config: &AnalyzeConfig,
    liked_tracks: &str,
    disliked_tracks: &str,
    recommended_tracks: &str,
) -> Result<Recommendations, anyhow::Error> {
    let amount = 10;
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
