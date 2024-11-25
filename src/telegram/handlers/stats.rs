use indoc::formatdoc;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};

use crate::entity::prelude::*;
use crate::state::{AppState, UserState};
use crate::track_status_service::TrackStatusService;
use crate::user_service::{UserService, UserStats};

pub async fn handle(
    m: &Message,
    bot: &Bot,
    app_state: &'static AppState,
    state: &UserState,
) -> anyhow::Result<bool> {
    let dislikes = TrackStatusService::count_status(
        app_state.db(),
        TrackStatus::Disliked,
        Some(state.user_id()),
        None,
    )
    .await?;

    let ignored = TrackStatusService::count_status(
        app_state.db(),
        TrackStatus::Ignore,
        Some(state.user_id()),
        None,
    )
    .await?;

    let skips = TrackStatusService::sum_skips(app_state.db(), Some(state.user_id())).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_profane,
        ..
    } = UserService::get_stats(app_state.db(), Some(state.user_id())).await?;

    let message = formatdoc!(
        r#"
            📉 <b>Some nice stats for you</b> 📈

            👎 You disliked <code>{dislikes}</code> songs
            ⏭ I skipped <code>{skips}</code> times
            💔 Removed <code>{removed_collection}</code> from liked songs
            🗑 Removed <code>{removed_playlists}</code> from playlists
            🔬 Checked lyrics <code>{lyrics_checked}</code> times
            🙈 You ignored <code>{ignored}</code> tracks lyrics
            🤬 <code>{lyrics_profane}</code> lyrics were considered as profane
        "#
    );

    bot.send_message(m.chat.id, message)
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::Html)
        .send()
        .await?;

    Ok(true)
}
