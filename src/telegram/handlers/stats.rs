use teloxide::prelude2::*;
use teloxide::types::ParseMode;

use crate::state::UserState;
use crate::track_status_service::{Status, TrackStatusService};

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let dislikes = TrackStatusService::count_status(
        &state.app.db,
        Status::Disliked,
        Some(&state.user_id),
        None,
    )
    .await?;

    let skips = TrackStatusService::sum_user_skips(&state.app.db, Some(&state.user_id)).await?;

    let message = format!("You disliked `{dislikes}` songs so far and I skipped `{skips}` times");

    bot.send_message(m.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(true)
}
