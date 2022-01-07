use std::fmt::{Display, Formatter};
use std::str::FromStr;

use anyhow::{anyhow, Result};
use rspotify::model::TrackId;
use rspotify::prelude::*;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::state::UserState;
use crate::{spotify, track_status_service, USER_ID};

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
            InlineButtons::Ignore(_) => "Ignore 🙈",
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

pub async fn handle(
    cx: UpdateWithCx<Bot, CallbackQuery>,
    state: &UserState<'static>,
) -> anyhow::Result<()> {
    let data = cx
        .update
        .data
        .ok_or_else(|| anyhow!("Callback needs data"))?;

    let button: InlineButtons = data.parse()?;

    match button {
        InlineButtons::Cancel(id) => {
            let track = state.spotify.track(&TrackId::from_id(&id).unwrap()).await?;

            track_status_service::set_status(
                &state.app.db,
                USER_ID.to_string(),
                id.clone(),
                track_status_service::Status::None,
            )
            .await?;

            cx.requester
                .edit_message_text(
                    cx.update.from.id,
                    cx.update.message.unwrap().id,
                    format!(
                        "Dislike cancelled for {}",
                        spotify::create_track_name(&track)
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
        }
        InlineButtons::Dislike(id) => {
            let track = state.spotify.track(&TrackId::from_id(&id).unwrap()).await?;

            track_status_service::set_status(
                &state.app.db,
                USER_ID.to_string(),
                id.clone(),
                track_status_service::Status::Disliked,
            )
            .await?;

            cx.requester
                .edit_message_text(
                    cx.update.from.id,
                    cx.update.message.unwrap().id,
                    format!("Disliked {}", spotify::create_track_name(&track)),
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
        }
        InlineButtons::Ignore(id) => {
            let track = state.spotify.track(&TrackId::from_id(&id).unwrap()).await?;

            track_status_service::set_status(
                &state.app.db,
                USER_ID.to_string(),
                id.clone(),
                track_status_service::Status::Ignore,
            )
            .await?;

            cx.requester
                .edit_message_text(
                    cx.update.from.id,
                    cx.update.message.unwrap().id,
                    format!(
                        "Bad words of {} will be forever ignored",
                        spotify::create_track_name(&track)
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
        }
    }

    Ok(())
}
