use indoc::formatdoc;
use rustify::entity::prelude::UserWhitelistStatus;
use rustify::state::{AppState, UserState};
use rustify::user_service::UserService;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{ChatId, LinkPreviewOptions, ParseMode, User};

async fn sync_name(
    app_state: &'static AppState,
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

    UserService::sync_name(app_state.db(), state.user_id(), &name).await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
async fn whitelisted(app_state: &'static AppState, state: &UserState) -> anyhow::Result<bool> {
    let res = app_state
        .whitelist()
        .get_status(app_state.db(), state.user_id())
        .await?;

    let chat_id = ChatId(state.user_id().parse()?);
    match res {
        (UserWhitelistStatus::Allowed, _) => return Ok(true),
        (UserWhitelistStatus::Denied, _) => {
            tracing::info!("Denied user tried to use bot");

            app_state
                .bot()
                .send_message(chat_id, "Sorry, your join request was rejected...")
                .parse_mode(ParseMode::Html)
                .send()
                .await?;
        },
        (UserWhitelistStatus::Pending, true) => {
            tracing::info!("New user was sent a request to join");

            let message = formatdoc!(
                r#"
                    This bot is in whitelist mode\\.
                    Admin already notified that you want to join, but you also can contact <a href="tg://user?id={}">[admin]()</a> and send this message to him\\.

                    User Id: <code>{}</code>"#,
                app_state.whitelist().contact_admin(),
                state.user_id(),
            );

            app_state
                .bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::Html)
                .send()
                .await?;

            let message = formatdoc!(
                r#"
                    New <a href="tg://user?id={user_id}">user</a> wants to join\\!

                    <code>/whitelist allow {user_id}</code>
                    <code>/whitelist deny {user_id}</code>
                "#,
                user_id = state.user_id(),
            );

            app_state
                .bot()
                .send_message(
                    ChatId(app_state.whitelist().contact_admin().parse()?),
                    message,
                )
                .parse_mode(ParseMode::Html)
                .send()
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
                app_state.whitelist().contact_admin(),
                state.user_id(),
            );

            app_state
                .bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::Html)
                .send()
                .await?;
        },
    };

    Ok(false)
}

async fn run() {
    // profanity::check_cases();

    rustify::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify bot..."
    );

    let app_state = AppState::init().await.expect("State to be built");

    tokio::spawn(rustify::utils::listen_for_ctrl_c());

    let handler = dptree::entry()
        .branch(
            Update::filter_message().endpoint(move |m: Message, bot: Bot| async {
                let state = app_state.user_state(&m.chat.id.to_string()).await?;

                if !whitelisted(app_state, &state).await? {
                    return Ok(());
                }

                if let Err(err) = sync_name(app_state,&state, m.from.as_ref()).await {
                    tracing::error!(err = ?err, user_id = state.user_id(), "Failed syncing user name");
                }

                let clone = (m.clone(), bot.clone());

                let result = rustify::telegram::handle_message(app_state, &state, bot, m).await;

                let (m, bot) = clone;
                if let Err(err) = &result {
                    tracing::error!(err = ?err, "Error on message handling");
                    bot.send_message(
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
                        .link_preview_options(LinkPreviewOptions{
                            is_disabled: true,
                            url: None,
                            prefer_small_media: false,
                            prefer_large_media: false,
                            show_above_text: false,
                        })
                        .send()
                        .await?;
                }

                result
            }),
        )
        .branch(Update::filter_callback_query().endpoint(
            move |q: CallbackQuery, bot: Bot| async {
                let state = app_state.user_state(&q.from.id.to_string()).await?;

                rustify::telegram::inline_buttons::handle(app_state, &state, bot, q).await
            },
        ));

    let mut dispatcher = Dispatcher::builder(app_state.bot().clone(), handler).build();

    let token = dispatcher.shutdown_token();

    tokio::spawn(async move {
        rustify::utils::ctrl_c().await;

        token.shutdown().expect("To be good").await;
    });

    dispatcher.dispatch().await;
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    run().await;
}
