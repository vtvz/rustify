use anyhow::Context;
use async_openai::types::{
    ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolArgs,
    ChatCompletionToolType,
    CreateChatCompletionRequestArgs,
    FunctionObjectArgs,
};
use futures::StreamExt;
use indoc::formatdoc;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rspotify::model::{Id, UserId};
use rspotify::prelude::{BaseClient as _, OAuthClient as _};
use serde_json::json;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::Requester;
use teloxide::types::{CallbackQuery, ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::app::App;
use crate::spotify::{ShortPlaylist, ShortTrack};
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::keyboards::StartKeyboard;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::user_service::UserService;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tracks {
    pub tracks: Vec<Track>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub artist_name: String,
    pub track_name: String,
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

    let mut tracks = tracks
        .iter()
        .map(|item| item.name_with_artists())
        .join("\n");

    let user_prompt = formatdoc!(
        "
            You are a music recommendation system.
            You analyze what people love and suggest fresh music to listen.
            Generate 30 suggestions what to listen based on the list below. Do not repeat tracks.
            Artists should be unique and do not overlap with the user's ones
            Do not suggest same artists. Do not suggest same tracks
            There are user's favorite tracks you need:

            {tracks}"
    );

    let config = app.analyze().unwrap();

    let req = CreateChatCompletionRequestArgs::default()
        .model(config.model())
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(user_prompt)
            .build()?
            .into()])
        .tools(vec![
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(
                    FunctionObjectArgs::default()
                        .name("suggest_tracks")
                        .description("Suggest users fresh tracks to listen")
                        .parameters(json!({
                            "type": "object",
                            "properties": {
                                "tracks": {
                                    "type": "array",
                                    "description": "List of unique suggested tracks",
                                    "minItems": 10,  "maxItems": 10,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "artist_name": {"type": "string", "description": "Artist name"},
                                            "track_name": {"type": "string", "description": "Track name without artist"},
                                        },
                                        "required": ["artist_name", "track_name"],
                                    },
                                },
                            },
                            "required": ["tracks"]
                        }))
                        .build()?,
                )
                .build()?,
        ])
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

    let search_url = url::Url::parse("https://open.spotify.com/search")?;

    let tracks: Tracks = serde_json::from_str(&response_message.function.arguments)?;

    let tracks = tracks
        .tracks
        .iter()
        .map(|genre| {
            let mut url = search_url.clone();
            url.path_segments_mut()
                .expect("Infallible")
                .push(&format!("{} {}", genre.artist_name, genre.track_name));

            (genre, url)
        })
        .map(|(genre, url)| {
            format!(
                r#"<a href="{url}">{} - {}</a>"#,
                genre.artist_name, genre.track_name
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
