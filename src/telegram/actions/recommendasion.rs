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
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::Requester;
use teloxide::sugar::request::RequestLinkPreviewExt;
use teloxide::types::{ChatId, ParseMode};

use crate::app::{AnalyzeConfig, App};
use crate::entity::prelude::TrackStatus;
use crate::spotify::ShortTrack;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
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
        .edit_message_text(chat_id, m.id, "Asking AI about recommended track for you")
        .await?;

    let track_names = tracks
        .iter()
        .map(|item| item.name_with_artists())
        .join("\n");

    let config = app.analyze().context("Failed to get analyze config")?;

    let track_recommendations = (|| get_recommendations(config, &track_names))
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
            panic!("")
        };

        let Some(track) = res.items.first().cloned() else {
            let mut url = search_url.clone();
            url.path_segments_mut().expect("Infallible").push(&format!(
                "{} - {}",
                track_recommendation.artist_name, track_recommendation.track_title
            ));

            track_links.push(format!(
                r#"🔍 <a href="{url}">{} - {}</a>"#,
                track_recommendation.artist_name, track_recommendation.track_title
            ));

            continue;
        };

        let track = ShortTrack::new(track);

        let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

        if matches!(status, TrackStatus::Disliked) {
            track_links.push(format!("👎 {}", track.track_tg_link()));

            continue;
        }

        spotify
            .add_item_to_queue(track.raw_id().clone().into(), None)
            .await?;

        track_links.push(track.track_tg_link());

        // dbg!(res);
    }

    app.bot()
        .edit_message_text(chat_id, m.id, track_links.join("\n"))
        .parse_mode(ParseMode::Html)
        .disable_link_preview(true)
        .await?;

    Ok(HandleStatus::Handled)
}

async fn get_recommendations(
    config: &AnalyzeConfig,
    tracks: &str,
) -> Result<Recommendations, anyhow::Error> {
    let amount = 10;
    let user_prompt = formatdoc!(
        "
            Generate a list of {amount} music track recommendations that are similar in style, genre, or mood to the provided tracks, but are not in the provided list.
            For each recommendation, provide the artist name and track title separately.
            Ensure variety by not suggesting the same artist twice and exploring different subgenres or related styles within the user's musical taste.
            Do not suggest tracks with explicit or profane lyrics. Only recommend real, existing songs that can be found on music streaming platforms.

            User's favorite tracks will be listed in the next message.
        "
    );

    let req = CreateChatCompletionRequestArgs::default()
        .model(config.model())
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a music recommendation engine")
                .build()?.
                into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_prompt.as_str())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(tracks)
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
