use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::state::UserState;
use crate::{Status, TrackStatusService};

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> anyhow::Result<bool> {
    let count =
        TrackStatusService::count_status(&state.app.db, state.user_id.clone(), Status::Disliked)
            .await?;

    cx.answer(format!("You disliked `{}` songs so far", count))
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(true)
}
