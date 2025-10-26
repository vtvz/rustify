use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{TrackStatusService, UserService, UserStats};
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let dislikes = TrackStatusService::count_status(
        app.db(),
        TrackStatus::Disliked,
        Some(state.user_id()),
        None,
    )
    .await?;

    let ignored = TrackStatusService::count_status(
        app.db(),
        TrackStatus::Ignore,
        Some(state.user_id()),
        None,
    )
    .await?;

    let skips = TrackStatusService::sum_skips(app.db(), Some(state.user_id())).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_profane,
        lyrics_analyzed,
        ..
    } = UserService::get_stats(app.db(), Some(state.user_id())).await?;

    let message = t!(
        "actions.stats",
        locale = state.locale(),
        dislikes = dislikes,
        skips = skips,
        removed_collection = removed_collection,
        removed_playlists = removed_playlists,
        lyrics_checked = lyrics_checked,
        lyrics_analyzed = lyrics_analyzed,
        ignored = ignored,
        lyrics_profane = lyrics_profane
    );

    app.bot()
        .send_message(m.chat.id, message)
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
