use std::str::FromStr;

use anyhow::Context;
use strum_macros::{AsRefStr, EnumString};
use teloxide::prelude::*;
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

use super::helpers::send_register_invite;
use crate::state::{AppState, UserState};

#[derive(Clone, EnumString, AsRefStr)]
pub enum StartKeyboard {
    #[strum(serialize = "👎 Dislike playing track")]
    Dislike,
    #[strum(serialize = "📈 Stats")]
    Stats,
    #[strum(serialize = "🗑 Cleanup your playlists")]
    Cleanup,
    #[strum(serialize = "🎤 Details of track")]
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
            .resize_keyboard(),
        )
    }
}

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<bool> {
    if !state.is_spotify_authed().await {
        send_register_invite(app, m.chat.id).await?;

        return Ok(true);
    }

    let text = m.text().context("No text available")?;

    let button = StartKeyboard::from_str(text);

    if button.is_err() {
        return Ok(false);
    }

    let button = button?;

    match button {
        StartKeyboard::Dislike => super::handlers::dislike::handle(app, state, m).await,
        StartKeyboard::Cleanup => super::handlers::cleanup::handle(app, state, m).await,
        StartKeyboard::Stats => super::handlers::stats::handle(app, state, m).await,
        StartKeyboard::Details => super::handlers::details::handle_current(app, state, m).await,
    }
}
