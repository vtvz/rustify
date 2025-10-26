use backon::{ExponentialBuilder, Retryable};
use indoc::formatdoc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::entity::prelude::*;
use crate::app::App;
use crate::services::UserService;
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

pub async fn handle(
    app: &'static App,
    _state: &UserState,
    m: &Message,
    locale: &str,
) -> anyhow::Result<HandleStatus> {
    let locale: Result<UserLocale, _> = locale.parse();

    let Ok(locale) = locale else {
        app.bot()
            .send_message(m.chat.id, "Pass right locale")
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let Some(reply) = m.reply_to_message() else {
        app.bot()
            .send_message(m.chat.id, "Reply to message")
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let users = UserService::get_users_for_locale(app.db(), locale).await?;

    let mut errors = 0;
    let mut sent = 0;

    for user in &users {
        let send_fn = || async {
            app.bot()
                .copy_message(ChatId(user.id.parse()?), reply.chat.id, reply.id)
                .await?;

            anyhow::Ok(())
        };

        let res = send_fn.retry(ExponentialBuilder::default()).await;

        match res {
            Ok(_) => sent += 1,
            Err(err) => {
                tracing::warn!(err = ?err, "Error on message broadcasting");
                errors += 1
            },
        }
    }

    let message = formatdoc!("Sent to {sent} users. Errors {errors}");

    app.bot()
        .send_message(m.chat.id, message)
        .parse_mode(ParseMode::Html)
        .send()
        .await?;

    Ok(HandleStatus::Handled)
}
