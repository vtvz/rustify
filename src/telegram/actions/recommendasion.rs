use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolArgs,
    ChatCompletionToolType,
    CreateChatCompletionRequestArgs,
    FunctionObjectArgs,
};
use futures::StreamExt;
use indoc::formatdoc;
use itertools::Itertools;
use rspotify::prelude::OAuthClient as _;
use serde_json::json;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, ParseMode};

use crate::app::{AnalyzeConfig, App};
use crate::spotify::ShortTrack;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
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

    let tracks = tracks
        .iter()
        .map(|item| item.name_with_artists())
        .join("\n");

    let config = app.analyze().unwrap();
    let tracks = get_recommendations(config, tracks).await?;

    let search_url = url::Url::parse("https://open.spotify.com/search")?;

    let tracks = tracks
        .recommendations
        .iter()
        .map(|genre| {
            let mut url = search_url.clone();
            url.path_segments_mut()
                .expect("Infallible")
                .push(&format!("{} - {}", genre.artist_name, genre.track_title));

            (genre, url)
        })
        .map(|(genre, url)| {
            format!(
                r#"<a href="{url}">{} - {}</a>"#,
                genre.artist_name, genre.track_title
            )
        })
        .join("\n");

    app.bot()
        .send_message(chat_id, tracks)
        .parse_mode(ParseMode::Html)
        .reply_markup(StartKeyboard::markup(state.locale()))
        .await?;

    Ok(HandleStatus::Handled)
}

async fn get_recommendations(
    config: &AnalyzeConfig,
    tracks: String,
) -> Result<Recommendations, anyhow::Error> {
    let user_prompt = formatdoc!(
        "
            User's favorite tracks:

            {tracks}

            Generate a list of 10 music tracks in the format 'Artist - Track Title' that are similar in style, genre, or mood,
            but are not in the provided list. Avoid duplicates and ensure diversity.
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
                .content(user_prompt)
                .build()?
                .into()
        ])
        .tools(vec![
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(
                    FunctionObjectArgs::default()
                        .name("recommend_tracks")
                        .description("Generate 10 music track recommendations based on user's favorite tracks")
                        .parameters(json!({
                            "type": "object",
                            "properties": {
                                "recommendations": {
                                    "type": "array",
                                    "description": "List of 10 recommended music tracks",
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
                                        "required": ["artist_name", "track_title"]
                                    },
                                },
                            },
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
        .unwrap()
        .message
        .clone()
        .tool_calls
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    let tracks: Recommendations = serde_json::from_str(&response_message.function.arguments)?;
    Ok(tracks)
}
