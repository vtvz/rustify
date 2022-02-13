use teloxide::prelude2::*;
use teloxide::types::ParseMode;

use crate::state::UserState;
use crate::track_status_service::{Status, TrackStatusService};

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let count =
        TrackStatusService::count_user_status(&state.app.db, &state.user_id, Status::Disliked)
            .await?;

    bot.send_message(m.chat.id, format!("You disliked `{}` songs so far", count))
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(true)
}
