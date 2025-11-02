use rspotify::model::TrackId;
use rspotify::prelude::BaseClient as _;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use super::super::inline_buttons::InlineButtons;
use crate::app::App;
use crate::entity::prelude::*;
use crate::services::TrackStatusService;
use crate::spotify::ShortTrack;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::teloxide::CallbackQueryExt as _;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), track_id))]
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

    let track = state
        .spotify()
        .await
        .track(TrackId::from_id(track_id)?, None)
        .await?;
    let track = ShortTrack::new(track);

    TrackStatusService::set_status(app.db(), state.user_id(), track_id, TrackStatus::Ignore)
        .await?;

    let keyboard =
        InlineButtons::from_track_status(TrackStatus::Ignore, track.id(), state.locale());

    app.bot()
        .edit_text(
            &message,
            t!(
                "actions.ignore",
                track_link = track.track_tg_link(),
                locale = state.locale()
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    Ok(())
}
