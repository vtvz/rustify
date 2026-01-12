use itertools::Itertools as _;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{TrackLanguageStatsService, TrackStatusService, UserService, UserStats};
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = state.user_id()))]
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

    let all_langs = TrackLanguageStatsService::sum_for_user(app.db(), state.user_id()).await?;

    let languages = TrackLanguageStatsService::stats_for_user(app.db(), state.user_id(), Some(10))
        .await?
        .into_iter()
        .map(|(lang, stat)| (lang.map_or("Unknown", |lang| lang.to_name()), stat))
        .map(|(lang, stat)| {
            format!(
                "• <i>{lang}:</i> <code>{:1}%</code>",
                f64::from(stat) * 100.0 / all_langs as f64
            )
        })
        .join("\n");

    let languages = if languages.is_empty() {
        "• <i>No data yet</i>".to_string()
    } else {
        languages
    };

    let text = t!(
        "actions.stats",
        locale = state.locale(),
        dislikes = dislikes,
        skips = skips,
        removed_collection = removed_collection,
        removed_playlists = removed_playlists,
        lyrics_checked = lyrics_checked,
        lyrics_analyzed = lyrics_analyzed,
        ignored = ignored,
        lyrics_profane = lyrics_profane,
        languages = languages,
    );

    app.bot()
        .send_message(m.chat.id, text)
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
