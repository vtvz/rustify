use indoc::formatdoc;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};

use crate::app::App;
use crate::entity::prelude::*;
use crate::telegram::handlers::HandleStatus;
use crate::track_status_service::TrackStatusService;
use crate::user::UserState;
use crate::user_service::{UserService, UserStats};

pub async fn handle(
    app: &'static App,
    _state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let disliked =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, None, None).await? as u64;
    let ignored =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, None, None).await? as u64;
    let skipped = TrackStatusService::sum_skips(app.db(), None).await? as u64;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_found,
        lyrics_profane,
        lyrics_genius,
        lyrics_musixmatch,
        lyrics_lrclib,
        lyrics_azlyrics,
        lyrics_analyzed,
    } = UserService::get_stats(app.db(), None).await?;

    let lyrics_found_ratio = 100.0 * lyrics_found as f32 / lyrics_checked as f32;
    let lyrics_genius_ratio = 100.0 * lyrics_genius as f32 / lyrics_found as f32;
    let lyrics_musixmatch_ratio = 100.0 * lyrics_musixmatch as f32 / lyrics_found as f32;
    let lyrics_lrclib_ratio = 100.0 * lyrics_lrclib as f32 / lyrics_found as f32;
    let lyrics_azlyrics_ratio = 100.0 * lyrics_azlyrics as f32 / lyrics_found as f32;
    let lyrics_profane_ratio = 100.0 * lyrics_profane as f32 / lyrics_found as f32;

    let users_count = UserService::count_users(app.db(), None).await?;
    let users_active = UserService::count_users(app.db(), Some(UserStatus::Active)).await?;
    let users_active_ratio = 100.0 * users_active as f32 / users_count as f32;

    let message = formatdoc!(
        r#"
            📉 <b>Global stats</b> 📈

            👎 Disliked <code>{disliked}</code> songs
            ⏭ Skipped <code>{skipped}</code> times
            🙈 Ignored <code>{ignored}</code> tracks lyrics
            💔 Removed <code>{removed_collection}</code> from liked songs
            🗑 Removed <code>{removed_playlists}</code> from playlists
            🔬 Checked lyrics <code>{lyrics_checked}</code> times
            🔍 Analyzed lyrics <code>{lyrics_analyzed}</code> time
            🤬 <code>{lyrics_profane} ({lyrics_profane_ratio:.1}%)</code> lyrics were considered as profane

            🤷<b>Users stats</b>

            • Amount <code>{users_count}</code>
            • Active <code>{users_active} ({users_active_ratio:.2}%)</code>

            <b>Lyrics provider stats</b>

            • Found <code>{lyrics_found} ({lyrics_found_ratio:.2}%)</code>
            • Genius <code>{lyrics_genius} ({lyrics_genius_ratio:.2}%)</code>
            • MusixMatch <code>{lyrics_musixmatch} ({lyrics_musixmatch_ratio:.2}%)</code>
            • LrcLib <code>{lyrics_lrclib} ({lyrics_lrclib_ratio:.2}%)</code>
            • AZLyrics <code>{lyrics_azlyrics} ({lyrics_azlyrics_ratio:.2}%)</code>
        "#
    );

    app.bot()
        .send_message(m.chat.id, message)
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
