use std::str::FromStr;

use anyhow::Context;
use teloxide::prelude::*;

use crate::state::{AppState, UserState};
use crate::telegram::actions;
use crate::telegram::keyboards::StartKeyboard;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<bool> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, m.chat.id).await?;

        return Ok(true);
    }

    let text = m.text().context("No text available")?;

    let button = StartKeyboard::from_str(text);

    if button.is_err() {
        return Ok(false);
    }

    let button = button?;

    match button {
        StartKeyboard::Dislike => actions::dislike::handle(app, state, m).await,
        StartKeyboard::Cleanup => actions::cleanup::handle(app, state, m).await,
        StartKeyboard::Stats => actions::stats::handle(app, state, m).await,
        StartKeyboard::Details => actions::details::handle_current(app, state, m).await,
    }
}
