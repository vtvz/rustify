use anyhow::Context;
use teloxide::prelude::*;

use super::HandleStatus;
use crate::app::App;
use crate::telegram::actions;
use crate::telegram::keyboards::{LanguageKeyboard, StartKeyboard};
use crate::user::UserState;

/// Handle an incoming Telegram message by routing it to language selection, login, or start-keyboard actions.
///
/// This function extracts the message text and:
/// - if it matches a language keyboard button, applies the selected locale and handles the language change;
/// - otherwise, if the user is not authenticated with Spotify, sends a login invite;
/// - otherwise, if it matches a start keyboard button, dispatches to the corresponding action (`Dislike`, `Stats`, or `Details`);
/// - if none of the above apply, marks the message as skipped.
///
/// Returns `Err` if the message has no text or if any underlying action fails.
///
/// # Returns
///
/// `HandleStatus::Handled` if the message was handled by one of the actions, `HandleStatus::Skipped` otherwise.
///
/// # Examples
///
/// ```no_run
/// use teloxide::prelude::*;
/// // `App`, `UserState`, `Message` and `HandleStatus` are provided by the crate.
/// # async fn example(app: &crate::App, state: &crate::UserState, msg: &Message) -> anyhow::Result<()> {
/// let status = crate::handlers::handle(app, state, msg).await?;
/// match status {
///     crate::HandleStatus::Handled => println!("message handled"),
///     crate::HandleStatus::Skipped => println!("message skipped"),
/// }
/// # Ok(())
/// # }
/// ```
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let text = m.text().context("No text available")?;

    let button = LanguageKeyboard::parse(text);

    if let Some(button) = button {
        actions::language::handle(app, state, m, button.into_locale()).await?;

        return Ok(HandleStatus::Handled);
    };

    if !state.is_spotify_authed().await {
        actions::login::send_login_invite(app, state).await?;

        return Ok(HandleStatus::Handled);
    }

    let button = StartKeyboard::from_str(text, state.locale());

    if let Some(button) = button {
        match button {
            StartKeyboard::Dislike => actions::dislike::handle(app, state, m).await?,
            StartKeyboard::Stats => actions::stats::handle(app, state, m).await?,
            StartKeyboard::Details => {
                actions::details::handle_current(app, state, m.chat.id).await?
            },
        };
        return Ok(HandleStatus::Handled);
    };

    Ok(HandleStatus::Skipped)
}