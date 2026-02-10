use itertools::Itertools as _;
use rspotify::model::TrackId;
use teloxide::payloads::{AnswerCallbackQuerySetters as _, EditMessageTextSetters as _};
use teloxide::prelude::Requester as _;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{CallbackQuery, ParseMode};

use crate::app::App;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::teloxide::CallbackQueryExt as _;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), %track_id))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    track_id: &str,
) -> anyhow::Result<()> {
    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    let mut redis_conn = app.redis_conn().await?;

    let track = state
        .spotify()
        .await
        .short_track_cached(&mut redis_conn, TrackId::from_id(track_id)?)
        .await?;

    app.bot()
        .edit_text(
            &message,
            t!(
                "song-links.fetch",
                track_name = track.track_tg_link(),
                album_name = track.album_tg_link(),
                locale = state.locale()
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .await?;

    let res = app.song_link().get(&track).await?;

    let links = res
        .links_by_platform
        .iter()
        .sorted_by_key(|(platform, _)| platform.to_string())
        .map(|(platform, link)| format!(r#"• <a href="{link}">{platform}</a>"#, link = link.url))
        .join("\n");

    app.bot()
        .edit_text(
            &message,
            t!(
                "song-links.result",
                track_name = track.track_tg_link(),
                album_name = track.album_tg_link(),
                links = links,
                locale = state.locale()
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}
