use anyhow::Context as _;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use rspotify::clients::OAuthClient;
use sea_orm::TransactionTrait;
use serde::Deserialize;
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::{ChatId, ParseMode};

use crate as rustify;
use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{NotificationService, UserService};
use crate::spotify::auth::SpotifyAuthService;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::keyboards::StartKeyboard;

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

#[tracing::instrument(skip_all)]
async fn callback_handler(
    State(app): State<&'static App>,
    Query(params): Query<CallbackParams>,
) -> Response {
    match process_callback(app, params).await {
        Ok(_) => Redirect::to("https://t.me/RustifyBot").into_response(),
        Err(err) => {
            tracing::error!(error = ?err, "Failed to process Spotify callback");
            (
                StatusCode::BAD_REQUEST,
                format!(
                    "Failed to authenticate with Spotify. Please try again.\n\n{}",
                    err
                ),
            )
                .into_response()
        },
    }
}

#[tracing::instrument(skip_all)]
async fn process_callback(app: &'static App, params: CallbackParams) -> anyhow::Result<()> {
    let state_uuid: uuid::Uuid = params
        .state
        .parse()
        .context("Invalid state parameter - not a valid UUID")?;

    let user = UserService::find_by_spotify_state(app.db(), &state_uuid)
        .await?
        .context("No user found with this state - possible CSRF attack")?;

    tracing::info!(user_id = %user.id, "Processing Spotify callback for user");

    let state = app.user_state(&user.id).await?;
    let instance = state.spotify_write().await;

    instance
        .request_token(&params.code)
        .await
        .context("Failed to exchange code for token")?;

    let token = {
        instance
            .token
            .lock()
            .await
            .ok()
            .and_then(|opt| opt.clone())
            .context("Token is None after exchange")?
    };

    {
        let txn = app.db().begin().await?;
        SpotifyAuthService::set_token(&txn, &user.id, token).await?;
        UserService::set_status(&txn, &user.id, UserStatus::Active).await?;
        txn.commit().await?;
    }

    app.bot()
        .send_message(
            ChatId(state.user_id().parse()?),
            t!(
                "login.success",
                magic_command = UserCommandDisplay::Magic,
                skippage_command = UserCommandDisplay::Skippage,
                dislike_button = t!("start-keyboard.dislike", locale = state.locale()),
                details_button = t!("start-keyboard.details", locale = state.locale()),
                locale = state.locale()
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(StartKeyboard::markup(state.locale()))
        .await?;

    if let Err(err) = NotificationService::notify_spotify_connected(app, &user).await {
        tracing::error!(
            err = ?err,
            user_id = %user.id,
            "Failed to send Spotify connected notification"
        );
    }

    Ok(())
}

pub async fn work() {
    rustify::infrastructure::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify OAuth callback server..."
    );

    let app = App::init().await.expect("App to be initialized");

    let router = Router::new()
        .route("/spotify-callback", get(callback_handler))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind(app.server_http_address())
        .await
        .expect("Failed to bind to port 3000");

    tracing::info!(
        address = app.server_http_address(),
        "OAuth callback server listening"
    );

    axum::serve(listener, router)
        .await
        .expect("Server failed to start");
}
