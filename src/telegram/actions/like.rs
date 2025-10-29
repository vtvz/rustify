use rspotify::prelude::OAuthClient as _;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::app::App;
use crate::spotify::CurrentlyPlaying;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, m.chat.id, state.locale()).await?;

        return Ok(HandleStatus::Handled);
    }

    let track = match CurrentlyPlaying::get(&*state.spotify().await).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            app.bot()
                .send_message(m.chat.id, message.localize(state.locale()))
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
        .reply_markup(StartKeyboard::markup(state.locale()))
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
