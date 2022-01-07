use super::helpers;
use crate::state::UserState;
use anyhow::{Context, Result};
use std::str::FromStr;
use strum_macros::{AsRefStr, EnumString};
use teloxide::prelude::*;
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

#[derive(Clone, EnumString, AsRefStr)]
pub enum StartKeyboard {
    #[strum(serialize = "👎 Dislike")]
    Dislike,
    #[strum(serialize = "📈 Stats")]
    Stats,
    #[strum(serialize = "🗑 Cleanup")]
    Cleanup,
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
                vec![StartKeyboard::Dislike.into()],
                vec![StartKeyboard::Stats.into(), StartKeyboard::Cleanup.into()],
            ])
            .resize_keyboard(true),
        )
    }
}

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState<'static>) -> Result<bool> {
    let text = cx.update.text().context("No text available")?;

    let button = StartKeyboard::from_str(text);

    if button.is_err() {
        return Ok(false);
    }

    let button = button?;

    match button {
        StartKeyboard::Dislike => {
            helpers::handle_dislike(cx, state).await?;
        }
        StartKeyboard::Cleanup => println!("Cleanup"),
        StartKeyboard::Stats => println!("Stats"),
    }

    Ok(true)
}
