use std::str::FromStr;

use strum_macros::{AsRefStr, EnumString};
use teloxide::prelude::*;
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

use super::helpers::send_register_invite;
use crate::errors::{Context, GenericResult};
use crate::state::UserState;

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

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> GenericResult<bool> {
    if !state.is_spotify_authed().await {
        send_register_invite(m, bot, state).await?;

        return Ok(true);
    }

    let text = m.text().context("No text available")?;

    let button = StartKeyboard::from_str(text);

    if button.is_err() {
        return Ok(false);
    }

    let button = button?;

    match button {
        StartKeyboard::Dislike => super::handlers::dislike::handle(m, bot, state).await,
        StartKeyboard::Cleanup => super::handlers::cleanup::handle(m, bot, state).await,
        StartKeyboard::Stats => super::handlers::stats::handle(m, bot, state).await,
        StartKeyboard::Details => super::handlers::details::handle_current(m, bot, state).await,
    }
}
