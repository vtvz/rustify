use indoc::formatdoc;
use itertools::Itertools as _;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::InlineKeyboardMarkup;

use crate::app::App;
use crate::entity::prelude::{TrackStatus, UserStatus};
use crate::services::{
    SpotifyPollingBackoffService,
    TrackLanguageStatsService,
    TrackStatusService,
    UserService,
};
use crate::telegram::inline_buttons_admin::{
    AdminInlineButtons,
    AdminUsersSortBy,
    AdminUsersSortOrder,
};
use crate::user::UserState;
use crate::utils::DurationPrettyFormat as _;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), target_user_id = %user_id))]
pub async fn handle_command(
    app: &'static App,
    state: &UserState,
    m: &Message,
    user_id: &str,
) -> anyhow::Result<()> {
    let text = format_user_details(app, user_id).await?;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        AdminInlineButtons::AdminUsersBack {
            page: 0,
            sort_by: AdminUsersSortBy::default(),
            sort_order: AdminUsersSortOrder::default(),
            status_filter: None,
        }
        .into_inline_keyboard_button(state.locale()),
    ]]);

    app.bot()
        .send_message(m.chat.id, text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), target_user_id = %user_id))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    _q: CallbackQuery,
    m: Message,
    user_id: String,
    page: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
    status_filter: Option<UserStatus>,
) -> anyhow::Result<()> {
    let text = format_user_details(app, &user_id).await?;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        AdminInlineButtons::AdminUsersBack {
            page,
            sort_by,
            sort_order,
            status_filter,
        }
        .into_inline_keyboard_button(state.locale()),
    ]]);

    app.bot().edit_text(&m, text).reply_markup(keyboard).await?;

    Ok(())
}

async fn format_user_details(app: &'static App, user_id: &str) -> anyhow::Result<String> {
    let Some(user) = UserService::get_by_id(app.db(), user_id).await? else {
        return Ok(format!("User with ID <code>{user_id}</code> is not found"));
    };

    let stats = UserService::get_stats(app.db(), Some(user_id)).await?;

    let mut redis_conn = app.redis_conn().await?;
    let idle_duration =
        SpotifyPollingBackoffService::get_idle_duration(&mut redis_conn, user_id).await?;
    let last_activity =
        SpotifyPollingBackoffService::get_last_activity(&mut redis_conn, user_id).await?;
    let last_activity = chrono::DateTime::from_timestamp(last_activity, 0).unwrap_or_default();

    let suspend_time =
        SpotifyPollingBackoffService::get_suspend_time(&mut redis_conn, user_id).await?;

    let dislikes =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, Some(user_id), None)
            .await?;

    let ignored =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, Some(user_id), None)
            .await?;

    let render_bool = |bool| if bool { "✅" } else { "❌" };

    let all_langs = TrackLanguageStatsService::sum_for_user(app.db(), user_id).await?;

    let languages = TrackLanguageStatsService::stats_for_user(app.db(), user_id, Some(20))
        .await?
        .into_iter()
        .map(|(lang, stat)| (lang.map_or("None", |lang| lang.to_name()), stat))
        .map(|(lang, stat)| {
            format!(
                "• <i>{lang}:</i> <code>{stat}</code> — <code>{:.1}%</code>",
                f64::from(stat) * 100.0 / all_langs as f64
            )
        })
        .join("\n");

    let text = formatdoc!(
        r#"
            👤 <b>User Details</b>

            <b>Basic Info:</b>
            • Name: <b>{name}</b>
            • ID: <code>{id}</code> <a href="tg://user?id={id}">link</a>
            • Status: <code>{status:?}</code>
            • Role: <code>{role:?}</code>
            • Locale: <code>{locale}</code>
            • Ref Code: {ref_code}
            • Created: <code>{created_at}</code>
            • Updated: <code>{updated_at}</code>

            <b>Configuration:</b>
            • Profanity Check: <code>{check_profanity}</code>
            • Track Skip: <code>{skip_tracks}</code>
            • AI Slop: <code>{ai_slop:?}</code>
            • Skippage Enabled: <code>{skippage_enabled}</code>
            • Skippage Duration: <code>{skippage_secs} seconds</code>
            • Magic Playlist: <code>{magic_playlist}</code>
            • Last Activity: <code>{last_activity}</code>
            • Idle Info: <code>{idle_duration}</code> / <code>{suspend_time}</code>

            <b>Statistics:</b>
            • Removed from Playlists: <code>{removed_playlists}</code>
            • Removed from Collection: <code>{removed_collection}</code>
            • Lyrics Checked: <code>{lyrics_checked}</code>
            • Lyrics Found: <code>{lyrics_found}</code>
            • Lyrics Profane: <code>{lyrics_profane}</code>
            • Lyrics Analyzed: <code>{lyrics_analyzed}</code>
            • Disliked Tracks: <code>{dislikes}</code>
            • Ignored Tracks: <code>{ignored}</code>

            <b>Lyrics Providers:</b>
            • Genius: <code>{lyrics_genius}</code>
            • MusixMatch: <code>{lyrics_musixmatch}</code>
            • LrcLib: <code>{lyrics_lrclib}</code>

            <blockquote expandable><b>Languages stats:</b>
            {languages}</blockquote>
        "#,
        name = user.name,
        id = user.id,
        status = user.status,
        role = user.role,
        locale = user.locale,
        ref_code = user.ref_code.map_or_else(
            || "<i>None</i>".into(),
            |code| format!("<code>{code}</code>")
        ),
        created_at = user.created_at.format("%Y-%m-%d %H:%M:%S"),
        updated_at = user.updated_at.format("%Y-%m-%d %H:%M:%S"),
        check_profanity = render_bool(user.cfg_check_profanity),
        skip_tracks = render_bool(user.cfg_skip_tracks),
        ai_slop = user.cfg_ai_slop_detection,
        skippage_enabled = render_bool(user.cfg_skippage_enabled),
        skippage_secs = user.cfg_skippage_secs,
        magic_playlist = user.magic_playlist.as_deref().unwrap_or("Not set"),
        last_activity = last_activity.format("%Y-%m-%d %H:%M:%S"),
        idle_duration = idle_duration.pretty_format(),
        suspend_time = suspend_time.map_or_else(|| "Inactive".into(), |time| time.pretty_format()),
        removed_playlists = stats.removed_playlists,
        removed_collection = stats.removed_collection,
        lyrics_checked = stats.lyrics_checked,
        lyrics_found = stats.lyrics_found,
        lyrics_profane = stats.lyrics_profane,
        lyrics_analyzed = stats.lyrics_analyzed,
        lyrics_genius = stats.lyrics_genius,
        lyrics_musixmatch = stats.lyrics_musixmatch,
        lyrics_lrclib = stats.lyrics_lrclib,
    );

    Ok(text)
}
