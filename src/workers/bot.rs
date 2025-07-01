use indoc::formatdoc;
use rustify::app::App;
use rustify::entity::prelude::UserWhitelistStatus;
use rustify::telegram::commands::UserCommand;
use rustify::telegram::utils::link_preview_disabled;
use rustify::user::UserState;
use rustify::user_service::UserService;
use sea_orm::Iterable;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode, User};
use teloxide::utils::command::BotCommands as _;

use crate::entity::prelude::UserLocale;
use crate::{self as rustify, error_handler};

async fn sync_name(
    app: &'static App,
    state: &UserState,
    tg_user: Option<&User>,
) -> anyhow::Result<()> {
    let spotify_user = state.spotify_user().await?.map(|spotify_user| {
        spotify_user
            .display_name
            .as_deref()
            .unwrap_or("unknown")
            .to_string()
    });

    let tg_user = tg_user.map(|tg_user| {
        format!(
            "{} {} {}",
            tg_user.first_name,
            tg_user.last_name.as_deref().unwrap_or_default(),
            tg_user
                .username
                .as_deref()
                .map(|username| format!("(@{})", username))
                .unwrap_or_default()
        )
        .trim()
        .to_string()
    });

    let name = vec![tg_user, spotify_user]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" | ");

    UserService::sync_name(app.db(), state.user_id(), &name).await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
async fn whitelisted(app: &'static App, state: &UserState) -> anyhow::Result<bool> {
    let res = app
        .whitelist()
        .get_status(app.db(), state.user_id())
        .await?;

    let chat_id = ChatId(state.user_id().parse()?);
    match res {
        (UserWhitelistStatus::Allowed, _) => return Ok(true),
        (UserWhitelistStatus::Denied, _) => {
            tracing::info!("Denied user tried to use bot");

            app.bot()
                .send_message(chat_id, "Sorry, your join request was rejected...")
                .parse_mode(ParseMode::Html)
                .await?;
        },
        (UserWhitelistStatus::Pending, true) => {
            tracing::info!("New user was sent a request to join");

            let message = formatdoc!(
                r#"
                    This bot is in whitelist mode\\.
                    Admin already notified that you want to join, but you also can contact <a href="tg://user?id={}">[admin]()</a> and send this message to him\\.

                    User Id: <code>{}</code>"#,
                app.whitelist().contact_admin(),
                state.user_id(),
            );

            app.bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::Html)
                .await?;

            let message = formatdoc!(
                r#"
                    New <a href="tg://user?id={user_id}">user</a> wants to join\\!

                    <code>/whitelist allow {user_id}</code>
                    <code>/whitelist deny {user_id}</code>
                "#,
                user_id = state.user_id(),
            );

            app.bot()
                .send_message(ChatId(app.whitelist().contact_admin().parse()?), message)
                .parse_mode(ParseMode::Html)
                .await?;
        },
        (UserWhitelistStatus::Pending, false) => {
            tracing::info!("Pending user tried to use bot");

            let message = formatdoc!(
                r#"
                    This bot is in whitelist mode\\.
                    Your request was already sent, but admin didn't decided yet\\.
                    You can contact <a href="tg://user?id={}">him</a> to speedup the process\\.
                    Send him this message, this will drastically help\\.

                    User Id: <code>{}</code>"#,
                app.whitelist().contact_admin(),
                state.user_id(),
            );

            app.bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::Html)
                .await?;
        },
    };

    Ok(false)
}

pub async fn work() {
    // profanity::check_cases();

    rustify::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify bot..."
    );

    let app = App::init().await.expect("State to be built");

    for locale in UserLocale::iter() {
        app.bot()
            .set_my_commands(UserCommand::localized_bot_commands(locale.as_ref()))
            .language_code(locale.as_ref())
            .await
            .expect("update commands should be working");
    }

    tokio::spawn(rustify::utils::listen_for_ctrl_c());

    let handler = dptree::entry()
        .branch(
            Update::filter_message().endpoint(move |m: Message| async move {
                let state = app.user_state(&m.chat.id.to_string()).await?;

                if !whitelisted(app, &state).await? {
                    return Ok(());
                }

                if let Err(err) = sync_name(app, &state, m.from.as_ref()).await {
                    tracing::error!(err = ?err, user_id = state.user_id(), "Failed syncing user name");
                }

                let result = rustify::telegram::handlers::message::handle(app, &state, m.clone()).await;

                if let Err(mut err) = result {
                    let res = error_handler::handle(&mut err, app, state.user_id(), state.locale()).await;
                    if !res.user_notified {
                        app.bot().send_message(
                            m.chat.id,
                            formatdoc!(
                                r#"
                                    <b>Sorry, error has happened :(</b>

                                    <a href="https://github.com/vtvz/rustify/issues/new">Report an issue on GitHub</a>
                                "#
                            )
                        )
                            .parse_mode(ParseMode::Html)
                            // TODO: wait for teloxide::sugar::request::RequestLinkPreviewExt to release
                            // .disable_link_preview()
                            .link_preview_options(link_preview_disabled())
                            .await?;
                    };
                }

                Ok(())
            }),
        )
        .branch(Update::filter_callback_query().endpoint(
            move |q: CallbackQuery| async {
                let state = app.user_state(&q.from.id.to_string()).await?;

                rustify::telegram::handlers::inline_buttons::handle(app, &state, q).await
            },
        ));

    let mut dispatcher = Dispatcher::builder(app.bot().clone(), handler).build();

    let token = dispatcher.shutdown_token();

    tokio::spawn(async move {
        rustify::utils::ctrl_c().await;

        token.shutdown().expect("To be good").await;
    });

    dispatcher.dispatch().await;
}
