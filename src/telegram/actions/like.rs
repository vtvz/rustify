use rspotify::prelude::OAuthClient as _;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::spotify::CurrentlyPlaying;
use crate::state::{AppState, UserState};
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::telegram::utils::link_preview_small_top;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        return Ok(HandleStatus::Skipped);
    }

    let track = match CurrentlyPlaying::get(&*state.spotify().await).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            app.bot()
                .send_message(m.chat.id, message.to_string())
                .await?;

            return Ok(HandleStatus::Handled);
        },
        CurrentlyPlaying::Ok(track, _) => track,
    };

    state
        .spotify()
        .await
        .current_user_saved_tracks_add([track.raw_id().clone()])
        .await?;

    app.bot()
        .send_message(m.chat.id, format!("Liked {}", track.track_tg_link()))
        .reply_markup(StartKeyboard::markup())
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
