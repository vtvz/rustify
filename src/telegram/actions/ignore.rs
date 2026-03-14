use rspotify::model::TrackId;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::InlineKeyboardMarkup;

use super::super::inline_buttons::InlineButtons;
use crate::app::App;
use crate::entity::prelude::*;
use crate::services::TrackStatusService;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), %track_id))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    _q: CallbackQuery,
    m: Message,
    track_id: &str,
) -> anyhow::Result<()> {
    let track = state
        .spotify()
        .await
        .short_track_cached(&mut app.redis_conn().await?, TrackId::from_id(track_id)?)
        .await?;

    TrackStatusService::set_status(app.db(), state.user_id(), track_id, TrackStatus::Ignore)
        .await?;

    let keyboard =
        InlineButtons::from_track_status(TrackStatus::Ignore, track.id(), state.locale());

    app.bot()
        .edit_text(
            &m,
            t!(
                "actions.ignore",
                track_link = track.track_tg_link(),
                locale = state.locale(),
                dislike_button_label = t!("inline-buttons.dislike", locale = state.locale()),
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    Ok(())
}
