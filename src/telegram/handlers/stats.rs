use crate::entity::prelude::*;
use teloxide::prelude2::*;
use teloxide::types::ParseMode;

use crate::state::UserState;
use crate::track_status_service::TrackStatusService;
use crate::user_service::{UserService, UserStats};

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let dislikes = TrackStatusService::count_status(
        &state.app.db,
        TrackStatus::Disliked,
        Some(&state.user_id),
        None,
    )
    .await?;

    let skips = TrackStatusService::sum_skips(&state.app.db, Some(&state.user_id)).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
    } = UserService::get_stats(&state.app.db, Some(&state.user_id)).await?;

    let message =
        format!("You disliked `{dislikes}` songs so far\\. I skipped `{skips}` times, removed `{removed_collection}` from liked songs and `{removed_playlists}` from playlists");

    bot.send_message(m.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(true)
}
