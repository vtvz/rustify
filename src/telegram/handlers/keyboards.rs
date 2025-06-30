use anyhow::Context;
use teloxide::prelude::*;

use super::HandleStatus;
use crate::app::App;
use crate::telegram::actions;
use crate::telegram::keyboards::{LanguageKeyboard, StartKeyboard};
use crate::user::UserState;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, m.chat.id).await?;

        return Ok(HandleStatus::Handled);
    }

    let text = m.text().context("No text available")?;

    let button = StartKeyboard::from_str(text, state.locale());

    if let Some(button) = button {
        match button {
            StartKeyboard::Dislike => actions::dislike::handle(app, state, m).await?,
            StartKeyboard::Stats => actions::stats::handle(app, state, m).await?,
            StartKeyboard::Details => {
                actions::details::handle_current(app, state, &m.chat.id).await?
            },
        };
        return Ok(HandleStatus::Handled);
    };

    let button = LanguageKeyboard::parse(text);

    if let Some(button) = button {
        actions::language::handle(app, state, m, button.into_locale()).await?;

        return Ok(HandleStatus::Handled);
    };

    Ok(HandleStatus::Skipped)
}
