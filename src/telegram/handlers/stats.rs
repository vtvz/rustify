use indoc::formatdoc;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};

use crate::entity::prelude::*;
use crate::state::UserState;
use crate::track_status_service::TrackStatusService;
use crate::user_service::{UserService, UserStats};

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let dislikes = TrackStatusService::count_status(
        state.app.db(),
        TrackStatus::Disliked,
        Some(&state.user_id),
        None,
    )
    .await?;

    let ignored = TrackStatusService::count_status(
        state.app.db(),
        TrackStatus::Ignore,
        Some(&state.user_id),
        None,
    )
    .await?;

    let skips = TrackStatusService::sum_skips(state.app.db(), Some(&state.user_id)).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_profane,
        ..
    } = UserService::get_stats(state.app.db(), Some(&state.user_id)).await?;

    let message = formatdoc!(
        "
            📉 **Some nice stats for you** 📈

            👎 You disliked `{dislikes}` songs
            ⏭ I skipped `{skips}` times
            💔 Removed `{removed_collection}` from liked songs
            🗑 Removed `{removed_playlists}` from playlists
            🔬 Checked lyrics `{lyrics_checked}` times
            🙈 You ignored `{ignored}` tracks lyrics
            🤬 `{lyrics_profane}` lyrics were considered as profane
        "
    );

    bot.send_message(m.chat.id, message)
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(true)
}
