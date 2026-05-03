use indoc::formatdoc;
use itertools::Itertools as _;
use sea_orm::Iterable as _;
use teloxide::prelude::*;
use teloxide::types::ReplyParameters;

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{TrackLanguageStatsService, TrackStatusService, UserService, UserStats};
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let disliked =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, None, None).await?;
    let ignored =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, None, None).await?;
    let skipped = TrackStatusService::sum_skips(app.db(), None).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_found,
        lyrics_profane,
        lyrics_genius,
        lyrics_musixmatch,
        lyrics_lrclib,
        lyrics_analyzed,
        ai_slop_spotify_ai_blocker,
        ai_slop_soul_over_ai,
        ai_slop_shlabs,
        ai_slop_human_made,
    } = UserService::get_stats(app.db(), None).await?;

    let user_locales = UserService::count_users_locales(app.db())
        .await?
        .iter()
        .map(|(locale, count)| {
            format!(
                "• {locale}: <code>{count}</code>",
                locale = locale.language()
            )
        })
        .collect_vec()
        .join("\n");

    let ai_slop_total =
        ai_slop_spotify_ai_blocker + ai_slop_soul_over_ai + ai_slop_shlabs + ai_slop_human_made;
    let ai_slop_spotify_ai_blocker_ratio =
        100.0 * ai_slop_spotify_ai_blocker as f32 / ai_slop_total as f32;
    let ai_slop_soul_over_ai_ratio = 100.0 * ai_slop_soul_over_ai as f32 / ai_slop_total as f32;
    let ai_slop_shlabs_ratio = 100.0 * ai_slop_shlabs as f32 / ai_slop_total as f32;
    let ai_slop_human_made_ratio = 100.0 * ai_slop_human_made as f32 / ai_slop_total as f32;

    let lyrics_found_ratio = 100.0 * lyrics_found as f32 / lyrics_checked as f32;
    let lyrics_genius_ratio = 100.0 * lyrics_genius as f32 / lyrics_found as f32;
    let lyrics_musixmatch_ratio = 100.0 * lyrics_musixmatch as f32 / lyrics_found as f32;
    let lyrics_lrclib_ratio = 100.0 * lyrics_lrclib as f32 / lyrics_found as f32;
    let lyrics_profane_ratio = 100.0 * lyrics_profane as f32 / lyrics_found as f32;

    let users_count = UserService::count_users(app.db(), None).await?;

    let mut user_stats = vec![];
    for status in UserStatus::iter() {
        let users = UserService::count_users(app.db(), Some(status)).await?;
        let ratio = 100.0 * users as f32 / users_count as f32;
        user_stats.push(format!("• {status:?} <code>{users} ({ratio:.2}%)</code>"));
    }
    let user_stats = user_stats.join("\n");

    let all_langs = TrackLanguageStatsService::sum_all_users(app.db()).await?;

    let languages = TrackLanguageStatsService::stats_all_users(app.db(), Some(20))
        .await?
        .into_iter()
        .map(|(lang, stat)| (lang.map_or("None", |lang| lang.to_name()), stat))
        .map(|(lang, stat)| {
            format!(
                "• <i>{lang}:</i> <code>{stat}</code> — <code>{:.1}%</code>",
                stat as f64 * 100.0 / all_langs as f64
            )
        })
        .join("\n");

    let text = formatdoc!(
        r"
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
            {user_stats}

            <b>Lyrics provider stats</b>

            • Found <code>{lyrics_found} ({lyrics_found_ratio:.2}%)</code>
            • Genius <code>{lyrics_genius} ({lyrics_genius_ratio:.2}%)</code>
            • MusixMatch <code>{lyrics_musixmatch} ({lyrics_musixmatch_ratio:.2}%)</code>
            • LrcLib <code>{lyrics_lrclib} ({lyrics_lrclib_ratio:.2}%)</code>

            <b>Locales stats</b>

            {user_locales}

            <b>AI Slop Detection</b>

            • Total <code>{ai_slop_total}</code>
            • Soul Over AI <code>{ai_slop_soul_over_ai} ({ai_slop_soul_over_ai_ratio:.2}%)</code>
            • Spotify AI Blocker <code>{ai_slop_spotify_ai_blocker} ({ai_slop_spotify_ai_blocker_ratio:.2}%)</code>
            • SHLabs <code>{ai_slop_shlabs} ({ai_slop_shlabs_ratio:.2}%)</code>
            • Human Made <code>{ai_slop_human_made} ({ai_slop_human_made_ratio:.2}%)</code>

            <blockquote expandable><b>Languages stats:</b>

            {languages}</blockquote>
        "
    );

    app.bot()
        .send_message(m.chat.id, text)
        .reply_parameters(ReplyParameters::new(m.id))
        .await?;

    Ok(HandleStatus::Handled)
}
