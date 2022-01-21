use std::str::FromStr;

use anyhow::{Context, Result};
use strum_macros::{AsRefStr, EnumString};
use teloxide::prelude::*;
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

use crate::state::UserState;
use crate::telegram::helpers::send_register_invite;

#[derive(Clone, EnumString, AsRefStr)]
pub enum StartKeyboard {
    #[strum(serialize = "👎 Dislike")]
    Dislike,
    #[strum(serialize = "📈 Stats")]
    Stats,
    #[strum(serialize = "🗑 Cleanup")]
    Cleanup,
    #[strum(serialize = "🎤 Details")]
    Details,
}

impl From<StartKeyboard> for KeyboardButton {
    fn from(keyboard: StartKeyboard) -> Self {
        Self::new(keyboard.as_ref())
    }
}

impl StartKeyboard {
    pub fn markup() -> ReplyMarkup {
        ReplyMarkup::Keyboard(
            KeyboardMarkup::new(vec![
                vec![Self::Dislike.into()],
                vec![
                    Self::Stats.into(),
                    Self::Cleanup.into(),
                    Self::Details.into(),
                ],
            ])
            .resize_keyboard(true),
        )
    }
}

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<bool> {
    if !state.is_spotify_authed().await {
        send_register_invite(cx, state).await?;

        return Ok(true);
    }

    let text = cx.update.text().context("No text available")?;

    let button = StartKeyboard::from_str(text);

    if button.is_err() {
        return Ok(false);
    }

    let button = button?;

    match button {
        StartKeyboard::Dislike => super::handlers::dislike::handle(cx, state).await,
        StartKeyboard::Cleanup => super::handlers::cleanup::handle(cx, state).await,
        StartKeyboard::Stats => super::handlers::stats::handle(cx, state).await,
        StartKeyboard::Details => super::handlers::details::handle_current(cx, state).await,
    }
}
