use std::fmt::{Display, Formatter};
use std::str::FromStr;

use anyhow::Context;
use rspotify::model::TrackId;
use rspotify::prelude::*;
use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
};

use crate::entity::prelude::*;
use crate::spotify;
use crate::state::UserState;
use crate::track_status_service::TrackStatusService;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum InlineButtons {
    Cancel(String),
    Dislike(String),
    Ignore(String),
}

impl InlineButtons {
    pub fn label(&self) -> &str {
        match self {
            InlineButtons::Cancel(_) => "Cancel ↩",
            InlineButtons::Dislike(_) => "Dislike 👎",
            InlineButtons::Ignore(_) => "Ignore text 🙈",
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButtonKind> for InlineButtons {
    fn into(self) -> InlineKeyboardButtonKind {
        InlineKeyboardButtonKind::CallbackData(self.to_string())
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButton> for InlineButtons {
    fn into(self) -> InlineKeyboardButton {
        let label = self.label();
        InlineKeyboardButton::new(label, self.clone().into())
    }
}

impl FromStr for InlineButtons {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Display for InlineButtons {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| std::fmt::Error)?
                .as_ref(),
        )
    }
}

pub async fn handle(q: CallbackQuery, bot: Bot, state: &UserState) -> anyhow::Result<()> {
    if !state.is_spotify_authed().await {
        if let Some(id) = q.inline_message_id {
            bot.answer_callback_query(id)
                .text("You need to register first")
                .send()
                .await?;
        }

        return Ok(());
    }

    let data = q.data.context("Callback needs data")?;

    let button: InlineButtons = data.parse()?;

    match button {
        InlineButtons::Cancel(id) => {
            let track = state
                .spotify
                .read()
                .await
                .track(TrackId::from_id(&id)?, None)
                .await?;

            TrackStatusService::set_status(state.app.db(), &state.user_id, &id, TrackStatus::None)
                .await?;

            bot.edit_message_text(
                q.from.id,
                q.message.context("Message is empty")?.id,
                format!(
                    "Dislike cancelled for {}",
                    spotify::utils::create_track_tg_link(&track)
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(InlineKeyboardMarkup::new(
                #[rustfmt::skip]
                    vec![
                        vec![InlineButtons::Dislike(id).into()]
                    ],
            ))
            .send()
            .await?;
        },
        InlineButtons::Dislike(id) => {
            let track = state
                .spotify
                .read()
                .await
                .track(TrackId::from_id(&id)?, None)
                .await?;

            TrackStatusService::set_status(
                state.app.db(),
                &state.user_id,
                &id,
                TrackStatus::Disliked,
            )
            .await?;

            bot.edit_message_text(
                q.from.id,
                q.message.context("Message is empty")?.id,
                format!("Disliked {}", spotify::utils::create_track_tg_link(&track)),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(InlineKeyboardMarkup::new(
                #[rustfmt::skip]
                    vec![
                        vec![InlineButtons::Cancel(id).into()]
                    ],
            ))
            .send()
            .await?;
        },
        InlineButtons::Ignore(id) => {
            let track = state
                .spotify
                .read()
                .await
                .track(TrackId::from_id(&id)?, None)
                .await?;

            TrackStatusService::set_status(
                state.app.db(),
                &state.user_id,
                &id,
                TrackStatus::Ignore,
            )
            .await?;

            bot.edit_message_text(
                q.from.id,
                q.message.context("Message is empty")?.id,
                format!(
                    "Bad words of {} will be forever ignored",
                    spotify::utils::create_track_tg_link(&track)
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(InlineKeyboardMarkup::new(
                #[rustfmt::skip]
                    vec![
                        vec![InlineButtons::Cancel(id).into()]
                    ],
            ))
            .send()
            .await?;
        },
    }

    Ok(())
}
